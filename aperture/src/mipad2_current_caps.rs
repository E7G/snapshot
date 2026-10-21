// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in recommendation for an already negotiated, explicitly pinned native source.
//! Never answers initial negotiation, changed modes, ACCEPT_CAPS, ALLOCATION or
//! serialized CAPS events. No capability cache, event suppression or buffer edits.
use gst::prelude::*;

pub(crate) fn enabled(device: bool, requested: bool, validated_raw_direct: bool) -> bool {
    device && requested && validated_raw_direct
}

fn native_fixed(caps: &gst::CapsRef) -> bool {
    if !caps.is_fixed() || caps.size() != 1 {
        return false;
    }
    let template = gst::Caps::builder("video/x-raw")
        .field("format", "NV12")
        .build();
    if !caps.is_subset(&template) {
        return false;
    }
    let s = caps.structure(0).unwrap();
    s.get::<i32>("width").is_ok_and(|v| v > 0)
        && s.get::<i32>("height").is_ok_and(|v| v > 0)
        && s.get::<gst::Fraction>("framerate")
            .is_ok_and(|v| v.numer() > 0 && v.denom() > 0)
}

pub(crate) fn recommendation(
    configured: &gst::CapsRef,
    negotiated: Option<&gst::CapsRef>,
    filter: Option<&gst::CapsRef>,
) -> Option<gst::Caps> {
    let current = negotiated?;
    // Requested configuration, not just last-seen dimensions, must still agree.
    // A photo/video mode change falls through until real negotiation completes.
    if !native_fixed(configured) || !native_fixed(current) || !current.is_subset(configured) {
        return None;
    }
    let answer = match filter {
        Some(filter) => filter.intersect(current),
        None => current.to_owned(),
    };
    // A new incompatible filter may require full downstream renegotiation
    // (for example changed colorimetry). Never cache an empty recommendation.
    (!answer.is_empty()).then_some(answer)
}

