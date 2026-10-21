// SPDX-License-Identifier: GPL-3.0-or-later
//! Experimental video encodebin INPUT queue bound, not a leaky producer queue.
//! Resolve the video ghost sink's actual target. Never select by auto queue name,
//! change audio/mux/preview queues, flush data or edit buffers/timestamps/events.
use gst::prelude::*;

pub(crate) fn enabled(
    device: bool,
    requested: bool,
    direct: bool,
    current_caps: bool,
    asynchronous: bool,
) -> bool {
    device && requested && direct && current_caps && !asynchronous
}

fn video_owner(element: &gst::Element) -> bool {
    element.name() == "video-encodebin"
        && element.factory().is_some_and(|f| f.name() == "encodebin")
}

fn video_pad(pad: &gst::Pad) -> bool {
    pad.direction() == gst::PadDirection::Sink
        && pad
            .name()
            .strip_prefix("video_")
            .is_some_and(|s| !s.is_empty() && s.bytes().all(|c| c.is_ascii_digit()))
}

fn input_queue(owner: &gst::Element, pad: &gst::Pad) -> Result<gst::Element, gst::glib::BoolError> {
    let fail = || {
        gst::glib::bool_error!(
            "Video encode input queue has an unexpected ghost/identity/queue topology"
        )
    };
    if !video_owner(owner) || !video_pad(pad) || pad.parent_element().as_ref() != Some(owner) {
        return Err(fail());
    }
    let ghost = pad.downcast_ref::<gst::GhostPad>().ok_or_else(fail)?;
    let mut sink = ghost.target().ok_or_else(fail)?;
    let mut element = sink.parent_element().ok_or_else(fail)?;
    if element.factory().is_some_and(|f| f.name() == "identity") {
        if element.parent().as_ref() != Some(owner.upcast_ref())
            || !element.property::<bool>("single-segment")
            || sink.name() != "sink"
        {
            return Err(fail());
        }
        sink = element
            .static_pad("src")
            .and_then(|p| p.peer())
            .ok_or_else(fail)?;
        element = sink.parent_element().ok_or_else(fail)?;
    }
    if sink.direction() != gst::PadDirection::Sink
        || sink.name() != "sink"
        || !element.factory().is_some_and(|f| f.name() == "queue")
        || element.parent().as_ref() != Some(owner.upcast_ref())
    {
        return Err(fail());
    }
    Ok(element)
}

fn configure(owner: &gst::Element, pad: &gst::Pad) -> Result<bool, gst::glib::BoolError> {
    if !video_owner(owner) || !video_pad(pad) {
        return Ok(false);
    }
    let queue = input_queue(owner, pad)?;
    // Never change a live queue's bounds or force a flush to make it fit.
    if queue.current_state() > gst::State::Ready
        || queue.static_pad("sink").is_none_or(|p| p.is_active())
        || queue.static_pad("src").is_none_or(|p| p.is_active())
    {
        return Err(gst::glib::bool_error!(
            "Refusing to resize an active video encode input queue"
        ));
    }
    let leaky = queue
        .property_value("leaky")
        .transform::<i32>()
        .map_err(|_| gst::glib::bool_error!("Unknown input queue leaky enum"))?
        .get::<i32>()
        .map_err(|_| gst::glib::bool_error!("Invalid input queue leaky enum"))?;
    if leaky != 0
        || queue.property::<bool>("flush-on-eos")
        || queue.property::<u32>("min-threshold-buffers") != 0
        || queue.property::<u32>("min-threshold-bytes") != 0
        || queue.property::<u64>("min-threshold-time") != 0
    {
        return Err(gst::glib::bool_error!(
            "Refusing incompatible leaky/flushing/threshold video input queue"
        ));
    }
    queue.set_property("max-size-buffers", 1u32);
    queue.set_property("max-size-bytes", 0u32);
    queue.set_property("max-size-time", 0u64);
    log::info!(
        "Mi Pad 2 video encode INPUT queue bounded: buffers=1 bytes=0 time=0 leaky=no flush-on-eos=false; actual video ghost target, audio/mux/preview unchanged"
    );
    Ok(true)
}

