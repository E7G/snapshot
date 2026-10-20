// SPDX-License-Identifier: GPL-3.0-or-later
//! Diagnostic lease isolation: downstream never retains scarce MMAP buffers.
use gst::prelude::*;

pub(crate) fn enabled(device: bool, opt_in: bool, factory: &str) -> bool {
    device && opt_in && factory == "v4l2src"
}

pub(crate) fn install(source: &gst::Element) {
    let Some(pad) = source.static_pad("src") else {
        return;
    };
    log::info!("Mi Pad 2 diagnostic native MMAP lease isolation enabled");
    let failures = std::sync::atomic::AtomicU64::new(0);
    pad.add_probe(gst::PadProbeType::BUFFER, move |_, info| {
        if let Some(gst::PadProbeData::Buffer(buffer)) = info.data.as_mut() {
            match buffer.copy_deep() {
                Ok(copy) => *buffer = copy,
                Err(error) => {
                    let failures = failures.fetch_add(1, std::sync::atomic::Ordering::Relaxed) + 1;
                    if failures <= 3 || failures % 30 == 0 {
                        log::error!("Mi Pad 2 capture-copy failed count={failures}: {error}");
                    }
                    // Retain original frame on allocation failure, never silent drop.
                }
            }
        }
        gst::PadProbeReturn::Ok
    });
}

/// Opt-in source-thread isolation is safe only after native lease copying.
pub(crate) fn decoupling_enabled(device: bool, copy: bool, opt_in: bool, factory: &str) -> bool {
    enabled(device, copy, factory) && opt_in
}

/// Eight full native frames bound memory; no leaky/drop/rate/caps policy.
/// At1080p NV12 this is about25MiB, separate from the native MMAP pool.
pub(crate) fn source_queue() -> Result<gst::Element, gst::glib::BoolError> {
    let queue = gst::ElementFactory::make("queue")
        .name("mipad2-capture-decouple")
        .property("max-size-buffers", 8u32)
        .property("max-size-bytes", 0u32)
        .property("max-size-time", 0u64)
        .build()?;
    queue.set_property_from_str("leaky", "no");
    Ok(queue)
}

/// Full copying decouples native MMAP leases from downstream allocation pools.
/// Only in that explicit device/source-scoped mode may allocation negotiation
/// stop at this boundary instead of blocking native dequeue on decodebin3's
/// serialized multiqueue allocation query. This never drops a frame or event.
pub(crate) fn allocation_guard_enabled(
    device: bool,
    copy: bool,
    opt_in: bool,
    factory: &str,
) -> bool {
    device && copy && opt_in && factory == "v4l2src"
}

pub(crate) fn source_allocation_guard() -> Result<gst::Element, gst::glib::BoolError> {
    gst::ElementFactory::make("identity")
        .name("mipad2-capture-allocation-guard")
        .property("drop-allocation", true)
        .property("signal-handoffs", false)
        .build()
}

/// Explicit source scheduling experiments; production defaults remain disabled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum QueueMode {
    Disabled,
    LegacyEight,
    GuardedTwo,
}

/// Never silently turn a malformed small-queue request into the rejected legacy
/// queue. A physical Mi Pad 2 source needs both deep copying and the allocation
/// boundary before this new buffer-push scheduling experiment is allowed.
pub(crate) fn queue_mode(
    device: bool,
    copy: bool,
    allocation_guard: bool,
    legacy_requested: bool,
    small_requested: bool,
    factory: &str,
) -> Result<QueueMode, gst::glib::BoolError> {
    if !device || factory != "v4l2src" {
        return Ok(QueueMode::Disabled);
    }
    if legacy_requested && small_requested {
        return Err(gst::glib::bool_error!(
            "Mi Pad 2 legacy and small capture queues are mutually exclusive"
        ));
    }
    if small_requested {
        if !copy || !allocation_guard {
            return Err(gst::glib::bool_error!(
                "Mi Pad 2 small capture queue requires native copy and allocation guard"
            ));
        }
        return Ok(QueueMode::GuardedTwo);
    }
    if decoupling_enabled(device, copy, legacy_requested, factory) {
        Ok(QueueMode::LegacyEight)
    } else {
        Ok(QueueMode::Disabled)
    }
}

