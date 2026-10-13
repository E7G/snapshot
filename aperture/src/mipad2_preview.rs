// SPDX-License-Identifier: GPL-3.0-or-later
//! Mi Pad 2 native capture selection and bounded sampling of the existing preview.
use gst_video::prelude::*;
use std::{
    io,
    sync::{Arc, Mutex, mpsc},
    time::{Duration, Instant},
};

pub(crate) fn native_capture_path(
    vendor: &str,
    product: &str,
    driver: Option<&str>,
    path: Option<&str>,
) -> Option<&'static str> {
    (vendor.trim() == "Xiaomi Inc"
        && product.trim() == "Mipad2"
        && driver == Some("atomisp")
        && path == Some("/dev/video0"))
    .then_some("/dev/video0")
}

pub(crate) fn create_native_source(path: &str) -> Result<gst::Element, gtk::glib::BoolError> {
    let source = gst::ElementFactory::make("v4l2src")
        .property("device", path)
        .build()
        .map_err(|err| gtk::glib::bool_error!("Native Mi Pad 2 capture unavailable: {err}"))?;
    source.set_property_from_str("io-mode", "mmap");
    Ok(source)
}

// Experimental process-only opt-in until full GUI recording/player validation.
fn matrix_policy(vendor: &str, product: &str, h264: bool, hardware: bool, opt_in: bool) -> bool {
    vendor.trim() == "Xiaomi Inc" && product.trim() == "Mipad2" && h264 && hardware && opt_in
}

pub(crate) fn recording_matrix_requested(h264: bool, hardware: bool) -> bool {
    let vendor = std::fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default();
    let product = std::fs::read_to_string("/sys/class/dmi/id/product_name").unwrap_or_default();
    matrix_policy(
        &vendor,
        &product,
        h264,
        hardware,
        std::env::var("MIPAD2_MP4_ROTATION_MATRIX").as_deref() == Ok("1"),
    )
}

pub(crate) fn orientation_tag(rotation: u8) -> &'static str {
    match rotation % 4 {
        0 => "rotate-0",
        1 => "rotate-90",
        2 => "rotate-180",
        _ => "rotate-270",
    }
}

pub(crate) fn prefer_native_nv12(caps: gst::Caps) -> gst::Caps {
    let nv12 = gst::Caps::builder("video/x-raw")
        .field("format", "NV12")
        .build();
    let mut result = nv12.intersect_with_mode(&caps, gst::CapsIntersectMode::First);
    result.merge(caps);
    result
}

const WIDTH: usize = 320;
const HEIGHT: usize = 180;
#[derive(Debug)]
struct Request {
    id: u64,
    remaining: usize,
    sender: mpsc::SyncSender<Vec<u8>>,
}
#[derive(Debug, Default)]
struct State {
    active: bool,
    next_id: u64,
    request: Option<Request>,
}

/// A thread-safe handle for autofocus. No second camera fd or pipeline is opened.
#[derive(Clone, Debug, Default)]
pub struct PreviewFrameSampler(Arc<Mutex<State>>);
impl PreviewFrameSampler {
    pub(crate) fn start(&self) {
        let mut s = self.0.lock().unwrap();
        s.active = true;
        s.request = None;
    }
    pub(crate) fn stop(&self) {
        let mut s = self.0.lock().unwrap();
        s.active = false;
        s.request = None;
    }

    /// Obtain fresh grayscale preview frames within one overall timeout.
    /// Cancellation on stop/switch disconnects the bounded receiver.
    pub fn capture(&self, count: usize, timeout: Duration) -> io::Result<Vec<Vec<u8>>> {
        if !(1..=8).contains(&count) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "Invalid preview sample count",
            ));
        }
        let deadline = Instant::now().checked_add(timeout).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "Invalid preview timeout")
        })?;
        let (sender, receiver) = mpsc::sync_channel(count);
        let id = {
            let mut s = self.0.lock().unwrap();
            if !s.active {
                return Err(io::Error::new(
                    io::ErrorKind::NotConnected,
                    "Preview is stopped",
                ));
            }
            if s.request.is_some() {
                return Err(io::Error::new(
                    io::ErrorKind::WouldBlock,
                    "Preview sampler is busy",
                ));
            }
            s.next_id = s.next_id.wrapping_add(1);
            let id = s.next_id;
            s.request = Some(Request {
                id,
                remaining: count,
                sender,
            });
            id
        };
        let result = (|| {
            let mut frames = Vec::with_capacity(count);
            for _ in 0..count {
                let remaining = deadline.saturating_duration_since(Instant::now());
                let frame = receiver.recv_timeout(remaining).map_err(|err| {
                    io::Error::new(
                        match err {
                            mpsc::RecvTimeoutError::Timeout => io::ErrorKind::TimedOut,
                            _ => io::ErrorKind::Interrupted,
                        },
                        err,
                    )
                })?;
                frames.push(frame);
            }
            Ok(frames)
        })();
        let mut s = self.0.lock().unwrap();
        if s.request.as_ref().is_some_and(|r| r.id == id) {
            s.request = None;
        }
        result
    }
    pub(crate) fn observe(&self, buffer: &gst::BufferRef, caps: &gst::CapsRef) {
        let id = {
            let Ok(s) = self.0.try_lock() else {
                return;
            };
            let Some(r) = s.request.as_ref() else {
                return;
            };
            r.id
        };
        let Some(frame) = gray_frame(buffer, caps) else {
            return;
        };
        let Ok(mut s) = self.0.try_lock() else {
            return;
        };
        let Some(r) = s.request.as_mut().filter(|r| r.id == id) else {
            return;
        };
        if r.sender.try_send(frame).is_ok() {
            r.remaining -= 1;
            if r.remaining == 0 {
                s.request = None;
            }
        } else {
            s.request = None;
        }
    }
}

