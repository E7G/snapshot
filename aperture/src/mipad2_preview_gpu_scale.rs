// SPDX-License-Identifier: GPL-3.0-or-later
//! Explicit preview-only GPU scaling candidate; recording caps are never changed.
use gst::prelude::*;

const FRAGMENT: &str = include_str!("mipad2_preview_fit.frag");

pub(crate) fn requested(device: bool, android: bool, flag: Option<&str>, factory: bool) -> bool {
    device && android && flag == Some("1") && factory
}

fn display_aspect(width: u32, height: u32, par_n: i32, par_d: i32) -> Option<f32> {
    if width == 0 || height == 0 || par_n <= 0 || par_d <= 0 {
        return None;
    }
    let dar = (f64::from(width) * f64::from(par_n) / (f64::from(height) * f64::from(par_d))) as f32;
    (dar.is_finite() && dar > 0.0).then_some(dar)
}

fn update_aspect(shader: &gst::Element, caps: &gst::CapsRef) -> Result<(), gst::glib::BoolError> {
    let info = gst_video::VideoInfo::from_caps(caps)?;
    if info.format() != gst_video::VideoFormat::Rgba {
        return Err(gst::glib::bool_error!(
            "GPU preview requires converted RGBA"
        ));
    }
    let dar = display_aspect(
        info.width(),
        info.height(),
        info.par().numer(),
        info.par().denom(),
    )
    .ok_or_else(|| gst::glib::bool_error!("Invalid GPU preview display aspect ratio"))?;
    shader.set_property(
        "uniforms",
        gst::Structure::builder("uniforms")
            .field("source_dar", dar)
            .build(),
    );
    Ok(())
}

fn aspect_probe(pad: &gst::Pad, shader: &gst::Element) {
    let weak = shader.downgrade();
    pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, probe| {
        if let Some(gst::PadProbeData::Event(event)) = probe.data.as_ref() {
            if let gst::EventView::Caps(caps) = event.view() {
                if let Some(shader) = weak.upgrade() {
                    if let Err(error) = update_aspect(&shader, caps.caps()) {
                        shader.post_error_message(gst::error_msg!(
                            gst::CoreError::Negotiation,
                            ["Invalid preview GPU scaler caps: {error}"]
                        ));
                        return gst::PadProbeReturn::Drop;
                    }
                }
            }
        }
        gst::PadProbeReturn::Ok
    });
}

