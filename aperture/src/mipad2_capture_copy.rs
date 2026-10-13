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

#[cfg(test)]
mod tests {
    use super::*;
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
}