fn watch_encodebin(element: &gst::Element) -> Result<bool, gst::glib::BoolError> {
    if !video_owner(element) {
        return Ok(false);
    }
    // Closures capture neither their owner nor a child: no ownership cycle or
    // retained stale queue after profile teardown. Resolve each new video pad.
    element.connect_pad_added(|owner, pad| {
        if let Err(error) = configure(owner, pad) {
            log::error!("Mi Pad 2 video encode INPUT queue policy rejected: {error}");
        }
    });
    for pad in element.sink_pads() {
        configure(element, &pad)?;
    }
    Ok(true)
}

pub(crate) fn install(camerabin: &gst::Element) -> Result<(), gst::glib::BoolError> {
    if !camerabin.factory().is_some_and(|f| f.name() == "camerabin")
        || camerabin.current_state() > gst::State::Ready
    {
        return Err(gst::glib::bool_error!(
            "Encode input policy requires a pre-streaming camerabin"
        ));
    }
    let bin = camerabin
        .downcast_ref::<gst::Bin>()
        .ok_or_else(|| gst::glib::bool_error!("Camerabin is not a bin"))?;
    bin.connect_deep_element_added(|_, _, element| {
        if let Err(error) = watch_encodebin(element) {
            log::error!("Mi Pad 2 video encode INPUT queue binding rejected: {error}");
        }
    });
    for element in bin.iterate_recurse().into_iter() {
        let element = element
            .map_err(|_| gst::glib::bool_error!("Camerabin changed while binding input queue"))?;
        watch_encodebin(&element)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    fn owner() -> gst::Element {
        gst::init().unwrap();
        gst::ElementFactory::make("encodebin")
            .name("video-encodebin")
            .build()
            .unwrap()
    }
    fn tuple(q: &gst::Element) -> (u32, u32, u64, i32, bool) {
        (
            q.property("max-size-buffers"),
            q.property("max-size-bytes"),
            q.property("max-size-time"),
            q.property_value("leaky")
                .transform::<i32>()
                .unwrap()
                .get()
                .unwrap(),
            q.property("flush-on-eos"),
        )
    }
    fn fixture_with_names(
        identity: bool,
        owner_name: &str,
        pad_name: &str,
    ) -> (gst::Element, gst::Pad, gst::Element) {
        gst::init().unwrap();
        let e = gst::ElementFactory::make("encodebin")
            .name(owner_name)
            .build()
            .unwrap();
        let b = e.downcast_ref::<gst::Bin>().unwrap();
        let q = gst::ElementFactory::make("queue")
            .name("arbitrary-input-name")
            .build()
            .unwrap();
        b.add(&q).unwrap();
        let sink = if identity {
            let id = gst::ElementFactory::make("identity")
                .property("single-segment", true)
                .build()
                .unwrap();
            b.add(&id).unwrap();
            id.link(&q).unwrap();
            id.static_pad("sink").unwrap()
        } else {
            q.static_pad("sink").unwrap()
        };
        let pad = gst::GhostPad::builder_with_target(&sink)
            .unwrap()
            .name(pad_name)
            .build();
        b.add_pad(&pad).unwrap();
        (e, pad.upcast(), q)
    }
    fn fixture(identity: bool) -> (gst::Element, gst::Pad, gst::Element) {
        fixture_with_names(identity, "video-encodebin", "video_37")
    }
    fn jpeg_profile() -> gst_pbutils::EncodingVideoProfile {
        gst_pbutils::EncodingVideoProfile::builder(&gst::Caps::builder("image/jpeg").build())
            .variable_framerate(true)
            .build()
    }
    fn existing_or_request(e: &gst::Element, kind: &str) -> gst::Pad {
        e.sink_pads()
            .into_iter()
            .find(|p| p.name().starts_with(&format!("{kind}_")))
            .unwrap_or_else(|| e.request_pad_simple(&format!("{kind}_%u")).unwrap())
    }
    #[test]
    fn scope_requires_every_validated_condition() {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    for d in [false, true] {
                        for e in [false, true] {
                            assert_eq!(enabled(a, b, c, d, e), a && b && c && d && !e);
                        }
                    }
                }
            }
        }
    }
    #[test]
    fn exact_direct_or_single_segment_target_not_auto_name() {
        for identity in [false, true] {
            let (e, p, q) = fixture(identity);
            assert_eq!(tuple(&q), (200, 10485760, 1000000000, 0, false));
            assert!(configure(&e, &p).unwrap());
            assert_eq!(input_queue(&e, &p).unwrap(), q);
            assert_eq!(tuple(&q), (1, 0, 0, 0, false));
        }
    }
    #[test]
    fn unrelated_audio_image_owner_and_wrong_root_untouched() {
        for name in ["audio_0", "image_0", "video_", "video_bad"] {
            let (e, p, q) = fixture_with_names(false, "video-encodebin", name);
            let before = tuple(&q);
            assert!(!configure(&e, &p).unwrap());
            assert_eq!(tuple(&q), before);
        }
        let (e, p, q) = fixture_with_names(false, "image-encodebin", "video_0");
        let before = tuple(&q);
        assert!(!configure(&e, &p).unwrap());
        assert_eq!(tuple(&q), before);
        assert!(install(&e).is_err());
        assert!(!watch_encodebin(&e).unwrap());
    }
    #[test]
    fn live_leaky_flushing_threshold_or_wrong_identity_rejected() {
        for mode in 0..5 {
            let (e, p, q) = fixture(false);
            match mode {
                0 => {
                    q.static_pad("sink").unwrap().set_active(true).unwrap();
                }
                1 => q.set_property_from_str("leaky", "downstream"),
                2 => q.set_property("flush-on-eos", true),
                3 => q.set_property("min-threshold-buffers", 2u32),
                _ => q.set_property("min-threshold-time", 100u64),
            };
            let before = tuple(&q);
            assert!(configure(&e, &p).is_err());
            assert_eq!(tuple(&q), before);
            q.static_pad("sink").unwrap().set_active(false).unwrap();
        }
        let (e, p, q) = fixture(true);
        let id = p
            .downcast_ref::<gst::GhostPad>()
            .unwrap()
            .target()
            .unwrap()
            .parent_element()
            .unwrap();
        id.set_property("single-segment", false);
        assert!(configure(&e, &p).is_err());
        assert_eq!(tuple(&q).0, 200);
    }
    #[test]
    fn actual_encodebin_video_bound_audio_and_mux_queues_unchanged() {
        let e = owner();
        watch_encodebin(&e).unwrap();
        let profile = gst_pbutils::EncodingContainerProfile::builder(
            &gst::Caps::builder("video/x-matroska").build(),
        )
        .add_profile(jpeg_profile())
        .add_profile(
            gst_pbutils::EncodingAudioProfile::builder(
                &gst::Caps::builder("audio/x-vorbis").build(),
            )
            .build(),
        )
        .build();
        e.set_property("profile", &profile);
        let video = existing_or_request(&e, "video");
        let audio = existing_or_request(&e, "audio");
        let q = input_queue(&e, &video).unwrap();
        assert_eq!(tuple(&q), (1, 0, 0, 0, false));
        let a = audio
            .downcast_ref::<gst::GhostPad>()
            .unwrap()
            .target()
            .unwrap()
            .parent_element()
            .unwrap();
        let audioq = if a.factory().unwrap().name() == "identity" {
            a.static_pad("src")
                .unwrap()
                .peer()
                .unwrap()
                .parent_element()
                .unwrap()
        } else {
            a
        };
        assert_eq!(tuple(&audioq), (200, 10485760, 1000000000, 0, false));
        let mut output = 0;
        for child in e
            .downcast_ref::<gst::Bin>()
            .unwrap()
            .iterate_elements()
            .into_iter()
        {
            let child = child.unwrap();
            if child.factory().is_some_and(|f| f.name() == "queue") && child != q && child != audioq
            {
                assert_eq!(tuple(&child), (0, 0, 3000000000, 0, false));
                output += 1;
            }
        }
        assert_eq!(output, 2);
    }
    #[test]
    fn profile_rebuild_has_no_stale_queue_or_reference_cycle() {
        let e = owner();
        let weak = e.downgrade();
        watch_encodebin(&e).unwrap();
        e.set_property("profile", jpeg_profile());
        let p = existing_or_request(&e, "video");
        let q = input_queue(&e, &p).unwrap();
        let old = q.downgrade();
        drop(q);
        drop(p);
        e.set_property("profile", jpeg_profile());
        let p = existing_or_request(&e, "video");
        assert_eq!(tuple(&input_queue(&e, &p).unwrap()), (1, 0, 0, 0, false));
        assert!(old.upgrade().is_none());
        drop(p);
        drop(e);
        assert!(weak.upgrade().is_none());
    }
    #[test]
    fn real_encodebin_slow_consumer_preserves_all_input_bytes_pts_and_eos() {
        gst::init().unwrap();
        for (width, height) in [(32i32, 24i32), (64, 48)] {
            let pipe = gst::Pipeline::new();
            let e = owner();
            watch_encodebin(&e).unwrap();
            e.set_property("profile", jpeg_profile());
            let input = existing_or_request(&e, "video");
            let q = input_queue(&e, &input).unwrap();
            let source = gst::ElementFactory::make("identity").build().unwrap();
            let sink = gst::ElementFactory::make("fakesink")
                .property("sync", false)
                .property("async", false)
                .build()
                .unwrap();
            pipe.add_many([&source, &e, &sink]).unwrap();
            source.static_pad("src").unwrap().link(&input).unwrap();
            e.link(&sink).unwrap();
            let encoder = e
                .downcast_ref::<gst::Bin>()
                .unwrap()
                .iterate_recurse()
                .into_iter()
                .map(Result::unwrap)
                .find(|c| c.factory().is_some_and(|f| f.name() == "jpegenc"))
                .unwrap();
            let seen = Arc::new(Mutex::new(Vec::new()));
            let received = seen.clone();
            let weakq = q.downgrade();
            encoder.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::BUFFER,
                move |_, info| {
                    let b = info.buffer().unwrap();
                    std::thread::sleep(std::time::Duration::from_millis(10));
                    assert!(
                        weakq
                            .upgrade()
                            .unwrap()
                            .property::<u32>("current-level-buffers")
                            <= 1
                    );
                    received.lock().unwrap().push((
                        b.pts(),
                        b.duration(),
                        b.offset(),
                        b.map_readable().unwrap().as_slice().to_vec(),
                    ));
                    gst::PadProbeReturn::Ok
                },
            );
            let out = Arc::new(Mutex::new(Vec::new()));
            let encoded = out.clone();
            encoder.static_pad("src").unwrap().add_probe(
                gst::PadProbeType::BUFFER,
                move |_, info| {
                    encoded.lock().unwrap().push(info.buffer().unwrap().pts());
                    gst::PadProbeReturn::Ok
                },
            );
            pipe.set_state(gst::State::Playing).unwrap();
            let pad = source.static_pad("src").unwrap();
            assert!(pad.push_event(gst::event::StreamStart::new("queue-regression")));
            let caps = gst::Caps::builder("video/x-raw")
                .field("format", "I420")
                .field("width", width)
                .field("height", height)
                .field("framerate", gst::Fraction::new(30, 1))
                .build();
            assert!(pad.push_event(gst::event::Caps::new(&caps)));
            assert!(
                pad.push_event(gst::event::Segment::new(&gst::FormattedSegment::<
                    gst::ClockTime,
                >::new()))
            );
            let mut expected = Vec::new();
            for index in 0..16u64 {
                let bytes = vec![16 + (index as u8) * 3; (width * height * 3 / 2) as usize];
                let mut b = gst::Buffer::from_mut_slice(bytes.clone());
                let time = gst::ClockTime::from_nseconds(index * 33333333);
                let duration = gst::ClockTime::from_nseconds(33333333);
                {
                    let b = b.get_mut().unwrap();
                    b.set_pts(time);
                    b.set_duration(duration);
                    b.set_offset(index);
                }
                expected.push((Some(time), Some(duration), index, bytes));
                assert_eq!(pad.push(b), Ok(gst::FlowSuccess::Ok));
            }
            assert!(pad.push_event(gst::event::Eos::new()));
            let message = pipe
                .bus()
                .unwrap()
                .timed_pop_filtered(
                    gst::ClockTime::from_seconds(10),
                    &[gst::MessageType::Eos, gst::MessageType::Error],
                )
                .expect("encode EOS timeout");
            pipe.set_state(gst::State::Null).unwrap();
            assert!(
                matches!(message.view(), gst::MessageView::Eos(_)),
                "{message:?}"
            );
            assert_eq!(*seen.lock().unwrap(), expected);
            assert_eq!(
                *out.lock().unwrap(),
                expected.iter().map(|v| v.0).collect::<Vec<_>>()
            );
            assert_eq!(tuple(&q), (1, 0, 0, 0, false));
        }
    }
}
