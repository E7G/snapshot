// SPDX-License-Identifier: GPL-3.0-or-later
//! Mi Pad 2 preview AE bridge. Only the app owning the preview runs this loop.
//! No second capture fd, external gst-launch process, or global background daemon.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong};
use std::sync::{Arc, Mutex, mpsc};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crate::mipad2_ae_algorithm::{self as algorithm, Range, Stats, Update};
use gst_video::VideoFrameExt;

const EXPOSURE: u32 = 0x00980911;
const GAIN: u32 = 0x009e0903;
const SAMPLE_INTERVAL: Duration = Duration::from_millis(100);
const SETTLE: Duration = Duration::from_millis(400);
const WARMUP: Duration = Duration::from_millis(800);

#[repr(C)]
#[derive(Default)]
struct Control {
    id: u32,
    value: i32,
}
#[repr(C)]
#[derive(Default)]
struct QueryControl {
    id: u32,
    kind: u32,
    name: [u8; 32],
    min: i32,
    max: i32,
    step: i32,
    default_value: i32,
    flags: u32,
    reserved: [u32; 2],
}
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}

fn request<T>(number: u32) -> c_ulong {
    ((3u32 << 30) | ((std::mem::size_of::<T>() as u32) << 16) | (u32::from(b'V') << 8) | number)
        as c_ulong
}
fn control_ioctl<T>(file: &File, number: u32, value: &mut T) -> io::Result<()> {
    // SAFETY: request encodes this exact initialized repr(C) structure, and
    // the file remains open for the whole synchronous ioctl. Target is x86 Linux.
    if unsafe { ioctl(file.as_raw_fd(), request::<T>(number), value as *mut T) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

struct Sensor {
    file: File,
    name: String,
}
impl Sensor {
    fn open(front: bool) -> io::Result<Self> {
        let prefix = if front { "ov5693 " } else { "t4ka3 " };
        let mut found = Vec::new();
        for entry in fs::read_dir("/sys/class/video4linux")? {
            let path = entry?.path();
            if !path
                .file_name()
                .is_some_and(|n| n.to_string_lossy().starts_with("v4l-subdev"))
            {
                continue;
            }
            if let Ok(name) = fs::read_to_string(path.join("name"))
                && name.starts_with(prefix)
            {
                found.push((path.file_name().unwrap().to_owned(), name.trim().to_owned()));
            }
        }
        if found.len() != 1 {
            return Err(io::Error::other("sensor identity not unique"));
        }
        let (node, name) = found.pop().unwrap();
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(std::path::Path::new("/dev").join(node))?;
        Ok(Self { file, name })
    }
    fn range(&self, id: u32) -> io::Result<Range> {
        let mut query = QueryControl {
            id,
            ..Default::default()
        };
        control_ioctl(&self.file, 36, &mut query)?;
        let range = Range {
            min: query.min,
            max: query.max,
            step: query.step,
        };
        if query.kind != 1 || query.flags & (1 | 4 | 16) != 0 || !range.valid() {
            return Err(io::Error::other(
                "control not a usable nonnegative integer range",
            ));
        }
        Ok(range)
    }
    fn get(&self, id: u32) -> io::Result<i32> {
        let mut ctrl = Control { id, value: 0 };
        control_ioctl(&self.file, 27, &mut ctrl)?;
        Ok(ctrl.value)
    }
    fn set(&self, id: u32, value: i32) -> io::Result<()> {
        let mut ctrl = Control { id, value };
        control_ioctl(&self.file, 28, &mut ctrl)?;
        if ctrl.value != value || self.get(id)? != value {
            return Err(io::Error::other("control readback mismatch"));
        }
        Ok(())
    }
    fn update(&self, stats: Stats) -> io::Result<()> {
        let er = self.range(EXPOSURE)?;
        let gr = self.range(GAIN)?;
        let exposure = self.get(EXPOSURE)?;
        let gain = self.get(GAIN)?;
        if let Some(update) = algorithm::next(stats, exposure, gain, er, gr) {
            let (id, value) = match update {
                Update::Exposure(v) => (EXPOSURE, v),
                Update::Gain(v) => (GAIN, v),
            };
            self.set(id, value)?;
            log::info!(
                "Mi Pad 2 AE {} Y={:.1} clipped={:.3} exposure={} gain={} -> {:?}",
                self.name,
                stats.mean,
                stats.clipped,
                exposure,
                gain,
                update
            );
        } else {
            log::debug!(
                "Mi Pad 2 AE {} steady Y={:.1} exposure={} gain={}",
                self.name,
                stats.mean,
                exposure,
                gain
            );
        }
        Ok(())
    }
}

fn frame_stats(buffer: &gst::BufferRef, caps: &gst::CapsRef) -> Option<Stats> {
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
        F::Bgrx | F::Bgra => (4, Some([2, 1, 0])),
        F::Rgbx | F::Rgba => (4, Some([0, 1, 2])),
        F::Xbgr | F::Abgr => (4, Some([3, 2, 1])),
        F::Xrgb | F::Argb => (4, Some([1, 2, 3])),
        F::Yuy2 => (2, None),
        _ => return None,
    };
    let limited =
        rgb.is_none() && info.colorimetry().range() != gst_video::VideoColorRange::Range0_255;
    algorithm::meter(
        data,
        info.width() as usize,
        info.height() as usize,
        stride,
        pixel_stride,
        rgb,
        limited,
    )
}

#[derive(Debug)]
struct Session {
    alive: bool,
    active: bool,
    front: bool,
    epoch: u64,
    holds: usize,
    since: Instant,
    last_sample: Option<Instant>,
    unsupported_warned: bool,
}
impl Session {
    fn accepts(&self, epoch: u64, observed: Instant) -> bool {
        self.alive
            && self.active
            && self.holds == 0
            && self.epoch == epoch
            && observed.elapsed() <= Duration::from_millis(200)
            && self.since.elapsed() >= WARMUP
    }
}
#[derive(Debug)]
struct Packet {
    epoch: u64,
    observed: Instant,
    stats: Stats,
}
#[derive(Debug)]
struct Runtime {
    session: Arc<Mutex<Session>>,
    sender: mpsc::SyncSender<Packet>,
    worker: Mutex<Option<JoinHandle<()>>>,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        self.session.lock().unwrap().alive = false;
        if let Some(worker) = self.worker.get_mut().unwrap().take() {
            let _ = worker.join();
        }
    }
}
#[derive(Clone, Debug)]
pub(crate) struct AutoExposure(Arc<Runtime>);

