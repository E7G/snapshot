// SPDX-License-Identifier: GPL-3.0-or-later
//! Strict opt-in tail for the known physical AtomISP system-memory NV12 source.
//! Raw frames do not need decodebin3's demux/stream-selection/multiqueue layer.
//! This removes that layer, not recording data, resolution, rate or audio.
use gst::prelude::*;

pub(crate) fn direct_enabled(
    device: bool,
    requested: bool,
    factory: &str,
    path: Option<&str>,
    native_profile: bool,
    copy: bool,
    guarded: bool,
    no_source_queue: bool,
    native_atomisp: bool,
) -> Result<bool, gst::glib::BoolError> {
    if !device || !requested {
        return Ok(false);
    }
    if !native_atomisp
        || factory != "v4l2src"
        || path != Some("/dev/video0")
        || !native_profile
        || !copy
        || !guarded
        || !no_source_queue
    {
        return Err(gst::glib::bool_error!(
            "Mi Pad 2 raw direct requires verified AtomISP physical /dev/video0 v4l2src, native NV12 profile, COPY1, ALLOCATION_GUARD1 and both source queues OFF"
        ));
    }
    Ok(true)
}

pub(crate) struct CameraTail {
    pub(crate) output: gst::Element,
    pub(crate) decoder: Option<gst::Element>,
}

impl CameraTail {
    pub(crate) fn new(
        bin: &gst::Bin,
        source_caps: &gst::Element,
        direct: bool,
    ) -> Result<Self, gst::glib::BoolError> {
        let mut builder = gst::ElementFactory::make("capsfilter");
        if direct {
            builder = builder.name("mipad2-native-raw-output");
        }
        let output = builder.build()?;
        let caps = if direct {
            // No ANY memory features or compressed/other raw format fallback.
            // Geometry/rate remain the upstream native video/full-photo modes.
            gst::Caps::builder("video/x-raw")
                .field("format", "NV12")
                .build()
        } else {
            gst::Caps::builder("video/x-raw").build()
        };
        output.set_property("caps", &caps);
        bin.add(&output)?;
        let decoder = if direct {
            source_caps.link(&output)?;
            None
        } else {
            let decoder = gst::ElementFactory::make("decodebin3").build()?;
            bin.add(&decoder)?;
            source_caps.link(&decoder)?;
            // The caller retains its existing dynamic video-pad/error handler.
            Some(decoder)
        };
        Ok(Self { output, decoder })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };

    fn scope(
        device: bool,
        requested: bool,
        factory: &str,
        path: Option<&str>,
        profile: bool,
        copy: bool,
        guard: bool,
        no_queue: bool,
    ) -> Result<bool, gst::glib::BoolError> {
        direct_enabled(
            device, requested, factory, path, profile, copy, guard, no_queue, true,
        )
    }

    #[test]
    fn direct_scope_is_explicit_and_rejects_invalid_native_requests() {
        assert!(
            direct_enabled(
                true,
                true,
                "v4l2src",
                Some("/dev/video0"),
                true,
                true,
                true,
                true,
                false
            )
            .is_err()
        );
        assert!(
            scope(
                true,
                true,
                "v4l2src",
                Some("/dev/video0"),
                true,
                true,
                true,
                true
            )
            .unwrap()
        );
        for device in [false, true] {
            for requested in [false, true] {
                if !device || !requested {
                    assert!(
                        !scope(
                            device,
                            requested,
                            "libcamerasrc",
                            None,
                            false,
                            false,
                            false,
                            false
                        )
                        .unwrap()
                    );
                }
            }
        }
        for (factory, path) in [
            ("libcamerasrc", Some("/dev/video0")),
            ("v4l2src", Some("/dev/video1")),
            ("v4l2src", None),
        ] {
            assert!(scope(true, true, factory, path, true, true, true, true).is_err());
        }
        for (profile, copy, guard, no_queue) in [
            (false, true, true, true),
            (true, false, true, true),
            (true, true, false, true),
            (true, true, true, false),
        ] {
            assert!(
                scope(
                    true,
                    true,
                    "v4l2src",
                    Some("/dev/video0"),
                    profile,
                    copy,
                    guard,
                    no_queue
                )
                .is_err()
            );
        }
    }

