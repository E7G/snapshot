// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in preview-only NV12 scaling before GL import. Requires the patched VA plugin.
use gst::prelude::*;

const WIDTH: u64 = 1280;
const HEIGHT: u64 = 720;
const DRM_FORMAT: &str = "NV12:0x0100000000000002";

pub(crate) fn requested(
    device: bool,
    android: bool,
    gpu_fit: bool,
    flag: Option<&str>,
    factory: bool,
) -> bool {
    device && android && gpu_fit && flag == Some("1") && factory
}

#[derive(Debug, PartialEq, Eq)]
struct Layout {
    width: i32,
    height: i32,
    par_n: i32,
    par_d: i32,
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        (a, b) = (b, a % b);
    }
    a
}

fn align_fraction(n: u64, d: u64) -> Option<u64> {
    let step = d.checked_mul(8)?;
    n.checked_add(step.checked_sub(1)?)?
        .checked_div(step)?
        .checked_mul(8)
}

fn layout(width: u32, height: u32, par_n: i32, par_d: i32) -> Option<Layout> {
    // Bound the native camera input, not its capture format or frame rate.
    if width == 0 || height == 0 || width > 4096 || height > 4096 || par_n <= 0 || par_d <= 0 {
        return None;
    }
    let n = u64::from(width).checked_mul(par_n as u64)?;
    let d = u64::from(height).checked_mul(par_d as u64)?;
    let (w, h) = if n.checked_mul(HEIGHT)? >= d.checked_mul(WIDTH)? {
        (WIDTH, align_fraction(WIDTH.checked_mul(d)?, n)?)
    } else {
        (align_fraction(HEIGHT.checked_mul(n)?, d)?, HEIGHT)
    };
    if w == 0 || h == 0 || w > WIDTH || h > HEIGHT {
        return None;
    }
    let pn = n.checked_mul(h)?;
    let pd = d.checked_mul(w)?;
    let g = gcd(pn, pd);
    Some(Layout {
        width: i32::try_from(w).ok()?,
        height: i32::try_from(h).ok()?,
        par_n: i32::try_from(pn / g).ok()?,
        par_d: i32::try_from(pd / g).ok()?,
    })
}

impl Layout {
    fn caps(&self) -> gst::Caps {
        gst::Caps::builder("video/x-raw")
            .features(["memory:DMABuf"])
            .field("format", "DMA_DRM")
            .field("drm-format", DRM_FORMAT)
            .field("width", self.width)
            .field("height", self.height)
            .field(
                "pixel-aspect-ratio",
                gst::Fraction::new(self.par_n, self.par_d),
            )
            .build()
    }
}

fn update_layout(size: &gst::Element, caps: &gst::CapsRef) -> Result<(), gst::glib::BoolError> {
    let info = gst_video::VideoInfo::from_caps(caps)?;
    if info.format() != gst_video::VideoFormat::Nv12 {
        return Err(gst::glib::bool_error!("VA preview requires native NV12"));
    }
    let layout = layout(
        info.width(),
        info.height(),
        info.par().numer(),
        info.par().denom(),
    )
    .ok_or_else(|| gst::glib::bool_error!("Invalid VA preview layout"))?;
    let next = layout.caps();
    if size.property::<gst::Caps>("caps") != next {
        size.set_property("caps", next);
    }
    Ok(())
}

