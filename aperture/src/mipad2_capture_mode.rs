// SPDX-License-Identifier: GPL-3.0-or-later
//! A shot-only ISP control lease, acquired only after the owner pipeline is NULL.
//! No second video capture owner and no external process or global settings.
use std::fs::{self, File, OpenOptions};
use std::io;
use std::os::fd::AsRawFd;
use std::os::raw::{c_int, c_ulong};

const RUN_MODE: u32 = 0x009a0d14;
const VFPP: u32 = 0x009a0d19;
const STILL: i32 = 2;
#[repr(C)]
#[derive(Default)]
struct Control {
    id: u32,
    value: i32,
}
#[repr(C)]
#[derive(Default)]
struct Query {
    id: u32,
    kind: u32,
    name: [u8; 32],
    min: i32,
    max: i32,
    step: i32,
    default: i32,
    flags: u32,
    reserved: [u32; 2],
}
unsafe extern "C" {
    fn ioctl(fd: c_int, request: c_ulong, ...) -> c_int;
}
fn request<T>(n: u32) -> c_ulong {
    ((3u32 << 30) | ((std::mem::size_of::<T>() as u32) << 16) | (u32::from(b'V') << 8) | n)
        as c_ulong
}
fn call<T>(f: &File, n: u32, arg: &mut T) -> io::Result<()> {
    // SAFETY: initialized repr(C) structure of the encoded exact size; this
    // live fd is held throughout the synchronous x86 Linux V4L2 ioctl.
    if unsafe { ioctl(f.as_raw_fd(), request::<T>(n), arg as *mut T) } < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}