pub(crate) fn create() -> Result<gst::Element, gst::glib::BoolError> {
    let shader = gst::ElementFactory::make("glshader")
        .name("mipad2-preview-gpu-fit")
        .property("fragment", FRAGMENT)
        .build()?;
    shader.set_property(
        "uniforms",
        gst::Structure::builder("uniforms")
            .field("source_dar", 16.0_f32 / 9.0)
            .build(),
    );
    let pad = shader.static_pad("sink").unwrap();
    // CAPS are serialized before their buffers. Read source DAR here: glshader's
    // built-in width/height uniforms describe its OUTPUT, not the source.
    aspect_probe(&pad, &shader);
    let output = gst::ElementFactory::make("capsfilter")
        .property(
            "caps",
            gst::Caps::builder("video/x-raw")
                .features(["memory:GLMemory"])
                .field("format", "RGBA")
                .field("width", 1280_i32)
                .field("height", 720_i32)
                .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                .field("texture-target", "2D")
                .build(),
        )
        .build()?;
    let bin = gst::Bin::new();
    bin.add_many([&shader, &output])?;
    shader.link(&output)?;
    bin.add_pad(&gst::GhostPad::with_target(&pad)?)?;
    bin.add_pad(&gst::GhostPad::with_target(
        &output.static_pad("src").unwrap(),
    )?)?;
    Ok(bin.upcast())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_bin_links_rotation_and_rejects_missing_src_mutation() {
        gst::init().unwrap();
        let upload = gst::ElementFactory::make("glupload").build().unwrap();
        let convert = gst::ElementFactory::make("glcolorconvert").build().unwrap();
        let fit = create().unwrap();
        let rotate = gst::ElementFactory::make("glvideoflip").build().unwrap();
        let display = gst::ElementFactory::make("fakesink").build().unwrap();
        let pipeline = gst::Pipeline::new();
        pipeline
            .add_many([&upload, &convert, &fit, &rotate, &display])
            .unwrap();
        let input = fit.static_pad("sink").unwrap();
        let output = fit.static_pad("src").unwrap();
        assert_eq!(input.direction(), gst::PadDirection::Sink);
        assert_eq!(output.direction(), gst::PadDirection::Src);
        assert!(input.is::<gst::GhostPad>() && output.is::<gst::GhostPad>());
        gst::Element::link_many([&upload, &convert, &fit, &rotate, &display]).unwrap();
        assert!(output.is_linked());
        // Reproduce the exact original construction defect without drawing:
        // removing this pad must make the downstream link impossible again.
        fit.remove_pad(&output).unwrap();
        assert!(fit.link(&rotate).is_err());
    }

    #[test]
    fn opt_in_is_exact_and_scoped() {
        for device in [false, true] {
            for android in [false, true] {
                for factory in [false, true] {
                    for flag in [None, Some("0"), Some("true"), Some("1")] {
                        assert_eq!(
                            requested(device, android, flag, factory),
                            device && android && factory && flag == Some("1")
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn dar_preserves_photo_portrait_and_non_square_pixels() {
        for (w, h, n, d, want) in [
            (1920, 1080, 1, 1, 16.0 / 9.0),
            (3264, 2448, 1, 1, 4.0 / 3.0),
            (2560, 1920, 1, 1, 4.0 / 3.0),
            (720, 1280, 1, 1, 9.0 / 16.0),
            (1280, 720, 2, 1, 32.0 / 9.0),
        ] {
            assert!((display_aspect(w, h, n, d).unwrap() - want).abs() < 1e-5);
        }
        for (w, h, n, d) in [
            (0, 720, 1, 1),
            (1280, 0, 1, 1),
            (1280, 720, 0, 1),
            (1280, 720, 1, 0),
            (1280, 720, -1, 1),
            (1280, 720, 1, -1),
        ] {
            assert!(display_aspect(w, h, n, d).is_none());
        }
    }

    #[test]
    fn serialized_caps_update_the_source_dar_not_output_dar() {
        gst::init().unwrap();
        let shader = gst::ElementFactory::make("glshader").build().unwrap();
        let template_caps = gst::Caps::builder("video/x-raw")
            .features(["memory:GLMemory"])
            .field("format", "RGBA")
            .build();
        let template = gst::PadTemplate::new(
            "sink",
            gst::PadDirection::Sink,
            gst::PadPresence::Always,
            &template_caps,
        )
        .unwrap();
        let pad = gst::Pad::builder_from_template(&template)
            .event_function(|_, _, _| true)
            .build();
        aspect_probe(&pad, &shader);
        pad.set_active(true).unwrap();
        for (w, h, n, d) in [
            (1920, 1080, 1, 1),
            (3264, 2448, 1, 1),
            (1280, 720, 2, 1),
            (720, 1280, 1, 1),
        ] {
            let caps = gst::Caps::builder("video/x-raw")
                .features(["memory:GLMemory"])
                .field("format", "RGBA")
                .field("width", w)
                .field("height", h)
                .field("pixel-aspect-ratio", gst::Fraction::new(n, d))
                .build();
            assert!(gst_video::VideoInfo::from_caps(&caps).is_ok());
            assert!(
                pad.query_accept_caps(&caps),
                "Fixture must accept its serialized CAPS"
            );
            assert!(pad.send_event(gst::event::Caps::new(&caps)));
            let uniforms = shader.property::<gst::Structure>("uniforms");
            assert!(
                (uniforms.get::<f32>("source_dar").unwrap()
                    - display_aspect(w as u32, h as u32, n, d).unwrap())
                .abs()
                    < 1e-5
            );
        }
        pad.set_active(false).unwrap();
    }
}