fn layout_probe(pad: &gst::Pad, size: &gst::Element) {
    let weak = size.downgrade();
    pad.add_probe(gst::PadProbeType::EVENT_DOWNSTREAM, move |_, probe| {
        if let Some(gst::PadProbeData::Event(event)) = probe.data.as_ref() {
            if let gst::EventView::Caps(caps) = event.view() {
                if let Some(size) = weak.upgrade() {
                    // CAPS are serialized before their buffers. Explicit PAR
                    // preserves DAR despite the driver's eight-pixel alignment.
                    if let Err(error) = update_layout(&size, caps.caps()) {
                        size.post_error_message(gst::error_msg!(
                            gst::CoreError::Negotiation,
                            ["Invalid preview VA scaler caps: {error}"]
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
    let vpp = gst::ElementFactory::make("vapostproc")
        .name("mipad2-preview-va-scale")
        .property("add-borders", false)
        .build()?;
    let size = gst::ElementFactory::make("capsfilter")
        .name("mipad2-preview-va-layout")
        .property("caps", layout(1920, 1080, 1, 1).unwrap().caps())
        .build()?;
    let input = vpp.static_pad("sink").unwrap();
    layout_probe(&input, &size);
    let bin = gst::Bin::new();
    bin.add_many([&vpp, &size])?;
    vpp.link(&size)?;
    bin.add_pad(&gst::GhostPad::with_target(&input)?)?;
    bin.add_pad(&gst::GhostPad::with_target(
        &size.static_pad("src").unwrap(),
    )?)?;
    Ok(bin.upcast())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    fn native_caps(w: i32, h: i32, n: i32, d: i32) -> gst::Caps {
        gst::Caps::builder("video/x-raw")
            .field("format", "NV12")
            .field("width", w)
            .field("height", h)
            .field("pixel-aspect-ratio", gst::Fraction::new(n, d))
            .field("framerate", gst::Fraction::new(1, 1))
            .field("colorimetry", "bt709")
            .field("chroma-site", "mpeg2")
            .build()
    }

    #[test]
    fn exact_opt_in_requires_all_scopes_and_existing_gpu_fit() {
        for device in [false, true] {
            for android in [false, true] {
                for fit in [false, true] {
                    for factory in [false, true] {
                        for flag in [None, Some("0"), Some("true"), Some("1")] {
                            assert_eq!(
                                requested(device, android, fit, flag, factory),
                                device && android && fit && factory && flag == Some("1")
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn checked_layout_preserves_exact_native_dar_with_eight_pixel_alignment() {
        for (w, h, n, d, want) in [
            (1920, 1080, 1, 1, (1280, 720, 1, 1)),
            (2560, 1920, 1, 1, (960, 720, 1, 1)),
            (3264, 2448, 1, 1, (960, 720, 1, 1)),
            (720, 1280, 1, 1, (408, 720, 135, 136)),
            (1280, 720, 2, 1, (1280, 360, 1, 1)),
            (1280, 720, 1, 2, (640, 720, 1, 1)),
        ] {
            let a = layout(w, h, n, d).unwrap();
            assert_eq!((a.width, a.height, a.par_n, a.par_d), want);
            assert_eq!((a.width % 8, a.height % 8), (0, 0));
            assert_eq!(
                u128::from(w) * n as u128 * a.height as u128 * a.par_d as u128,
                u128::from(h) * d as u128 * a.width as u128 * a.par_n as u128
            );
        }
        for (w, h, n, d) in [
            (0, 720, 1, 1),
            (1280, 0, 1, 1),
            (4097, 720, 1, 1),
            (1280, 4097, 1, 1),
            (1280, 720, 0, 1),
            (1280, 720, 1, 0),
            (1280, 720, -1, 1),
            (1280, 720, 1, -1),
            (4096, 1, i32::MAX, 1),
        ] {
            assert!(layout(w, h, n, d).is_none());
        }
    }

    #[test]
    fn serialized_native_caps_update_layout_and_do_not_retain_the_filter() {
        gst::init().unwrap();
        let size = gst::ElementFactory::make("capsfilter")
            .property("caps", layout(1920, 1080, 1, 1).unwrap().caps())
            .build()
            .unwrap();
        let template = gst::PadTemplate::new(
            "sink",
            gst::PadDirection::Sink,
            gst::PadPresence::Always,
            &gst::Caps::from_str("video/x-raw,format=NV12").unwrap(),
        )
        .unwrap();
        let pad = gst::Pad::builder_from_template(&template)
            .event_function(|_, _, _| true)
            .build();
        layout_probe(&pad, &size);
        pad.set_active(true).unwrap();
        for (w, h, n, d) in [
            (1920, 1080, 1, 1),
            (3264, 2448, 1, 1),
            (720, 1280, 1, 1),
            (1280, 720, 2, 1),
        ] {
            assert!(pad.send_event(gst::event::Caps::new(&native_caps(w, h, n, d))));
            assert_eq!(
                size.property::<gst::Caps>("caps"),
                layout(w as u32, h as u32, n, d).unwrap().caps()
            );
        }
        let weak = size.downgrade();
        drop(size);
        assert!(weak.upgrade().is_none());
        pad.set_active(false).unwrap();
    }

    #[test]
    fn rejects_non_nv12_and_reports_negotiation_error_on_the_bus() {
        gst::init().unwrap();
        let pipeline = gst::Pipeline::new();
        let size = gst::ElementFactory::make("capsfilter")
            .property("caps", layout(1920, 1080, 1, 1).unwrap().caps())
            .build()
            .unwrap();
        pipeline.add(&size).unwrap();
        let pad = gst::Pad::builder(gst::PadDirection::Sink)
            .event_function(|_, _, _| true)
            .build();
        layout_probe(&pad, &size);
        pad.set_active(true).unwrap();
        let bad = gst_video::VideoInfo::builder(gst_video::VideoFormat::Rgba, 1280, 720)
            .build()
            .unwrap()
            .to_caps()
            .unwrap();
        assert!(update_layout(&size, &bad).is_err());
        pad.send_event(gst::event::Caps::new(&bad));
        let bus = pipeline.bus().unwrap();
        let msg = bus
            .pop_filtered(&[gst::MessageType::Error])
            .expect("Must not silently accept invalid caps");
        assert!(matches!(msg.view(), gst::MessageView::Error(_)));
        pad.set_active(false).unwrap();
    }

    #[test]
    fn production_bins_preserve_full_fov_across_nine_native_caps_changes() {
        gst::init().unwrap();
        let plugin_path = std::env::var_os("MIPAD2_TEST_VA_PLUGIN")
            .expect("Set MIPAD2_TEST_VA_PLUGIN to the hash-verified private patched plugin");
        let plugin = gst::Plugin::load_file(std::path::Path::new(&plugin_path)).unwrap();
        assert_eq!(
            gst::ElementFactory::find("vapostproc")
                .unwrap()
                .plugin()
                .unwrap(),
            plugin
        );
        let source = gst::ElementFactory::make("appsrc")
            .property("format", gst::Format::Time)
            .build()
            .unwrap();
        let va = create().unwrap();
        let upload = gst::ElementFactory::make("glupload").build().unwrap();
        let convert = gst::ElementFactory::make("glcolorconvert").build().unwrap();
        let fit = crate::mipad2_preview_gpu_scale::create().unwrap();
        let download = gst::ElementFactory::make("gldownload").build().unwrap();
        let outcaps = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::from_str("video/x-raw,format=RGBA").unwrap(),
            )
            .build()
            .unwrap();
        let sink = gst::ElementFactory::make("appsink")
            .property("sync", false)
            .property("max-buffers", 1_u32)
            .build()
            .unwrap();
        let pipeline = gst::Pipeline::new();
        let elements = [
            &source, &va, &upload, &convert, &fit, &download, &outcaps, &sink,
        ];
        pipeline.add_many(elements).unwrap();
        gst::Element::link_many(elements).unwrap();
        let dma = Arc::new(AtomicUsize::new(0));
        let count = dma.clone();
        va.static_pad("src")
            .unwrap()
            .add_probe(gst::PadProbeType::BUFFER, move |_, info| {
                if let Some(gst::PadProbeData::Buffer(b)) = info.data.as_ref() {
                    if b.n_memory() > 0 && b.iter_memories().all(|m| m.is_type("dmabuf")) {
                        count.fetch_add(1, Ordering::Relaxed);
                    }
                }
                gst::PadProbeReturn::Ok
            });
        source.set_property("caps", native_caps(1920, 1080, 1, 1));
        // Always restore NULL even when a pixel/renegotiation assertion fails.
        struct Reset(gst::Pipeline);
        impl Drop for Reset {
            fn drop(&mut self) {
                let _ = self.0.set_state(gst::State::Null);
                let _ = self.0.state(gst::ClockTime::from_seconds(5));
            }
        }
        let _reset = Reset(pipeline.clone());
        pipeline.set_state(gst::State::Playing).unwrap();
        let cases = [
            (1920, 1080, 1, 1),
            (2560, 1920, 1, 1),
            (3264, 2448, 1, 1),
            (720, 1280, 1, 1),
            (1280, 720, 2, 1),
            (1920, 1080, 1, 1),
            (2560, 1920, 1, 1),
            (720, 1280, 1, 1),
            (1920, 1080, 1, 1),
        ];
        for (index, (w, h, pn, pd)) in cases.into_iter().enumerate() {
            source.set_property("caps", native_caps(w, h, pn, pd));
            let (w, h) = (w as usize, h as usize);
            let mut b = gst::Buffer::with_size(w * h * 3 / 2).unwrap();
            {
                let b = b.get_mut().unwrap();
                b.set_pts(gst::ClockTime::from_seconds(index as u64));
                b.set_duration(gst::ClockTime::SECOND);
                gst_video::VideoMeta::add(
                    b,
                    gst_video::VideoFrameFlags::empty(),
                    gst_video::VideoFormat::Nv12,
                    w as u32,
                    h as u32,
                )
                .unwrap();
                let mut map = b.map_writable().unwrap();
                let data = map.as_mut_slice();
                data[w * h..].fill(128);
                for y in 0..h {
                    for x in 0..w {
                        data[y * w + x] = if x >= 9 * w / 10 {
                            235
                        } else if y >= 9 * h / 10 {
                            200
                        } else if y < h / 2 {
                            if x < w / 2 { 32 } else { 90 }
                        } else if x < w / 2 {
                            160
                        } else {
                            220
                        };
                    }
                }
            }
            assert_eq!(
                source.emit_by_name::<gst::FlowReturn>("push-buffer", &[&b]),
                gst::FlowReturn::Ok
            );
            let sample = sink
                .emit_by_name::<Option<gst::Sample>>(
                    "try-pull-sample",
                    &[&(12 * gst::ClockTime::SECOND).nseconds()],
                )
                .unwrap_or_else(|| {
                    panic!(
                        "No output at native case {index}; error {:?}",
                        pipeline
                            .bus()
                            .unwrap()
                            .pop_filtered(&[gst::MessageType::Error])
                    )
                });
            let b = sample.buffer().unwrap();
            assert_eq!(b.pts(), Some(gst::ClockTime::from_seconds(index as u64)));
            let vi = gst_video::VideoInfo::from_caps(sample.caps().unwrap()).unwrap();
            assert_eq!(
                (vi.width(), vi.height(), vi.format()),
                (1280, 720, gst_video::VideoFormat::Rgba)
            );
            let meta = b.meta::<gst_video::VideoMeta>();
            let stride = meta.as_ref().map_or(vi.stride()[0], |m| m.stride()[0]) as usize;
            let offset = meta.as_ref().map_or(vi.offset()[0], |m| m.offset()[0]);
            let map = b.map_readable().unwrap();
            let pixel = |x: usize, y: usize| -> &[u8] {
                &map.as_slice()[offset + y * stride + x * 4..offset + y * stride + x * 4 + 4]
            };
            let dar = w as f64 * pn as f64 / (h as f64 * pd as f64);
            let fx = (dar / (1280.0 / 720.0)).min(1.0);
            let fy = ((1280.0 / 720.0) / dar).min(1.0);
            for (ux, uy, value) in [
                (0.25, 0.25, 19_i16),
                (0.75, 0.25, 86),
                (0.25, 0.75, 168),
                (0.75, 0.75, 238),
                (0.95, 0.5, 255),
                (0.5, 0.95, 214),
            ] {
                let p = pixel(
                    (1280.0 * (0.5 + (ux - 0.5) * fx)) as usize,
                    (720.0 * (0.5 + (uy - 0.5) * fy)) as usize,
                );
                assert!(
                    p[..3].iter().all(|c| (i16::from(*c) - value).abs() <= 3) && p[3] == 255,
                    "case {index} marker {ux},{uy}: {p:?}"
                );
            }
            if fx < 0.99 {
                assert_eq!(pixel(0, 360), [0, 0, 0, 255]);
                assert_eq!(pixel(1279, 360), [0, 0, 0, 255]);
            }
            if fy < 0.99 {
                assert_eq!(pixel(640, 0), [0, 0, 0, 255]);
                assert_eq!(pixel(640, 719), [0, 0, 0, 255]);
            }
            let active_w = (0..1280)
                .filter(|x| pixel(*x, 360)[..3].iter().any(|c| *c > 8))
                .count();
            let active_h = (0..720)
                .filter(|y| pixel(640, *y)[..3].iter().any(|c| *c > 8))
                .count();
            assert!(
                (active_w as f64 - 1280.0 * fx).abs() <= 2.1
                    && (active_h as f64 - 720.0 * fy).abs() <= 2.1
            );
        }
        assert_eq!(
            source.emit_by_name::<gst::FlowReturn>("end-of-stream", &[]),
            gst::FlowReturn::Ok
        );
        assert_eq!(dma.load(Ordering::Relaxed), 9);
    }
}
