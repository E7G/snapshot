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
}