fn gray_frame(buffer: &gst::BufferRef, caps: &gst::CapsRef) -> Option<Vec<u8>> {
    if buffer.flags().intersects(
        gst::BufferFlags::CORRUPTED | gst::BufferFlags::DECODE_ONLY | gst::BufferFlags::GAP,
    ) {
        return None;
    }
    let info = gst_video::VideoInfo::from_caps(caps).ok()?;
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info).ok()?;
    let data = frame.plane_data(0).ok()?;
    let stride = usize::try_from(frame.plane_stride()[0]).ok()?;
    use gst_video::VideoFormat as F;
    let (pixel_stride, rgb) = match info.format() {
        F::I420 | F::Yv12 | F::Nv12 | F::Nv21 | F::Gray8 => (1, None),
        F::Yuy2 => (2, None),
        F::Rgbx | F::Rgba => (4, Some([0, 1, 2])),
        F::Bgrx | F::Bgra => (4, Some([2, 1, 0])),
        F::Xbgr | F::Abgr => (4, Some([3, 2, 1])),
        F::Xrgb | F::Argb => (4, Some([1, 2, 3])),
        _ => return None,
    };
    let w = info.width() as usize;
    let h = info.height() as usize;
    if w == 0 || h == 0 {
        return None;
    }
    let channel = rgb.map_or(0, |c| *c.iter().max().unwrap());
    let row_end = (w - 1)
        .checked_mul(pixel_stride)?
        .checked_add(channel + 1)?;
    if stride < row_end || (h - 1).checked_mul(stride)?.checked_add(row_end)? > data.len() {
        return None;
    }
    let limited =
        rgb.is_none() && info.colorimetry().range() != gst_video::VideoColorRange::Range0_255;
    let mut output = Vec::with_capacity(WIDTH * HEIGHT);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let at = (y * h / HEIGHT) * stride + (x * w / WIDTH) * pixel_stride;
            let v = match rgb {
                Some([r, g, b]) => {
                    0.2126 * data[at + r] as f64
                        + 0.7152 * data[at + g] as f64
                        + 0.0722 * data[at + b] as f64
                }
                None => data[at] as f64,
            };
            output.push(if limited {
                ((v - 16.) * 255. / 219.).clamp(0., 255.).round() as u8
            } else {
                v.round() as u8
            });
        }
    }
    Some(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn recording_matrix_is_scoped_and_opt_in() {
        for vendor in ["Xiaomi Inc", "Xiaomi Inc\n", "Other"] {
            for product in ["Mipad2", "Mipad2\n", "Other"] {
                for h264 in [false, true] {
                    for hw in [false, true] {
                        for enabled in [false, true] {
                            assert_eq!(
                                matrix_policy(vendor, product, h264, hw, enabled),
                                vendor.trim() == "Xiaomi Inc"
                                    && product.trim() == "Mipad2"
                                    && h264
                                    && hw
                                    && enabled
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn all_orientation_tags_and_wraps() {
        for i in 0..=u8::MAX {
            assert_eq!(
                orientation_tag(i),
                ["rotate-0", "rotate-90", "rotate-180", "rotate-270"][usize::from(i % 4)]
            );
        }
    }

    #[test]
    fn nv12_preference_preserves_geometry_rate_and_fallback() {
        gst::init().unwrap();
        let original: gst::Caps = "video/x-raw,format=I420,width=1920,height=1440,framerate=30/1;video/x-raw,format=NV12,width=1920,height=1440,framerate=30/1".parse().unwrap();
        let sorted = prefer_native_nv12(original.clone());
        assert!(original.is_subset(&sorted) && sorted.is_subset(&original));
        let first = sorted.structure(0).unwrap();
        assert_eq!(first.get::<&str>("format").unwrap(), "NV12");
        assert_eq!(first.get::<i32>("width").unwrap(), 1920);
        assert_eq!(first.get::<i32>("height").unwrap(), 1440);
        assert_eq!(
            first.get::<gst::Fraction>("framerate").unwrap(),
            gst::Fraction::new(30, 1)
        );
        let fallback: gst::Caps = "video/x-raw,format=I420,width=1920,height=1440,framerate=30/1"
            .parse()
            .unwrap();
        assert_eq!(prefer_native_nv12(fallback.clone()), fallback);
    }

    #[test]
    fn policy_is_device_specific() {
        assert_eq!(
            native_capture_path(
                "Xiaomi Inc\n",
                "Mipad2\n",
                Some("atomisp"),
                Some("/dev/video0")
            ),
            Some("/dev/video0")
        );
        for (v, p, d, path) in [
            ("Other", "Mipad2", Some("atomisp"), Some("/dev/video0")),
            ("Xiaomi Inc", "Other", Some("atomisp"), Some("/dev/video0")),
            (
                "Xiaomi Inc",
                "Mipad2",
                Some("uvcvideo"),
                Some("/dev/video0"),
            ),
            ("Xiaomi Inc", "Mipad2", Some("atomisp"), Some("/dev/video1")),
            ("Xiaomi Inc", "Mipad2", None, Some("/dev/video0")),
        ] {
            assert_eq!(native_capture_path(v, p, d, path), None);
        }
    }
    #[test]
    fn native_source_uses_mmap() {
        gst::init().unwrap();
        let s = create_native_source("/dev/video0").unwrap();
        assert_eq!(s.factory().unwrap().name(), "v4l2src");
        assert_eq!(s.property::<String>("device"), "/dev/video0");
        assert_eq!(
            s.property_value("io-mode")
                .transform::<i32>()
                .unwrap()
                .get::<i32>()
                .unwrap(),
            2
        );
    }
    #[test]
    fn samples_existing_frames_and_cancels_on_stop() {
        gst::init().unwrap();
        let info = gst_video::VideoInfo::builder(gst_video::VideoFormat::Gray8, 17, 11)
            .build()
            .unwrap();
        let caps = info.to_caps().unwrap();
        let b = gst::Buffer::from_mut_slice(vec![128u8; info.size()]);
        let gray = gray_frame(&b, &caps).unwrap();
        assert_eq!(gray.len(), WIDTH * HEIGHT);
        let sampler = PreviewFrameSampler::default();
        sampler.start();
        let s = sampler.clone();
        let t = std::thread::spawn(move || s.capture(3, Duration::from_secs(1)));
        for _ in 0..100 {
            if sampler.0.lock().unwrap().request.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        for _ in 0..3 {
            sampler.observe(&b, &caps);
        }
        assert_eq!(t.join().unwrap().unwrap().len(), 3);
        let s = sampler.clone();
        let t = std::thread::spawn(move || s.capture(1, Duration::from_secs(1)));
        for _ in 0..100 {
            if sampler.0.lock().unwrap().request.is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(1));
        }
        sampler.stop();
        assert_eq!(
            t.join().unwrap().unwrap_err().kind(),
            io::ErrorKind::Interrupted
        );
        assert_eq!(
            sampler.capture(1, Duration::ZERO).unwrap_err().kind(),
            io::ErrorKind::NotConnected
        );
    }
    #[test]
    fn timeout_and_corrupt_buffers_do_not_poison_next_request() {
        gst::init().unwrap();
        let s = PreviewFrameSampler::default();
        s.start();
        assert_eq!(
            s.capture(1, Duration::from_millis(1)).unwrap_err().kind(),
            io::ErrorKind::TimedOut
        );
        assert!(s.0.lock().unwrap().request.is_none());
        let info = gst_video::VideoInfo::builder(gst_video::VideoFormat::Gray8, 8, 8)
            .build()
            .unwrap();
        let mut b = gst::Buffer::from_mut_slice(vec![128u8; info.size()]);
        b.get_mut().unwrap().set_flags(gst::BufferFlags::CORRUPTED);
        assert!(gray_frame(&b, &info.to_caps().unwrap()).is_none());
        assert_eq!(
            s.capture(0, Duration::ZERO).unwrap_err().kind(),
            io::ErrorKind::InvalidInput
        );
    }
}