/// Freezes AE while a rear autofocus sweep compares frame contrast.
#[derive(Debug)]
pub struct ExposureHold {
    runtime: Arc<Runtime>,
    epoch: u64,
}
impl Drop for ExposureHold {
    fn drop(&mut self) {
        let mut s = self.runtime.session.lock().unwrap();
        if s.epoch == self.epoch {
            s.holds = s.holds.saturating_sub(1);
            s.since = Instant::now(); // discard frames taken during the sweep
        }
    }
}

impl AutoExposure {
    pub fn new() -> io::Result<Self> {
        let session = Arc::new(Mutex::new(Session {
            alive: true,
            active: false,
            front: true,
            epoch: 0,
            holds: 0,
            since: Instant::now(),
            last_sample: None,
            unsupported_warned: false,
        }));
        let (sender, receiver) = mpsc::sync_channel::<Packet>(1);
        let state = session.clone();
        let worker = thread::Builder::new()
            .name("mipad2-ae".into())
            .spawn(move || {
                let mut sensor: Option<Sensor> = None;
                let mut epoch = u64::MAX;
                let mut last_update = Instant::now();
                loop {
                    let packet = receiver.recv_timeout(SAMPLE_INTERVAL);
                    let mut s = state.lock().unwrap();
                    if !s.alive {
                        break;
                    }
                    if !s.active || s.epoch != epoch {
                        sensor = None;
                    }
                    let Ok(packet) = packet else {
                        continue;
                    };
                    if !s.accepts(packet.epoch, packet.observed) {
                        continue;
                    }
                    if sensor.is_none() {
                        epoch = s.epoch;
                        match Sensor::open(s.front) {
                            Ok(value) => {
                                log::info!("Mi Pad 2 AE active for {}", value.name);
                                sensor = Some(value);
                            }
                            Err(err) => {
                                log::warn!("Mi Pad 2 AE disabled: {err}");
                                s.active = false;
                                continue;
                            }
                        }
                        last_update = Instant::now() - SETTLE;
                    }
                    if last_update.elapsed() < SETTLE {
                        continue;
                    }
                    // Keep the session lock through control writes: stop/switch is
                    // serialized with this operation, never a stale-sensor write.
                    if let Err(err) = sensor.as_ref().unwrap().update(packet.stats) {
                        log::warn!("Mi Pad 2 AE disabled after control failure: {err}");
                        s.active = false;
                        sensor = None;
                    }
                    last_update = Instant::now();
                }
            })?;
        Ok(Self(Arc::new(Runtime {
            session,
            sender,
            worker: Mutex::new(Some(worker)),
        })))
    }
    pub fn pause(&self) {
        let mut s = self.0.session.lock().unwrap();
        s.active = false;
        s.epoch = s.epoch.wrapping_add(1);
        s.holds = 0;
    }
    pub fn start(&self, front: bool) {
        let mut s = self.0.session.lock().unwrap();
        s.active = true;
        s.front = front;
        s.epoch = s.epoch.wrapping_add(1);
        s.holds = 0;
        s.since = Instant::now();
        s.last_sample = None;
        s.unsupported_warned = false;
    }
    pub fn hold(&self) -> ExposureHold {
        let mut s = self.0.session.lock().unwrap();
        s.holds += 1;
        ExposureHold {
            runtime: self.0.clone(),
            epoch: s.epoch,
        }
    }
    pub fn observe(&self, buffer: &gst::BufferRef, caps: &gst::CapsRef) {
        let (epoch, observed) = {
            let Ok(mut s) = self.0.session.try_lock() else {
                return;
            };
            if !s.active
                || s.holds > 0
                || s.last_sample.is_some_and(|t| t.elapsed() < SAMPLE_INTERVAL)
            {
                return;
            }
            let now = Instant::now();
            s.last_sample = Some(now);
            (s.epoch, now)
        };
        let stats = frame_stats(buffer, caps);
        if let Some(stats) = stats {
            // A bounded mailbox drops frames instead of blocking the preview.
            let _ = self.0.sender.try_send(Packet {
                epoch,
                observed,
                stats,
            });
        } else {
            let mut s = self.0.session.lock().unwrap();
            if !s.unsupported_warned {
                log::warn!("Mi Pad 2 AE cannot meter negotiated buffer/caps: {caps}");
                s.unsupported_warned = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn v4l2_abi() {
        assert_eq!(std::mem::size_of::<Control>(), 8);
        assert_eq!(std::mem::size_of::<QueryControl>(), 68);
        assert_eq!(request::<Control>(27), 0xc008561b);
        assert_eq!(request::<Control>(28), 0xc008561c);
        assert_eq!(request::<QueryControl>(36), 0xc0445624);
    }
    #[test]
    fn stale_paused_held_and_delayed_frames_rejected() {
        let mut s = Session {
            alive: true,
            active: true,
            front: true,
            epoch: 3,
            holds: 0,
            since: Instant::now() - WARMUP,
            last_sample: None,
            unsupported_warned: false,
        };
        assert!(s.accepts(3, Instant::now()));
        assert!(!s.accepts(2, Instant::now()));
        assert!(!s.accepts(3, Instant::now() - Duration::from_secs(1)));
        s.holds = 1;
        assert!(!s.accepts(3, Instant::now()));
        s.holds = 0;
        s.active = false;
        assert!(!s.accepts(3, Instant::now()));
        s.active = true;
        s.since = Instant::now();
        assert!(!s.accepts(3, Instant::now()));
    }
    #[test]
    fn old_hold_does_not_change_new_session() {
        let ae = AutoExposure::new().unwrap();
        ae.start(true);
        let hold = ae.hold();
        assert_eq!(ae.0.session.lock().unwrap().holds, 1);
        ae.pause();
        assert!(!ae.0.session.lock().unwrap().active);
        ae.start(false);
        let epoch = ae.0.session.lock().unwrap().epoch;
        drop(hold);
        let s = ae.0.session.lock().unwrap();
        assert_eq!(s.epoch, epoch);
        assert!(s.active && !s.front && s.holds == 0);
    }
    #[test]
    fn bounded_mailbox_drops_instead_of_blocking() {
        let (sender, _receiver) = mpsc::sync_channel(1);
        let packet = || Packet {
            epoch: 0,
            observed: Instant::now(),
            stats: Stats {
                mean: 120.0,
                clipped: 0.0,
            },
        };
        assert!(sender.try_send(packet()).is_ok());
        assert!(matches!(
            sender.try_send(packet()),
            Err(mpsc::TrySendError::Full(_))
        ));
    }
    #[test]
    fn actual_gstreamer_frame_mapping() {
        gst::init().unwrap();
        let color = "bt709".parse::<gst_video::VideoColorimetry>().unwrap();
        let info = gst_video::VideoInfo::builder(gst_video::VideoFormat::I420, 4, 4)
            .colorimetry(&color)
            .build()
            .unwrap();
        let caps = info.to_caps().unwrap();
        let mut buffer = gst::Buffer::with_size(info.size()).unwrap();
        buffer
            .get_mut()
            .unwrap()
            .map_writable()
            .unwrap()
            .as_mut_slice()
            .fill(126);
        let s = frame_stats(&buffer, &caps).unwrap();
        assert!((s.mean - 128.08).abs() < 0.1);
        assert_eq!(s.clipped, 0.0);
        buffer
            .get_mut()
            .unwrap()
            .set_flags(gst::BufferFlags::CORRUPTED);
        assert!(frame_stats(&buffer, &caps).is_none());
        let info = gst_video::VideoInfo::builder(gst_video::VideoFormat::Bgrx, 2, 2)
            .build()
            .unwrap();
        let caps = info.to_caps().unwrap();
        let mut buffer = gst::Buffer::with_size(info.size()).unwrap();
        buffer
            .get_mut()
            .unwrap()
            .map_writable()
            .unwrap()
            .as_mut_slice()
            .fill(255);
        let s = frame_stats(&buffer, &caps).unwrap();
        assert!((s.mean - 255.0).abs() < 0.01);
        assert_eq!(s.clipped, 1.0);
        let buffer = gst::Buffer::with_size(1).unwrap();
        assert!(frame_stats(&buffer, &caps).is_none());
    }
}