pub(crate) fn install(source_caps: &gst::Element) -> Result<(), gst::glib::BoolError> {
    if !source_caps
        .factory()
        .is_some_and(|f| f.name() == "capsfilter")
    {
        return Err(gst::glib::bool_error!(
            "Native caps recommendation needs the source capsfilter"
        ));
    }
    let pad = source_caps
        .static_pad("sink")
        .ok_or_else(|| gst::glib::bool_error!("Source capsfilter has no sink"))?;
    let weak = source_caps.downgrade();
    pad.add_probe(gst::PadProbeType::QUERY_DOWNSTREAM, move |pad, info| {
        let Some(query) = info.query_mut() else {
            return gst::PadProbeReturn::Ok;
        };
        let gst::QueryViewMut::Caps(query) = query.view_mut() else {
            return gst::PadProbeReturn::Ok;
        };
        let Some(element) = weak.upgrade() else {
            return gst::PadProbeReturn::Ok;
        };
        let configured = element.property::<gst::Caps>("caps");
        let current = pad.current_caps();
        if let Some(answer) = recommendation(&configured, current.as_deref(), query.filter()) {
            query.set_result(Some(&answer));
            return gst::PadProbeReturn::Handled;
        }
        gst::PadProbeReturn::Ok
    });
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    fn caps(width: i32, height: i32, rate: i32) -> gst::Caps {
        gst::init().unwrap();
        gst::Caps::builder("video/x-raw")
            .field("format", "NV12")
            .field("width", width)
            .field("height", height)
            .field("framerate", gst::Fraction::new(rate, 1))
            .build()
    }
    #[test]
    fn scope_requires_device_request_and_validated_direct() {
        for a in [false, true] {
            for b in [false, true] {
                for c in [false, true] {
                    assert_eq!(enabled(a, b, c), a && b && c);
                }
            }
        }
    }
    #[test]
    fn unnegotiated_unfixed_and_incomplete_configuration_fall_through() {
        let native = caps(1920, 1080, 30);
        assert!(recommendation(&native, None, None).is_none());
        for text in [
            "ANY",
            "EMPTY",
            "video/x-raw,format=NV12",
            "video/x-raw,format=NV12,width=1920,height=1080,framerate=[0/1,30/1]",
            "video/x-raw,format=NV12,width=0,height=1080,framerate=30/1",
        ] {
            let configured: gst::Caps = text.parse().unwrap();
            assert!(
                recommendation(&configured, Some(&native), None).is_none(),
                "{text}"
            );
        }
    }
    #[test]
    fn mode_changes_are_not_frozen_to_old_video_or_photo_caps() {
        let modes = [
            caps(1920, 1080, 30),
            caps(2560, 1920, 15),
            caps(3264, 2448, 15),
            caps(1920, 1080, 30),
        ];
        for pair in modes.windows(2) {
            assert!(recommendation(&pair[1], Some(&pair[0]), None).is_none());
            assert_eq!(
                recommendation(&pair[1], Some(&pair[1]), None).unwrap(),
                pair[1]
            );
        }
    }
    #[test]
    fn current_colorimetry_and_filter_intersections_are_preserved() {
        let configured = caps(1920, 1080, 30);
        let mut current = configured.clone();
        current
            .make_mut()
            .structure_mut(0)
            .unwrap()
            .set("colorimetry", "bt709");
        assert_eq!(
            recommendation(&configured, Some(&current), None).unwrap(),
            current
        );
        let impossible = caps(2560, 1920, 15);
        assert!(recommendation(&configured, Some(&current), Some(&impossible)).is_none());
        let incompatible_color: gst::Caps =
            "video/x-raw,format=NV12,colorimetry=bt601".parse().unwrap();
        assert!(recommendation(&configured, Some(&current), Some(&incompatible_color)).is_none());
        let filter: gst::Caps = "video/x-raw,format=NV12,colorimetry=bt709".parse().unwrap();
        assert_eq!(
            recommendation(&configured, Some(&current), Some(&filter)).unwrap(),
            current
        );
    }
    #[test]
    fn never_claims_other_formats_or_memory_domains() {
        let native = caps(1920, 1080, 30);
        for text in [
            "video/x-raw(memory:GLMemory),format=NV12,width=1920,height=1080,framerate=30/1",
            "video/x-raw(memory:DMABuf),format=NV12,width=1920,height=1080,framerate=30/1",
            "video/x-raw,format=I420,width=1920,height=1080,framerate=30/1",
        ] {
            let other: gst::Caps = text.parse().unwrap();
            assert!(recommendation(&native, Some(&other), None).is_none());
            assert!(recommendation(&other, Some(&other), None).is_none());
            assert!(recommendation(&native, Some(&native), Some(&other)).is_none());
        }
    }
    #[test]
    fn probe_has_no_element_reference_cycle_and_rejects_wrong_element() {
        gst::init().unwrap();
        let e = gst::ElementFactory::make("capsfilter").build().unwrap();
        let weak = e.downgrade();
        install(&e).unwrap();
        drop(e);
        assert!(weak.upgrade().is_none());
        assert!(install(&gst::ElementFactory::make("identity").build().unwrap()).is_err());
    }
    struct Graph {
        pipe: gst::Pipeline,
        source: gst::Element,
        fixed: gst::Element,
        sink: gst::Element,
    }
    impl Drop for Graph {
        fn drop(&mut self) {
            let _ = self.pipe.set_state(gst::State::Null);
        }
    }
    fn graph(boundary: bool) -> Graph {
        gst::init().unwrap();
        let pipe = gst::Pipeline::new();
        let source = gst::ElementFactory::make("identity").build().unwrap();
        let fixed = gst::ElementFactory::make("capsfilter")
            .property("caps", caps(1920, 1080, 30))
            .build()
            .unwrap();
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("async", false)
            .property("enable-last-sample", false)
            .property("signal-handoffs", true)
            .build()
            .unwrap();
        if boundary {
            install(&fixed).unwrap();
        }
        pipe.add_many([&source, &fixed, &sink]).unwrap();
        gst::Element::link_many([&source, &fixed, &sink]).unwrap();
        Graph {
            pipe,
            source,
            fixed,
            sink,
        }
    }
    fn begin(g: &Graph, c: &gst::Caps) {
        g.pipe.set_state(gst::State::Playing).unwrap();
        let p = g.source.static_pad("src").unwrap();
        assert!(p.push_event(gst::event::StreamStart::new("native-current-caps")));
        assert!(p.push_event(gst::event::Caps::new(c)));
        assert!(
            p.push_event(gst::event::Segment::new(&gst::FormattedSegment::<
                gst::ClockTime,
            >::new()))
        );
    }
    #[test]
    fn actual_repeated_caps_queries_skip_downstream_only_after_negotiation() {
        for installed in [false, true] {
            let g = graph(installed);
            let calls = Arc::new(AtomicUsize::new(0));
            let seen = calls.clone();
            g.sink.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::QUERY_DOWNSTREAM,
                move |_, info| {
                    if let Some(q) = info.query_mut() {
                        if let gst::QueryViewMut::Caps(q) = q.view_mut() {
                            seen.fetch_add(1, Ordering::SeqCst);
                            q.set_result(Some(&caps(1920, 1080, 30)));
                            return gst::PadProbeReturn::Handled;
                        }
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            let p = g.source.static_pad("src").unwrap();
            for e in [&g.source, &g.fixed, &g.sink] {
                for pad in e.pads() {
                    pad.set_active(true).unwrap();
                }
            }
            assert!(!p.peer_query_caps(None).is_empty());
            assert!(calls.load(Ordering::SeqCst) > 0);
            begin(&g, &caps(1920, 1080, 30));
            calls.store(0, Ordering::SeqCst);
            for _ in 0..12 {
                assert_eq!(p.peer_query_caps(None), caps(1920, 1080, 30));
            }
            assert_eq!(calls.load(Ordering::SeqCst) == 0, installed);
            let impossible: gst::Caps =
                "video/x-raw,format=I420,width=1920,height=1080,framerate=30/1"
                    .parse()
                    .unwrap();
            assert!(!p.peer_query_accept_caps(&impossible));
            let probe_seen = Arc::new(AtomicUsize::new(0));
            let seen = probe_seen.clone();
            g.fixed.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::QUERY_DOWNSTREAM,
                move |_, info| {
                    if info
                        .query()
                        .is_some_and(|q| matches!(q.view(), gst::QueryView::Allocation(_)))
                    {
                        seen.fetch_add(1, Ordering::SeqCst);
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            let mut allocation = gst::query::Allocation::new(None, false);
            let _ = p.peer_query(&mut allocation);
            assert!(probe_seen.load(Ordering::SeqCst) > 0);
        }
    }
    #[test]
    fn full_video_photo_transitions_preserve_every_buffer_timing_and_eos() {
        let g = graph(true);
        let seen = Arc::new(Mutex::new(Vec::new()));
        let collected = seen.clone();
        g.sink.connect("handoff", false, move |args| {
            let b = args[1].get::<gst::Buffer>().unwrap();
            let pad = args[2].get::<gst::Pad>().unwrap();
            let info = gst_video::VideoInfo::from_caps(&pad.current_caps().unwrap()).unwrap();
            let bytes = b.map_readable().unwrap();
            assert!(bytes.as_slice().iter().all(|v| *v == b.offset() as u8));
            collected.lock().unwrap().push((
                b.offset(),
                b.pts(),
                b.duration(),
                info.width(),
                info.height(),
                b.size(),
            ));
            None
        });
        let mut expected = Vec::new();
        let mut now = 0;
        for (epoch, (width, height, rate)) in [
            (1920, 1080, 30),
            (2560, 1920, 15),
            (3264, 2448, 15),
            (1920, 1080, 30),
        ]
        .into_iter()
        .enumerate()
        {
            let c = caps(width, height, rate);
            g.fixed.set_property("caps", &c);
            if epoch == 0 {
                begin(&g, &c);
            } else {
                // No current-format recommendation until the real new CAPS event.
                assert!(
                    recommendation(
                        &c,
                        g.fixed
                            .static_pad("sink")
                            .unwrap()
                            .current_caps()
                            .as_deref(),
                        None
                    )
                    .is_none()
                );
                assert!(
                    g.source
                        .static_pad("src")
                        .unwrap()
                        .push_event(gst::event::Caps::new(&c))
                );
            }
            assert!(
                g.source
                    .static_pad("src")
                    .unwrap()
                    .peer_query_caps(None)
                    .is_subset(&c)
            );
            let size = gst_video::VideoInfo::from_caps(&c).unwrap().size();
            let duration = 1_000_000_000 / rate as u64;
            for _ in 0..2 {
                let n = expected.len() as u64;
                let mut b = gst::Buffer::from_mut_slice(vec![n as u8; size]);
                let bmut = b.get_mut().unwrap();
                bmut.set_offset(n);
                bmut.set_pts(gst::ClockTime::from_nseconds(now));
                bmut.set_duration(gst::ClockTime::from_nseconds(duration));
                expected.push((
                    n,
                    Some(gst::ClockTime::from_nseconds(now)),
                    Some(gst::ClockTime::from_nseconds(duration)),
                    width as u32,
                    height as u32,
                    size,
                ));
                now += duration;
                assert_eq!(
                    g.source.static_pad("src").unwrap().push(b),
                    Ok(gst::FlowSuccess::Ok)
                );
            }
        }
        assert!(
            g.source
                .static_pad("src")
                .unwrap()
                .push_event(gst::event::Eos::new())
        );
        let m = g
            .pipe
            .bus()
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(5),
                &[gst::MessageType::Error, gst::MessageType::Eos],
            )
            .unwrap();
        assert!(matches!(m.view(), gst::MessageView::Eos(_)), "{m:?}");
        assert_eq!(*seen.lock().unwrap(), expected);
    }
}