fn read_control(f: &File, id: u32) -> io::Result<i32> {
    let mut c = Control { id, value: 0 };
    call(f, 27, &mut c)?;
    Ok(c.value)
}
fn write_control(f: &File, id: u32, value: i32) -> io::Result<()> {
    let mut c = Control { id, value };
    call(f, 28, &mut c)?;
    if c.value != value {
        return Err(io::Error::other("ISP control write was clamped"));
    }
    Ok(())
}
fn qualified(q: &Query) -> bool {
    q.id == RUN_MODE
        && q.kind == 3
        && q.min == 1
        && q.max == 3
        && q.default == 3
        && q.flags & 4 == 0
        && q.name.split(|c| *c == 0).next() == Some(b"Atomisp run mode".as_slice())
}
trait ModeIo: std::fmt::Debug {
    fn read(&mut self) -> io::Result<i32>;
    fn write(&mut self, value: i32) -> io::Result<()>;
    fn read_vfpp(&mut self) -> io::Result<i32>;
    fn write_vfpp(&mut self, value: i32) -> io::Result<()>;
}
#[derive(Debug)]
struct FileMode(File);
impl ModeIo for FileMode {
    fn read(&mut self) -> io::Result<i32> {
        read_control(&self.0, RUN_MODE)
    }
    fn write(&mut self, v: i32) -> io::Result<()> {
        write_control(&self.0, RUN_MODE, v)
    }
    fn read_vfpp(&mut self) -> io::Result<i32> {
        read_control(&self.0, VFPP)
    }
    fn write_vfpp(&mut self, v: i32) -> io::Result<()> {
        write_control(&self.0, VFPP, v)
    }
}
#[derive(Debug)]
struct Lease<T: ModeIo> {
    io: T,
    previous: i32,
    previous_vfpp: i32,
    cleanup_pending: bool,
}
impl<T: ModeIo> Lease<T> {
    fn enter(mut io: T) -> io::Result<Self> {
        let previous = io.read()?;
        if !(1..=3).contains(&previous) {
            return Err(io::Error::other("Unknown original ISP run mode"));
        }
        let previous_vfpp = io.read_vfpp()?;
        if !(0..=2).contains(&previous_vfpp) {
            return Err(io::Error::other("Unknown original ISP VFPP"));
        }
        let mut lease = Self {
            io,
            previous,
            previous_vfpp,
            cleanup_pending: true,
        };
        lease.io.write(STILL)?;
        if lease.io.read()? != STILL {
            return Err(io::Error::other("ISP did not enter still mode"));
        }
        // Single video node consumes main frames, not the separate CSS VF output.
        lease.io.write_vfpp(2)?;
        if lease.io.read_vfpp()? != 2 {
            return Err(io::Error::other(
                "ISP did not enter single-output capture configuration",
            ));
        }
        Ok(lease)
    }
    fn restore(&mut self) -> io::Result<()> {
        // An explicit failed restore must be reported, not hidden by Drop retry.
        self.cleanup_pending = false;
        let vfpp = self.io.write_vfpp(self.previous_vfpp).and_then(|_| {
            if self.io.read_vfpp()? == self.previous_vfpp {
                Ok(())
            } else {
                Err(io::Error::other("Original ISP VFPP not restored"))
            }
        });
        // Try both restorations even if the first fails; return a real error.
        let mode = self.io.write(self.previous).and_then(|_| {
            if self.io.read()? == self.previous {
                Ok(())
            } else {
                Err(io::Error::other("Original ISP run mode not restored"))
            }
        });
        vfpp.and(mode)
    }
}
impl<T: ModeIo> Drop for Lease<T> {
    fn drop(&mut self) {
        if self.cleanup_pending
            && let Err(error) = self.restore()
        {
            log::error!("Mi Pad 2 ISP mode lease cleanup failed: {error}");
        }
    }
}
#[derive(Debug)]
pub(crate) struct ModeGuard(Lease<FileMode>);
impl ModeGuard {
    pub(crate) fn acquire() -> io::Result<Self> {
        for entry in fs::read_dir("/sys/class/video4linux")? {
            let path = entry?.path();
            let Some(name) = path
                .file_name()
                .filter(|n| n.to_string_lossy().starts_with("v4l-subdev"))
            else {
                continue;
            };
            let Ok(device) = fs::canonicalize(path.join("device")) else {
                continue;
            };
            if !device.components().any(|c| c.as_os_str() == "0000:00:03.0") {
                continue;
            }
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .open(std::path::Path::new("/dev").join(name))?;
            let mut q = Query {
                id: RUN_MODE,
                ..Default::default()
            };
            if call(&f, 36, &mut q).is_err() || !qualified(&q) {
                continue;
            }
            let mut vfpp = Query {
                id: VFPP,
                ..Default::default()
            };
            call(&f, 36, &mut vfpp)?;
            if vfpp.kind != 3 || vfpp.min != 0 || vfpp.max != 2 || vfpp.flags & 4 != 0 {
                return Err(io::Error::other("Unexpected ISP VFPP control identity"));
            }
            let mut lease = Lease::enter(FileMode(f))?;
            // From here the pipeline owner explicitly restores only after NULL.
            // Drop must not restore while a failed shutdown leaves it active.
            lease.cleanup_pending = false;
            log::info!(
                "Mi Pad 2 ISP still mode/VFPP2 entered; saved original mode={} VFPP={}",
                lease.previous,
                lease.previous_vfpp
            );
            return Ok(Self(lease));
        }
        Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Qualified AtomISP run-mode subdevice not found",
        ))
    }
    pub(crate) fn restore(mut self) -> io::Result<()> {
        self.0.restore()?;
        log::info!(
            "Mi Pad 2 original ISP mode restored={} VFPP={}",
            self.0.previous,
            self.0.previous_vfpp
        );
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;
    #[derive(Debug)]
    struct State {
        mode: i32,
        vfpp: i32,
        writes: Vec<(u32, i32)>,
        fail_write: bool,
        fail_read: bool,
        bad_readback: bool,
        fail_vfpp_write: bool,
    }
    #[derive(Debug, Clone)]
    struct Mock(Rc<RefCell<State>>);
    impl ModeIo for Mock {
        fn read(&mut self) -> io::Result<i32> {
            let mut s = self.0.borrow_mut();
            if s.fail_read {
                s.fail_read = false;
                return Err(io::Error::other("read failure"));
            }
            if s.bad_readback && !s.writes.is_empty() {
                s.bad_readback = false;
                return Ok(99);
            }
            Ok(s.mode)
        }
        fn write(&mut self, v: i32) -> io::Result<()> {
            let mut s = self.0.borrow_mut();
            s.writes.push((RUN_MODE, v));
            if s.fail_write {
                s.fail_write = false;
                return Err(io::Error::other("write failure"));
            }
            s.mode = v;
            Ok(())
        }
        fn read_vfpp(&mut self) -> io::Result<i32> {
            Ok(self.0.borrow().vfpp)
        }
        fn write_vfpp(&mut self, v: i32) -> io::Result<()> {
            let mut s = self.0.borrow_mut();
            s.writes.push((VFPP, v));
            if s.fail_vfpp_write {
                s.fail_vfpp_write = false;
                return Err(io::Error::other("VFPP write failure"));
            }
            s.vfpp = v;
            Ok(())
        }
    }
    fn mock(mode: i32, vfpp: i32) -> Mock {
        Mock(Rc::new(RefCell::new(State {
            mode,
            vfpp,
            writes: vec![],
            fail_write: false,
            fail_read: false,
            bad_readback: false,
            fail_vfpp_write: false,
        })))
    }
    #[test]
    fn exact_ioctl_and_identity_abi() {
        assert_eq!(std::mem::size_of::<Control>(), 8);
        assert_eq!(std::mem::size_of::<Query>(), 68);
        assert_eq!(request::<Control>(27), 0xc008561b);
        assert_eq!(request::<Control>(28), 0xc008561c);
        assert_eq!(request::<Query>(36), 0xc0445624);
        let mut q = Query {
            id: RUN_MODE,
            kind: 3,
            min: 1,
            max: 3,
            default: 3,
            ..Default::default()
        };
        q.name[..16].copy_from_slice(b"Atomisp run mode");
        assert!(qualified(&q));
        q.flags = 4;
        assert!(!qualified(&q));
        q.flags = 0;
        q.kind = 1;
        assert!(!qualified(&q));
    }
    #[test]
    fn exact_original_mode_and_vfpp_restored_on_explicit_or_drop_exit() {
        for mode in 1..=3 {
            for vfpp in 0..=2 {
                for explicit in [true, false] {
                    let m = mock(mode, vfpp);
                    let mut lease = Lease::enter(m.clone()).unwrap();
                    assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (2, 2));
                    if explicit {
                        lease.restore().unwrap();
                    }
                    drop(lease);
                    let s = m.0.borrow();
                    assert_eq!((s.mode, s.vfpp), (mode, vfpp));
                    assert_eq!(
                        s.writes,
                        vec![(RUN_MODE, 2), (VFPP, 2), (VFPP, vfpp), (RUN_MODE, mode)]
                    );
                }
            }
        }
    }
    #[test]
    fn failed_entry_rolls_back_both_original_controls() {
        for mode in 1..=3 {
            for vfpp in 0..=2 {
                let m = mock(mode, vfpp);
                m.0.borrow_mut().fail_write = true;
                assert!(Lease::enter(m.clone()).is_err());
                assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (mode, vfpp));
                let m = mock(mode, vfpp);
                m.0.borrow_mut().fail_vfpp_write = true;
                assert!(Lease::enter(m.clone()).is_err());
                assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (mode, vfpp));
                let m = mock(mode, vfpp);
                m.0.borrow_mut().bad_readback = true;
                assert!(Lease::enter(m.clone()).is_err());
                assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (mode, vfpp));
                let m = mock(mode, vfpp);
                m.0.borrow_mut().fail_read = true;
                assert!(Lease::enter(m.clone()).is_err());
                assert!(m.0.borrow().writes.is_empty());
            }
        }
        for (mode, vfpp) in [(99, 0), (3, 99)] {
            let m = mock(mode, vfpp);
            assert!(Lease::enter(m.clone()).is_err());
            assert!(m.0.borrow().writes.is_empty());
        }
    }
    #[test]
    fn failed_restore_attempts_both_controls_and_does_not_hide_or_retry_error() {
        for fail_vfpp in [false, true] {
            let m = mock(3, 0);
            let mut lease = Lease::enter(m.clone()).unwrap();
            if fail_vfpp {
                m.0.borrow_mut().fail_vfpp_write = true;
            } else {
                m.0.borrow_mut().fail_write = true;
            }
            assert!(lease.restore().is_err());
            drop(lease);
            assert_eq!(
                m.0.borrow().writes,
                vec![(RUN_MODE, 2), (VFPP, 2), (VFPP, 0), (RUN_MODE, 3)]
            );
            if fail_vfpp {
                assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (3, 2));
            } else {
                assert_eq!((m.0.borrow().mode, m.0.borrow().vfpp), (2, 0));
            }
        }
    }
}