/// Two queued copies, plus separate producer/consumer in-flight buffers. This
/// bounds this queue, not all downstream allocations or end-to-end latency.
/// Never leak frames, wait for a prefill threshold, or discard the EOS tail.
pub(crate) fn source_small_queue() -> Result<gst::Element, gst::glib::BoolError> {
    let queue = gst::ElementFactory::make("queue")
        .name("mipad2-capture-decouple-small")
        .property("max-size-buffers", 2u32)
        .property("max-size-bytes", 0u32)
        .property("max-size-time", 0u64)
        .property("min-threshold-buffers", 0u32)
        .property("min-threshold-bytes", 0u32)
        .property("min-threshold-time", 0u64)
        .property("flush-on-eos", false)
        .property("silent", true)
        .build()?;
    queue.set_property_from_str("leaky", "no");
    Ok(queue)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    struct Owned {
        data: Vec<u8>,
        released: Arc<AtomicBool>,
    }
    impl AsMut<[u8]> for Owned {
        fn as_mut(&mut self) -> &mut [u8] {
            &mut self.data
        }
    }
    impl Drop for Owned {
        fn drop(&mut self) {
            self.released.store(true, Ordering::SeqCst);
        }
    }
    #[test]
    fn scoped_opt_in_native_only() {
        assert!(enabled(true, true, "v4l2src"));
        assert!(!enabled(false, true, "v4l2src"));
        assert!(!enabled(true, false, "v4l2src"));
        assert!(!enabled(true, true, "pipewiresrc"));
    }
    #[test]
    fn full_nv12_pixels_layout_timing_and_lease_release() {
        gst::init().unwrap();
        let released = Arc::new(AtomicBool::new(false));
        let bytes: Vec<u8> = (0..4149248).map(|x| (x % 251) as u8).collect();
        let mut original = gst::Buffer::from_mut_slice(Owned {
            data: bytes.clone(),
            released: released.clone(),
        });
        let b = original.get_mut().unwrap();
        b.set_pts(gst::ClockTime::from_nseconds(123456789));
        b.set_dts(gst::ClockTime::from_nseconds(120000000));
        b.set_duration(gst::ClockTime::from_nseconds(33333333));
        b.set_offset(101);
        b.set_offset_end(102);
        b.set_flags(gst::BufferFlags::DISCONT);
        gst_video::VideoMeta::add_full(
            b,
            gst_video::VideoFrameFlags::empty(),
            gst_video::VideoFormat::Nv12,
            1920,
            1440,
            &[0, 2764800],
            &[1920, 1920],
        )
        .unwrap();
        let copy = original.copy_deep().unwrap();
        assert_eq!(copy.pts(), original.pts());
        assert_eq!(copy.dts(), original.dts());
        assert_eq!(copy.duration(), original.duration());
        assert_eq!(copy.offset(), 101);
        assert_eq!(copy.offset_end(), 102);
        assert_eq!(copy.flags(), original.flags());
        let a = original.map_readable().unwrap();
        let c = copy.map_readable().unwrap();
        assert_eq!(a.as_slice(), c.as_slice());
        assert_ne!(a.as_slice().as_ptr(), c.as_slice().as_ptr());
        drop(a);
        drop(c);
        let m = copy.meta::<gst_video::VideoMeta>().unwrap();
        assert_eq!(m.width(), 1920);
        assert_eq!(m.height(), 1440);
        assert_eq!(m.offset(), &[0, 2764800]);
        assert_eq!(m.stride(), &[1920, 1920]);
        drop(original);
        assert!(released.load(Ordering::SeqCst));
        assert_eq!(copy.map_readable().unwrap().as_slice(), bytes.as_slice());
    }
    #[test]
    fn source_queue_requires_device_copy_and_explicit_opt_in() {
        assert!(decoupling_enabled(true, true, true, "v4l2src"));
        assert!(!decoupling_enabled(false, true, true, "v4l2src"));
        assert!(!decoupling_enabled(true, false, true, "v4l2src"));
        assert!(!decoupling_enabled(true, true, false, "v4l2src"));
        assert!(!decoupling_enabled(true, true, true, "pipewiresrc"));
    }

    #[test]
    fn copied_queue_pumps_bounded_frames_while_consumer_blocks_without_loss() {
        use std::sync::{Condvar, Mutex, atomic::AtomicUsize, mpsc};
        use std::time::{Duration, Instant};
        gst::init().unwrap();
        for queued in [false, true] {
            let pipeline = gst::Pipeline::new();
            let source = gst::ElementFactory::make("appsrc")
                .property("is-live", true)
                .property_from_str("format", "time")
                .property(
                    "caps",
                    gst::Caps::from_str("video/x-raw,format=NV12,width=8,height=8,framerate=30/1")
                        .unwrap(),
                )
                .build()
                .unwrap();
            let sink = gst::ElementFactory::make("fakesink")
                .property("sync", false)
                .property("async", false)
                .property("signal-handoffs", true)
                .property("enable-last-sample", false)
                .build()
                .unwrap();
            let gate = Arc::new((Mutex::new(false), Condvar::new()));
            let gate_worker = gate.clone();
            let first = Arc::new(AtomicUsize::new(0));
            let first_worker = first.clone();
            let (entered_tx, entered_rx) = mpsc::channel();
            let (frames_tx, frames_rx) = mpsc::channel();
            sink.connect("handoff", false, move |values| {
                let frame = values[1].get::<gst::Buffer>().unwrap();
                let _ = frames_tx.send(frame);
                if first_worker.fetch_add(1, Ordering::SeqCst) == 0 {
                    let _ = entered_tx.send(());
                    let (mutex, cv) = &*gate_worker;
                    let ready = mutex.lock().unwrap();
                    let _ = cv.wait_timeout_while(ready, Duration::from_secs(5), |v| !*v);
                }
                None
            });
            install(&source);
            pipeline.add_many([&source, &sink]).unwrap();
            let queue = queued.then(|| source_queue().unwrap());
            if let Some(queue) = queue.as_ref() {
                pipeline.add(queue).unwrap();
                gst::Element::link_many([&source, queue, &sink]).unwrap();
                assert_eq!(queue.property::<u32>("max-size-buffers"), 8);
                assert_eq!(queue.property::<u32>("max-size-bytes"), 0);
                assert_eq!(queue.property::<u64>("max-size-time"), 0);
            } else {
                source.link(&sink).unwrap();
            }
            pipeline.set_state(gst::State::Playing).unwrap();
            let mut leases = Vec::new();
            for n in 0..12u64 {
                let released = Arc::new(AtomicBool::new(false));
                leases.push(released.clone());
                let mut buffer = gst::Buffer::from_mut_slice(Owned {
                    data: vec![n as u8; 96],
                    released,
                });
                let b = buffer.get_mut().unwrap();
                b.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
                b.set_offset(n);
                assert_eq!(
                    source.emit_by_name::<gst::FlowReturn>("push-buffer", &[&buffer]),
                    gst::FlowReturn::Ok
                );
            }
            assert_eq!(
                source.emit_by_name::<gst::FlowReturn>("end-of-stream", &[]),
                gst::FlowReturn::Ok
            );
            entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let deadline = Instant::now() + Duration::from_secs(2);
            if queued {
                while leases.iter().filter(|v| v.load(Ordering::SeqCst)).count() < 8
                    && Instant::now() < deadline
                {
                    std::thread::sleep(Duration::from_millis(2));
                }
            }
            let freed = leases.iter().filter(|v| v.load(Ordering::SeqCst)).count();
            // Release the bounded callback BEFORE assertions/teardown even on failure.
            {
                let (mutex, cv) = &*gate;
                *mutex.lock().unwrap() = true;
                cv.notify_all();
            }
            if queued {
                assert!(freed >= 8, "source did not decouple, freed={freed}");
            } else {
                assert!(
                    freed <= 1,
                    "negative control unexpectedly pumped, freed={freed}"
                );
            }
            for n in 0..12u64 {
                let frame = frames_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                assert_eq!(
                    frame.pts(),
                    Some(gst::ClockTime::from_nseconds(n * 33_333_333))
                );
                assert_eq!(
                    frame.map_readable().unwrap().as_slice(),
                    vec![n as u8; 96].as_slice()
                );
            }
            let message = pipeline.bus().unwrap().timed_pop_filtered(
                gst::ClockTime::from_seconds(3),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            );
            assert!(message.is_some_and(|m| m.type_() == gst::MessageType::Eos));
            pipeline.set_state(gst::State::Null).unwrap();
            assert!(leases.iter().all(|v| v.load(Ordering::SeqCst)));
        }
    }
    #[test]
    fn allocation_guard_requires_device_copy_optin_and_native_factory() {
        for device in [false, true] {
            for copy in [false, true] {
                for flag in [false, true] {
                    for factory in ["v4l2src", "appsrc", "pipewiresrc", ""] {
                        assert_eq!(
                            allocation_guard_enabled(device, copy, flag, factory),
                            device && copy && flag && factory == "v4l2src"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn allocation_guard_stops_actual_downstream_query_not_caps_or_buffers() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };
        gst::init().unwrap();
        for guarded in [false, true] {
            let source = gst::ElementFactory::make("identity").build().unwrap();
            let boundary = if guarded {
                source_allocation_guard().unwrap()
            } else {
                gst::ElementFactory::make("identity").build().unwrap()
            };
            let consumer = gst::ElementFactory::make("identity").build().unwrap();
            gst::Element::link_many([&source, &boundary, &consumer]).unwrap();
            for element in [&source, &boundary, &consumer] {
                for pad in element.pads() {
                    pad.set_active(true).unwrap();
                }
            }
            let allocations = Arc::new(AtomicUsize::new(0));
            let caps_calls = Arc::new(AtomicUsize::new(0));
            let a = allocations.clone();
            let c = caps_calls.clone();
            consumer.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::QUERY_DOWNSTREAM,
                move |_, info| {
                    if let Some(query) = info.query_mut() {
                        match query.view_mut() {
                            gst::QueryViewMut::Allocation(_) => {
                                a.fetch_add(1, Ordering::SeqCst);
                                std::thread::sleep(std::time::Duration::from_millis(60));
                                return gst::PadProbeReturn::Handled;
                            }
                            gst::QueryViewMut::Caps(query) => {
                                c.fetch_add(1, Ordering::SeqCst);
                                let caps = gst::Caps::builder("video/x-raw")
                                    .field("format", "NV12")
                                    .build();
                                query.set_result(Some(&caps));
                                return gst::PadProbeReturn::Handled;
                            }
                            _ => (),
                        }
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            let pad = source.static_pad("src").unwrap();
            let mut query = gst::query::Allocation::new(None, false);
            let result = pad.peer_query(&mut query);
            assert_eq!(allocations.load(Ordering::SeqCst), usize::from(!guarded));
            assert_eq!(result, !guarded); // Explicitly reject only allocation, not forced success.
            let caps = pad.peer_query_caps(None);
            assert!(caps_calls.load(Ordering::SeqCst) > 0);
            assert!(!caps.is_empty());
            if guarded {
                assert!(boundary.property::<bool>("drop-allocation"));
                assert_eq!(boundary.property::<f32>("drop-probability"), 0.0);
                assert!(!boundary.property::<bool>("single-segment"));
                assert!(!boundary.property::<bool>("sync"));
            }
            for element in [&source, &boundary, &consumer] {
                for pad in element.pads() {
                    pad.set_active(false).unwrap();
                }
            }
        }
    }

    #[test]
    fn actual_copy_and_allocation_guard_preserve_all_bytes_timing_offsets_and_eos() {
        use std::sync::{Arc, Mutex};
        gst::init().unwrap();
        let info = gst_video::VideoInfo::builder(gst_video::VideoFormat::Nv12, 8, 4)
            .build()
            .unwrap();
        let caps = info.to_caps().unwrap();
        let pipeline = gst::Pipeline::new();
        let source = gst::ElementFactory::make("appsrc")
            .property("caps", &caps)
            .property("format", gst::Format::Time)
            .build()
            .unwrap();
        install(&source);
        let guard = source_allocation_guard().unwrap();
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("signal-handoffs", true)
            .build()
            .unwrap();
        pipeline.add_many([&source, &guard, &sink]).unwrap();
        gst::Element::link_many([&source, &guard, &sink]).unwrap();
        let seen = Arc::new(Mutex::new(Vec::new()));
        let collected = seen.clone();
        sink.connect("handoff", false, move |args| {
            let buffer = args[1].get::<gst::Buffer>().unwrap();
            collected.lock().unwrap().push((
                buffer.offset(),
                buffer.pts(),
                buffer.duration(),
                buffer.map_readable().unwrap().as_slice().to_vec(),
            ));
            None
        });
        pipeline.set_state(gst::State::Playing).unwrap();
        for n in 0..12u64 {
            let mut buffer = gst::Buffer::with_size(info.size()).unwrap();
            let b = buffer.get_mut().unwrap();
            b.map_writable().unwrap().as_mut_slice().fill(n as u8);
            b.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
            b.set_duration(gst::ClockTime::from_nseconds(33_333_333));
            b.set_offset(n);
            assert_eq!(
                source.emit_by_name::<gst::FlowReturn>("push-buffer", &[&buffer]),
                gst::FlowReturn::Ok
            );
        }
        assert_eq!(
            source.emit_by_name::<gst::FlowReturn>("end-of-stream", &[]),
            gst::FlowReturn::Ok
        );
        let msg = pipeline
            .bus()
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(5),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            )
            .unwrap();
        assert!(matches!(msg.view(), gst::MessageView::Eos(_)), "{msg:?}");
        pipeline.set_state(gst::State::Null).unwrap();
        let frames = seen.lock().unwrap();
        assert_eq!(frames.len(), 12);
        for (n, (offset, pts, duration, bytes)) in frames.iter().enumerate() {
            assert_eq!(*offset, n as u64);
            assert_eq!(
                *pts,
                Some(gst::ClockTime::from_nseconds(n as u64 * 33_333_333))
            );
            assert_eq!(*duration, Some(gst::ClockTime::from_nseconds(33_333_333)));
            assert_eq!(bytes, &vec![n as u8; info.size()]);
        }
    }
    #[test]
    fn small_queue_configuration_is_scoped_guarded_and_never_legacy_fallback() {
        for device in [false, true] {
            for copy in [false, true] {
                for guard in [false, true] {
                    for legacy in [false, true] {
                        for small in [false, true] {
                            for factory in ["v4l2src", "appsrc", "pipewiresrc", ""] {
                                let result =
                                    queue_mode(device, copy, guard, legacy, small, factory);
                                if !device || factory != "v4l2src" {
                                    assert_eq!(result.unwrap(), QueueMode::Disabled);
                                } else if legacy && small || small && (!copy || !guard) {
                                    assert!(result.is_err());
                                } else if small {
                                    assert_eq!(result.unwrap(), QueueMode::GuardedTwo);
                                } else if legacy && copy {
                                    assert_eq!(result.unwrap(), QueueMode::LegacyEight);
                                } else {
                                    assert_eq!(result.unwrap(), QueueMode::Disabled);
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn actual_guarded_two_queue_bounds_copy_ahead_preserves_padded_frames_and_eos() {
        use std::sync::{Condvar, Mutex, atomic::AtomicUsize, mpsc};
        use std::time::{Duration, Instant};
        gst::init().unwrap();
        for queued in [false, true] {
            let pipeline = gst::Pipeline::new();
            let gate = Arc::new((Mutex::new(false), Condvar::new()));
            struct Stop(gst::Pipeline, Arc<(Mutex<bool>, Condvar)>);
            impl Drop for Stop {
                fn drop(&mut self) {
                    *self.1.0.lock().unwrap() = true;
                    self.1.1.notify_all();
                    let _ = self.0.set_state(gst::State::Null);
                }
            }
            let _stop = Stop(pipeline.clone(), gate.clone());
            let source = gst::ElementFactory::make("appsrc")
                .property("is-live", true)
                .property_from_str("format", "time")
                .property(
                    "caps",
                    gst::Caps::from_str("video/x-raw,format=NV12,width=8,height=4,framerate=30/1")
                        .unwrap(),
                )
                .build()
                .unwrap();
            install(&source);
            let pumped = Arc::new(AtomicUsize::new(0));
            let source_thread = Arc::new(Mutex::new(None));
            let count = pumped.clone();
            let thread = source_thread.clone();
            source
                .static_pad("src")
                .unwrap()
                .add_probe(gst::PadProbeType::BUFFER, move |_, _| {
                    *thread.lock().unwrap() = Some(std::thread::current().id());
                    count.fetch_add(1, Ordering::SeqCst);
                    gst::PadProbeReturn::Ok
                });
            let guard = source_allocation_guard().unwrap();
            let sink = gst::ElementFactory::make("fakesink")
                .property("sync", false)
                .property("async", false)
                .property("signal-handoffs", true)
                .property("enable-last-sample", false)
                .build()
                .unwrap();
            let allocations = Arc::new(AtomicUsize::new(0));
            let count = allocations.clone();
            sink.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::QUERY_DOWNSTREAM,
                move |_, info| {
                    if info
                        .query()
                        .is_some_and(|q| matches!(q.view(), gst::QueryView::Allocation(_)))
                    {
                        count.fetch_add(1, Ordering::SeqCst);
                        return gst::PadProbeReturn::Handled;
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            let (entered_tx, entered_rx) = mpsc::channel();
            let (frames_tx, frames_rx) = mpsc::channel();
            let first = Arc::new(AtomicBool::new(true));
            let gate_worker = gate.clone();
            sink.connect("handoff", false, move |values| {
                let frame = values[1].get::<gst::Buffer>().unwrap();
                let _ = frames_tx.send(frame);
                if first.swap(false, Ordering::SeqCst) {
                    let _ = entered_tx.send(std::thread::current().id());
                    let (mutex, cv) = &*gate_worker;
                    let ready = mutex.lock().unwrap();
                    let _ = cv.wait_timeout_while(ready, Duration::from_secs(5), |v| !*v);
                }
                None
            });
            pipeline.add_many([&source, &guard, &sink]).unwrap();
            let queue = queued.then(|| source_small_queue().unwrap());
            if let Some(q) = queue.as_ref() {
                pipeline.add(q).unwrap();
                gst::Element::link_many([&source, &guard, q, &sink]).unwrap();
                assert_eq!(q.property::<u32>("max-size-buffers"), 2);
                assert_eq!(q.property::<u32>("max-size-bytes"), 0);
                assert_eq!(q.property::<u64>("max-size-time"), 0);
                assert_eq!(q.property::<u32>("min-threshold-buffers"), 0);
                assert_eq!(q.property::<u64>("min-threshold-time"), 0);
                assert!(!q.property::<bool>("flush-on-eos"));
                assert_eq!(
                    q.property_value("leaky").serialize().unwrap().as_str(),
                    "no"
                );
            } else {
                gst::Element::link_many([&source, &guard, &sink]).unwrap();
            }
            pipeline.set_state(gst::State::Playing).unwrap();
            // The capture thread queries allocation before pushing a frame.
            // Do not demand an auxiliary thread bypass a currently held stream
            // lock: bounded nonleaky backpressure is intentional when full.
            let guard_queries = Arc::new(AtomicUsize::new(0));
            let count = guard_queries.clone();
            guard.static_pad("sink").unwrap().add_probe(
                gst::PadProbeType::QUERY_DOWNSTREAM,
                move |_, info| {
                    if info
                        .query()
                        .is_some_and(|q| matches!(q.view(), gst::QueryView::Allocation(_)))
                    {
                        count.fetch_add(1, Ordering::SeqCst);
                    }
                    gst::PadProbeReturn::Ok
                },
            );
            let before = guard_queries.load(Ordering::SeqCst);
            let mut query = gst::query::Allocation::new(None, false);
            assert!(!source.static_pad("src").unwrap().peer_query(&mut query));
            assert!(guard_queries.load(Ordering::SeqCst) > before);
            let mut leases = Vec::new();
            for n in 0..12u64 {
                let released = Arc::new(AtomicBool::new(false));
                leases.push(released.clone());
                let mut buffer = gst::Buffer::from_mut_slice(Owned {
                    data: (0..96).map(|i| (i as u8).wrapping_add(n as u8)).collect(),
                    released,
                });
                let b = buffer.get_mut().unwrap();
                b.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
                b.set_dts(gst::ClockTime::from_nseconds(n * 33_333_333));
                b.set_duration(gst::ClockTime::from_nseconds(33_333_333));
                b.set_offset(n);
                b.set_offset_end(n + 1);
                if n == 0 {
                    b.set_flags(gst::BufferFlags::DISCONT);
                }
                gst_video::VideoMeta::add_full(
                    b,
                    gst_video::VideoFrameFlags::empty(),
                    gst_video::VideoFormat::Nv12,
                    8,
                    4,
                    &[0, 64],
                    &[16, 16],
                )
                .unwrap();
                assert_eq!(
                    source.emit_by_name::<gst::FlowReturn>("push-buffer", &[&buffer]),
                    gst::FlowReturn::Ok
                );
            }
            assert_eq!(
                source.emit_by_name::<gst::FlowReturn>("end-of-stream", &[]),
                gst::FlowReturn::Ok
            );
            let consumer_thread = entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
            let expected = if queued { 4 } else { 1 }; // consumer + two queued + producer, or synchronous
            let deadline = Instant::now() + Duration::from_secs(2);
            while pumped.load(Ordering::SeqCst) < expected && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(2));
            }
            std::thread::sleep(Duration::from_millis(20));
            let pumped_blocked = pumped.load(Ordering::SeqCst);
            let freed_blocked = leases.iter().filter(|v| v.load(Ordering::SeqCst)).count();
            let level = queue
                .as_ref()
                .map(|q| q.property::<u32>("current-level-buffers"));
            let different_thread = source_thread.lock().unwrap().unwrap() != consumer_thread;
            // Unblock before assertions; Stop also guarantees teardown on panic.
            *gate.0.lock().unwrap() = true;
            gate.1.notify_all();
            assert_eq!(pumped_blocked, expected);
            assert_eq!(different_thread, queued);
            assert_eq!(allocations.load(Ordering::SeqCst), 0);
            if queued {
                assert_eq!(level, Some(2));
                assert!(
                    (3..=4).contains(&freed_blocked),
                    "native leases freed={freed_blocked}"
                );
            } else {
                assert!(freed_blocked <= 1);
            }
            for n in 0..12u64 {
                let frame = frames_rx.recv_timeout(Duration::from_secs(3)).unwrap();
                assert_eq!(frame.offset(), n);
                assert_eq!(frame.offset_end(), n + 1);
                assert_eq!(
                    frame.pts(),
                    Some(gst::ClockTime::from_nseconds(n * 33_333_333))
                );
                assert_eq!(frame.dts(), frame.pts());
                assert_eq!(
                    frame.duration(),
                    Some(gst::ClockTime::from_nseconds(33_333_333))
                );
                if n == 0 {
                    assert!(frame.flags().contains(gst::BufferFlags::DISCONT));
                }
                let expected: Vec<u8> = (0..96).map(|i| (i as u8).wrapping_add(n as u8)).collect();
                assert_eq!(
                    frame.map_readable().unwrap().as_slice(),
                    expected.as_slice()
                );
                let meta = frame.meta::<gst_video::VideoMeta>().unwrap();
                assert_eq!((meta.width(), meta.height()), (8, 4));
                assert_eq!(meta.offset(), &[0, 64]);
                assert_eq!(meta.stride(), &[16, 16]);
            }
            let msg = pipeline
                .bus()
                .unwrap()
                .timed_pop_filtered(
                    gst::ClockTime::from_seconds(3),
                    &[gst::MessageType::Eos, gst::MessageType::Error],
                )
                .unwrap();
            assert!(matches!(msg.view(), gst::MessageView::Eos(_)), "{msg:?}");
            pipeline.set_state(gst::State::Null).unwrap();
            assert!(leases.iter().all(|v| v.load(Ordering::SeqCst)));
        }
    }
    #[test]
    fn guarded_small_queue_flush_wakes_blocked_producer_and_restarts_stream() {
        use std::sync::{Condvar, Mutex, atomic::AtomicUsize, mpsc};
        use std::time::{Duration, Instant};
        gst::init().unwrap();
        let pipeline = gst::Pipeline::new();
        let gate = Arc::new((Mutex::new(false), Condvar::new()));
        struct Stop(gst::Pipeline, Arc<(Mutex<bool>, Condvar)>);
        impl Drop for Stop {
            fn drop(&mut self) {
                *self.1.0.lock().unwrap() = true;
                self.1.1.notify_all();
                let _ = self.0.set_state(gst::State::Null);
            }
        }
        let _stop = Stop(pipeline.clone(), gate.clone());
        let source = gst::ElementFactory::make("identity").build().unwrap();
        install(&source);
        let guard = source_allocation_guard().unwrap();
        let queue = source_small_queue().unwrap();
        let sink = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("async", false)
            .property("signal-handoffs", true)
            .property("enable-last-sample", false)
            .build()
            .unwrap();
        // A real flushing consumer must cancel its own blocking operation.
        // fakesink's user handoff cannot do that automatically: explicitly
        // release this test-only gate when FLUSH_START reaches its sink pad.
        let flush_observed = Arc::new(AtomicBool::new(false));
        let observed = flush_observed.clone();
        let flush_gate = gate.clone();
        sink.static_pad("sink").unwrap().add_probe(
            gst::PadProbeType::EVENT_DOWNSTREAM | gst::PadProbeType::EVENT_FLUSH,
            move |_, info| {
                if info
                    .event()
                    .is_some_and(|event| matches!(event.view(), gst::EventView::FlushStart(_)))
                {
                    observed.store(true, Ordering::SeqCst);
                    *flush_gate.0.lock().unwrap() = true;
                    flush_gate.1.notify_all();
                }
                gst::PadProbeReturn::Ok
            },
        );
        pipeline.add_many([&source, &guard, &queue, &sink]).unwrap();
        gst::Element::link_many([&source, &guard, &queue, &sink]).unwrap();
        let (entered_tx, entered_rx) = mpsc::channel();
        let (frames_tx, frames_rx) = mpsc::channel();
        let first = Arc::new(AtomicBool::new(true));
        let consumer_returned = Arc::new(AtomicBool::new(false));
        let returned = consumer_returned.clone();
        let worker_gate = gate.clone();
        sink.connect("handoff", false, move |values| {
            let frame = values[1].get::<gst::Buffer>().unwrap();
            let _ = frames_tx.send((
                frame.offset(),
                frame.map_readable().unwrap().as_slice().to_vec(),
            ));
            if first.swap(false, Ordering::SeqCst) {
                let _ = entered_tx.send(());
                let ready = worker_gate.0.lock().unwrap();
                let _ = worker_gate
                    .1
                    .wait_timeout_while(ready, Duration::from_secs(5), |v| !*v);
                returned.store(true, Ordering::SeqCst);
            }
            None
        });
        let pushed = Arc::new(AtomicUsize::new(0));
        let count = pushed.clone();
        let pad = source.static_pad("src").unwrap();
        pad.add_probe(gst::PadProbeType::BUFFER, move |_, _| {
            count.fetch_add(1, Ordering::SeqCst);
            gst::PadProbeReturn::Ok
        });
        pipeline.set_state(gst::State::Playing).unwrap();
        let caps =
            gst::Caps::from_str("video/x-raw,format=NV12,width=8,height=4,framerate=30/1").unwrap();
        assert!(pad.push_event(gst::event::StreamStart::new("small-queue-flush-before")));
        assert!(pad.push_event(gst::event::Caps::new(&caps)));
        let segment = gst::FormattedSegment::<gst::ClockTime>::new();
        assert!(pad.push_event(gst::event::Segment::new(&segment)));
        let native_released = Arc::new(AtomicUsize::new(0));
        let released = native_released.clone();
        let producer_pad = pad.clone();
        let (result_tx, result_rx) = mpsc::channel();
        let producer = std::thread::spawn(move || {
            for n in 0..12u64 {
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
                let mut buffer =
                    gst::Buffer::from_mut_slice(Lease(vec![n as u8; 48], released.clone()));
                let b = buffer.get_mut().unwrap();
                b.set_pts(gst::ClockTime::from_nseconds(n * 33_333_333));
                b.set_duration(gst::ClockTime::from_nseconds(33_333_333));
                b.set_offset(n);
                let result = producer_pad.push(buffer);
                if result.is_err() {
                    let _ = result_tx.send((n, result));
                    return;
                }
            }
            let _ = result_tx.send((12, Ok(gst::FlowSuccess::Ok)));
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(2);
        while pushed.load(Ordering::SeqCst) < 4 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        let full = queue.property::<u32>("current-level-buffers");
        // FLUSH_START must reach the responsive consumer and release the
        // full-queue producer without depending on the five-second safety timer.
        let flush_started = Instant::now();
        let flush_pad = pad.clone();
        let flusher =
            std::thread::spawn(move || flush_pad.push_event(gst::event::FlushStart::new()));
        let interrupted = result_rx.recv_timeout(Duration::from_secs(1));
        *gate.0.lock().unwrap() = true;
        gate.1.notify_all();
        producer.join().unwrap();
        assert!(flusher.join().unwrap());
        assert!(flush_started.elapsed() < Duration::from_secs(1));
        assert!(flush_observed.load(Ordering::SeqCst));
        assert_eq!(full, 2);
        let (interrupted_index, flow) = interrupted.unwrap();
        // GstQueue forwards FLUSH_START before setting srcresult=FLUSHING.
        // The cancelled consumer may free one slot in that interval, letting
        // the already-blocked push return OK before the next push observes
        // upstream flushing. Both bounded interleavings are valid; neither
        // may deliver a queued old-epoch frame or suppress Flushing.
        assert!(matches!(interrupted_index, 3 | 4));
        assert_eq!(flow, Err(gst::FlowError::Flushing));
        assert!(consumer_returned.load(Ordering::SeqCst));
        assert_eq!(
            native_released.load(Ordering::SeqCst),
            interrupted_index as usize + 1
        );
        assert!(pad.push_event(gst::event::FlushStop::new(true)));
        assert!(pad.push_event(gst::event::StreamStart::new("small-queue-flush-after")));
        assert!(pad.push_event(gst::event::Caps::new(&caps)));
        assert!(pad.push_event(gst::event::Segment::new(&segment)));
        for n in 20..32u64 {
            let mut buffer = gst::Buffer::from_mut_slice(vec![n as u8; 48]);
            let b = buffer.get_mut().unwrap();
            b.set_pts(gst::ClockTime::from_nseconds((n - 20) * 33_333_333));
            b.set_duration(gst::ClockTime::from_nseconds(33_333_333));
            b.set_offset(n);
            assert_eq!(pad.push(buffer), Ok(gst::FlowSuccess::Ok));
        }
        assert!(pad.push_event(gst::event::Eos::new()));
        let msg = pipeline
            .bus()
            .unwrap()
            .timed_pop_filtered(
                gst::ClockTime::from_seconds(3),
                &[gst::MessageType::Eos, gst::MessageType::Error],
            )
            .unwrap();
        assert!(matches!(msg.view(), gst::MessageView::Eos(_)), "{msg:?}");
        let mut after = Vec::new();
        while let Ok((offset, bytes)) = frames_rx.try_recv() {
            assert!(
                offset == 0 || offset >= 20,
                "old queued frame leaked across flush: {offset}"
            );
            if offset >= 20 {
                after.push((offset, bytes));
            }
        }
        assert_eq!(after.len(), 12);
        for (n, (offset, bytes)) in after.iter().enumerate() {
            assert_eq!(*offset, n as u64 + 20);
            assert_eq!(bytes, &vec![*offset as u8; 48]);
        }
        pipeline.set_state(gst::State::Null).unwrap();
    }
}