    struct Graph {
        pipe: gst::Pipeline,
        source: gst::Element,
        tail: CameraTail,
        sink: gst::Element,
    }
    impl Drop for Graph {
        fn drop(&mut self) {
            let _ = self.pipe.set_state(gst::State::Null);
        }
    }
    fn graph(copy: bool) -> Graph {
        gst::init().unwrap();
        let pipe = gst::Pipeline::new();
        let source = gst::ElementFactory::make("identity").build().unwrap();
        if copy {
            crate::mipad2_capture_copy::install(&source);
        }
        let guard = crate::mipad2_capture_copy::source_allocation_guard().unwrap();
        let input = gst::ElementFactory::make("capsfilter").build().unwrap();
        pipe.add_many([&source, &guard, &input]).unwrap();
        gst::Element::link_many([&source, &guard, &input]).unwrap();
        let tail = CameraTail::new(pipe.upcast_ref(), &input, true).unwrap();
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("async", false)
            .property("enable-last-sample", false)
            .property("signal-handoffs", true)
            .build()
            .unwrap();
        pipe.add(&sink).unwrap();
        tail.output.link(&sink).unwrap();
        Graph {
            pipe,
            source,
            tail,
            sink,
        }
    }
    fn begin(pad: &gst::Pad, caps: &gst::Caps, name: &str) {
        assert!(pad.push_event(gst::event::StreamStart::new(name)));
        assert!(pad.push_event(gst::event::Caps::new(caps)));
        assert!(
            pad.push_event(gst::event::Segment::new(&gst::FormattedSegment::<
                gst::ClockTime,
            >::new()))
        );
    }
    fn eos(g: &Graph) {
        assert!(
            g.source
                .static_pad("src")
                .unwrap()
                .push_event(gst::event::Eos::new())
        );
        let msg = g
            .pipe
            .bus()
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(3),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            )
            .unwrap();
        assert!(matches!(msg.view(), gst::MessageView::Eos(_)), "{msg:?}");
    }

    #[test]
    fn direct_has_no_decoder_or_queue_and_default_keeps_decodebin3() {
        let g = graph(false);
        assert!(g.tail.decoder.is_none());
        for e in g.pipe.children() {
            assert!(!matches!(
                e.factory().unwrap().name().as_str(),
                "decodebin3" | "queue" | "multiqueue"
            ));
        }
        let legacy = gst::Bin::new();
        let input = gst::ElementFactory::make("capsfilter").build().unwrap();
        legacy.add(&input).unwrap();
        let tail = CameraTail::new(&legacy, &input, false).unwrap();
        assert_eq!(
            tail.decoder.unwrap().factory().unwrap().name(),
            "decodebin3"
        );
        assert_eq!(
            tail.output.property::<gst::Caps>("caps"),
            gst::Caps::builder("video/x-raw").build()
        );
    }

    #[test]
    fn direct_rejects_compressed_other_raw_and_non_system_memory_caps() {
        let g = graph(false);
        let pad = g.tail.output.static_pad("sink").unwrap();
        for text in [
            "image/jpeg,width=8,height=6",
            "video/x-raw,format=I420,width=8,height=6",
            "video/x-raw,format=RGBA,width=8,height=6",
            "video/x-raw(memory:DMABuf),format=NV12,width=8,height=6",
        ] {
            let caps: gst::Caps = text.parse().unwrap();
            assert!(!pad.query_accept_caps(&caps), "{caps:?}");
        }
    }

    #[test]
    fn direct_preserves_padded_nv12_metadata_timing_events_and_releases_native_copy() {
        let g = graph(true);
        let received = Arc::new(Mutex::new(Vec::new()));
        let got = received.clone();
        let released = Arc::new(AtomicUsize::new(0));
        let release_check = released.clone();
        let caller = std::thread::current().id();
        g.sink.connect("handoff", false, move |values| {
            let buffer = values[1].get::<gst::Buffer>().unwrap();
            let meta = buffer.meta::<gst_video::VideoMeta>().unwrap();
            assert_eq!(std::thread::current().id(), caller);
            assert!(release_check.load(Ordering::SeqCst) > buffer.offset() as usize);
            got.lock().unwrap().push((
                buffer.map_readable().unwrap().as_slice().to_vec(),
                buffer.pts(),
                buffer.dts(),
                buffer.duration(),
                buffer.offset(),
                buffer.flags(),
                meta.offset().to_vec(),
                meta.stride().to_vec(),
            ));
            None
        });
        let events = Arc::new(Mutex::new(Vec::new()));
        let event_log = events.clone();
        g.sink.static_pad("sink").unwrap().add_probe(
            gst::PadProbeType::EVENT_DOWNSTREAM,
            move |_, info| {
                if let Some(e) = info.event() {
                    event_log.lock().unwrap().push(e.type_());
                }
                gst::PadProbeReturn::Ok
            },
        );
        g.pipe.set_state(gst::State::Playing).unwrap();
        let pad = g.source.static_pad("src").unwrap();
        let caps: gst::Caps =
            "video/x-raw,format=NV12,width=8,height=6,framerate=30/1,colorimetry=bt709"
                .parse()
                .unwrap();
        begin(&pad, &caps, "direct-padded");
        struct Lease(Vec<u8>, Arc<AtomicUsize>);
        impl AsMut<[u8]> for Lease {
            fn as_mut(&mut self) -> &mut [u8] {
                &mut self.0
            }
        }
        impl Drop for Lease {
            fn drop(&mut self) {
                self.1.fetch_add(1, Ordering::SeqCst);
            }
        }
        for n in 0..12u64 {
            let mut b = gst::Buffer::from_mut_slice(Lease(vec![n as u8; 144], released.clone()));
            let b_mut = b.get_mut().unwrap();
            b_mut.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
            b_mut.set_dts(gst::ClockTime::from_nseconds(n * 33_333_333));
            b_mut.set_duration(gst::ClockTime::from_nseconds(33_333_333));
            b_mut.set_offset(n);
            b_mut.set_flags(gst::BufferFlags::DISCONT);
            gst_video::VideoMeta::add_full(
                b_mut,
                gst_video::VideoFrameFlags::empty(),
                gst_video::VideoFormat::Nv12,
                8,
                6,
                &[0, 96],
                &[16, 16],
            )
            .unwrap();
            assert_eq!(pad.push(b), Ok(gst::FlowSuccess::Ok));
        }
        eos(&g);
        let frames = received.lock().unwrap();
        assert_eq!(frames.len(), 12);
        assert_eq!(released.load(Ordering::SeqCst), 12);
        for (n, f) in frames.iter().enumerate() {
            assert_eq!(f.0, vec![n as u8; 144]);
            assert_eq!(
                f.1,
                Some(gst::ClockTime::from_nseconds(n as u64 * 33_333_333))
            );
            assert_eq!(f.1, f.2);
            assert_eq!(f.3, Some(gst::ClockTime::from_nseconds(33_333_333)));
            assert_eq!(f.4, n as u64);
            assert!(f.5.contains(gst::BufferFlags::DISCONT));
            assert_eq!(&f.6[..2], &[0, 96]);
            assert_eq!(&f.7[..2], &[16, 16]);
        }
        assert_eq!(
            g.tail.output.static_pad("src").unwrap().current_caps(),
            Some(caps)
        );
        let ev = events.lock().unwrap();
        for required in [
            gst::EventType::StreamStart,
            gst::EventType::Caps,
            gst::EventType::Segment,
            gst::EventType::Eos,
        ] {
            assert!(ev.contains(&required));
        }
    }

    #[test]
    fn direct_video_and_both_full_photo_caps_survive_null_reopen() {
        let g = graph(false);
        let (tx, rx) = std::sync::mpsc::channel();
        g.sink.connect("handoff", false, move |v| {
            tx.send(v[1].get::<gst::Buffer>().unwrap().size()).unwrap();
            None
        });
        for (w, h, fps) in [
            (1920, 1080, 30),
            (2560, 1920, 15),
            (3264, 2448, 15),
            (1920, 1080, 30),
        ] {
            let caps = gst::Caps::builder("video/x-raw")
                .field("format", "NV12")
                .field("width", w)
                .field("height", h)
                .field("framerate", gst::Fraction::new(fps, 1))
                .build();
            let size = gst_video::VideoInfo::from_caps(&caps).unwrap().size();
            g.pipe.set_state(gst::State::Playing).unwrap();
            let pad = g.source.static_pad("src").unwrap();
            begin(&pad, &caps, "direct-mode-reopen");
            let mut b = gst::Buffer::with_size(size).unwrap();
            b.get_mut().unwrap().set_pts(gst::ClockTime::ZERO);
            assert_eq!(pad.push(b), Ok(gst::FlowSuccess::Ok));
            assert_eq!(
                rx.recv_timeout(std::time::Duration::from_secs(2)).unwrap(),
                size
            );
            eos(&g);
            assert_eq!(
                g.tail.output.static_pad("src").unwrap().current_caps(),
                Some(caps)
            );
            g.pipe.set_state(gst::State::Null).unwrap();
        }
    }

    #[test]
    fn direct_keeps_allocation_guard_and_latency_query_passthrough() {
        let g = graph(false);
        let downstream = Arc::new(AtomicUsize::new(0));
        let seen = downstream.clone();
        g.sink.static_pad("sink").unwrap().add_probe(
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
        let caps: gst::Caps = "video/x-raw,format=NV12,width=8,height=6,framerate=30/1"
            .parse()
            .unwrap();
        let mut q = gst::query::Allocation::new(Some(&caps), true);
        assert!(!g.source.static_pad("src").unwrap().peer_query(&mut q));
        assert_eq!(downstream.load(Ordering::SeqCst), 0);
        // A normal upstream latency query must not be rejected like ALLOCATION.
        let latency_seen = Arc::new(AtomicUsize::new(0));
        let counted = latency_seen.clone();
        g.source.static_pad("src").unwrap().add_probe(
            gst::PadProbeType::QUERY_UPSTREAM | gst::PadProbeType::PUSH,
            move |_, info| {
                if info
                    .query()
                    .is_some_and(|q| matches!(q.view(), gst::QueryView::Latency(_)))
                {
                    counted.fetch_add(1, Ordering::SeqCst);
                }
                gst::PadProbeReturn::Ok
            },
        );
        let mut q = gst::query::Latency::new();
        let _ = g.sink.static_pad("sink").unwrap().peer_query(&mut q);
        assert_eq!(latency_seen.load(Ordering::SeqCst), 1);
    }
    #[test]
    fn direct_flush_cancels_consumer_and_new_stream_has_no_stale_tail() {
        use std::sync::{Condvar, atomic::AtomicBool, mpsc};
        use std::time::{Duration, Instant};
        let g = graph(true);
        let pad = g.source.static_pad("src").unwrap();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        struct Unblock(Arc<(Mutex<bool>, Condvar)>);
        impl Drop for Unblock {
            fn drop(&mut self) {
                *self.0.0.lock().unwrap() = true;
                self.0.1.notify_all();
            }
        }
        let _unblock = Unblock(gate.clone());
        let flush_seen = Arc::new(AtomicBool::new(false));
        let observed = flush_seen.clone();
        let flush_gate = gate.clone();
        g.sink.static_pad("sink").unwrap().add_probe(
            gst::PadProbeType::EVENT_DOWNSTREAM | gst::PadProbeType::EVENT_FLUSH,
            move |_, info| {
                if info
                    .event()
                    .is_some_and(|e| matches!(e.view(), gst::EventView::FlushStart(_)))
                {
                    observed.store(true, Ordering::SeqCst);
                    *flush_gate.0.lock().unwrap() = true;
                    flush_gate.1.notify_all();
                }
                gst::PadProbeReturn::Ok
            },
        );
        let (entered_tx, entered_rx) = mpsc::channel();
        let (frames_tx, frames_rx) = mpsc::channel();
        let worker_gate = gate.clone();
        let first = Arc::new(AtomicBool::new(true));
        g.sink.connect("handoff", false, move |v| {
            let b = v[1].get::<gst::Buffer>().unwrap();
            frames_tx
                .send((b.offset(), b.map_readable().unwrap().as_slice().to_vec()))
                .unwrap();
            if first.swap(false, Ordering::SeqCst) {
                entered_tx.send(()).unwrap();
                let lock = worker_gate.0.lock().unwrap();
                let _ = worker_gate
                    .1
                    .wait_timeout_while(lock, Duration::from_secs(5), |ready| !*ready);
            }
            None
        });
        g.pipe.set_state(gst::State::Playing).unwrap();
        let caps: gst::Caps = "video/x-raw,format=NV12,width=8,height=6,framerate=30/1"
            .parse()
            .unwrap();
        begin(&pad, &caps, "raw-flush-before");
        let (done_tx, done_rx) = mpsc::channel();
        let producer_pad = pad.clone();
        let producer = std::thread::spawn(move || {
            for n in 0..4u64 {
                let mut b = gst::Buffer::from_mut_slice(vec![n as u8; 72]);
                let m = b.get_mut().unwrap();
                m.set_offset(n);
                m.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
                let flow = producer_pad.push(b);
                if flow.is_err() {
                    done_tx.send(flow).unwrap();
                    return;
                }
            }
            done_tx.send(Ok(gst::FlowSuccess::Ok)).unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let started = Instant::now();
        assert!(pad.push_event(gst::event::FlushStart::new()));
        let interrupted = done_rx.recv_timeout(Duration::from_secs(1));
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        producer.join().unwrap();
        assert!(started.elapsed() < Duration::from_secs(1));
        assert!(flush_seen.load(Ordering::SeqCst));
        assert_eq!(interrupted.unwrap(), Err(gst::FlowError::Flushing));
        assert!(pad.push_event(gst::event::FlushStop::new(true)));
        begin(&pad, &caps, "raw-flush-after");
        for n in 20..32u64 {
            let mut b = gst::Buffer::from_mut_slice(vec![n as u8; 72]);
            let m = b.get_mut().unwrap();
            m.set_offset(n);
            m.set_pts(gst::ClockTime::from_nseconds((n - 20) * 33_333_333));
            assert_eq!(pad.push(b), Ok(gst::FlowSuccess::Ok));
        }
        eos(&g);
        let frames: Vec<_> = frames_rx.try_iter().collect();
        assert_eq!(frames.len(), 13);
        assert_eq!(frames[0].0, 0);
        for (n, (offset, bytes)) in frames.iter().skip(1).enumerate() {
            assert_eq!(*offset, n as u64 + 20);
            assert_eq!(bytes, &vec![*offset as u8; 72]);
        }
    }

    #[test]
    fn default_decoder_still_decodes_compressed_camera_stream() {
        gst::init().unwrap();
        let pipe = gst::Pipeline::new();
        struct Stop(gst::Pipeline);
        impl Drop for Stop {
            fn drop(&mut self) {
                let _ = self.0.set_state(gst::State::Null);
            }
        }
        let _stop = Stop(pipe.clone());
        let src = gst::ElementFactory::make("videotestsrc")
            .property("num-buffers", 3i32)
            .build()
            .unwrap();
        let raw = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("format", "I420")
                    .field("width", 32i32)
                    .field("height", 24i32)
                    .field("framerate", gst::Fraction::new(30, 1))
                    .build(),
            )
            .build()
            .unwrap();
        let jpeg = gst::ElementFactory::make("jpegenc")
            .build()
            .expect("Host compressed-camera regression test requires jpegenc");
        let input = gst::ElementFactory::make("capsfilter").build().unwrap();
        pipe.add_many([&src, &raw, &jpeg, &input]).unwrap();
        gst::Element::link_many([&src, &raw, &jpeg, &input]).unwrap();
        let tail = CameraTail::new(pipe.upcast_ref(), &input, false).unwrap();
        let output = tail.output.clone();
        tail.decoder
            .as_ref()
            .unwrap()
            .connect_pad_added(move |_, pad| {
                if pad
                    .stream()
                    .is_some_and(|s| matches!(s.stream_type(), gst::StreamType::VIDEO))
                {
                    pad.link(&output.static_pad("sink").unwrap()).unwrap();
                }
            });
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("async", false)
            .property("signal-handoffs", true)
            .build()
            .unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let seen = count.clone();
        sink.connect("handoff", false, move |_| {
            seen.fetch_add(1, Ordering::SeqCst);
            None
        });
        pipe.add(&sink).unwrap();
        tail.output.link(&sink).unwrap();
        pipe.set_state(gst::State::Playing).unwrap();
        let msg = pipe
            .bus()
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(5),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            )
            .unwrap();
        assert!(matches!(msg.view(), gst::MessageView::Eos(_)), "{msg:?}");
        assert_eq!(count.load(Ordering::SeqCst), 3);
        let caps = tail
            .output
            .static_pad("src")
            .unwrap()
            .current_caps()
            .unwrap();
        let info = gst_video::VideoInfo::from_caps(&caps).unwrap();
        assert_eq!((info.width(), info.height()), (32, 24));
    }
}
