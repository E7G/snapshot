// SPDX-License-Identifier: GPL-3.0-or-later
//! Diagnostic-only content accounting. Digests never leave process memory.
use gtk::glib::{Checksum, ChecksumType};
use std::collections::VecDeque;

pub(super) type Digest = [u8; 16];

pub(super) fn enabled(trace: bool, content: bool, label: &str) -> bool {
    trace && content && matches!(label, "native-source" | "encoder-in")
}

/// Hash active NV12 rows, excluding allocator padding. No image/buffer retained.
pub(super) fn fingerprint(
    width: usize,
    height: usize,
    y: &[u8],
    y_stride: usize,
    uv: &[u8],
    uv_stride: usize,
) -> Option<Digest> {
    if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
        return None;
    }
    let mut checksum = Checksum::new(ChecksumType::Md5)?;
    checksum.update(b"NV12");
    checksum.update(&(width as u64).to_le_bytes());
    checksum.update(&(height as u64).to_le_bytes());
    for (plane, rows, stride) in [(y, height, y_stride), (uv, height / 2, uv_stride)] {
        if stride < width {
            return None;
        }
        let end = (rows - 1).checked_mul(stride)?.checked_add(width)?;
        if end > plane.len() {
            return None;
        }
        if stride == width {
            checksum.update(&plane[..end]);
        } else {
            for row in 0..rows {
                let start = row.checked_mul(stride)?;
                checksum.update(plane.get(start..start.checked_add(width)?)?);
            }
        }
    }
    checksum.digest().try_into().ok()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Sample {
    pub id: u64,
    pub pts: Option<u64>,
    digest: Digest,
}

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Match {
    Unique(Sample),
    Ambiguous,
    Missing,
}

pub(super) struct History {
    pub epoch: u64,
    owner: u64,
    next_id: u64,
    capacity: usize,
    samples: VecDeque<Sample>,
}

impl History {
    pub fn new(capacity: usize) -> Self {
        Self {
            epoch: 0,
            owner: 0,
            next_id: 0,
            capacity,
            samples: VecDeque::new(),
        }
    }

    pub fn begin(&mut self, owner: u64) {
        self.owner = owner;
        self.epoch += 1;
        self.samples.clear();
    }

    pub fn observe(&mut self, owner: u64, pts: Option<u64>, digest: Digest) -> bool {
        if owner == 0 || owner != self.owner || self.capacity == 0 {
            return false;
        }
        self.next_id += 1;
        self.samples.push_back(Sample {
            id: self.next_id,
            pts,
            digest,
        });
        if self.samples.len() > self.capacity {
            self.samples.pop_front();
        }
        true
    }

    pub fn resolve(&self, pts: Option<u64>, digest: Digest) -> Match {
        // PTS may be rebased by camerabin. Content must match in either case.
        let same: Vec<_> = self
            .samples
            .iter()
            .filter(|s| s.digest == digest)
            .copied()
            .collect();
        if let Some(pts) = pts {
            let exact: Vec<_> = same
                .iter()
                .filter(|s| s.pts == Some(pts))
                .copied()
                .collect();
            if exact.len() == 1 {
                return Match::Unique(exact[0]);
            }
        }
        match same.as_slice() {
            [] => Match::Missing,
            [sample] => Match::Unique(*sample),
            _ => Match::Ambiguous,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trace_content_needs_both_flags_and_raw_boundary() {
        for trace in [false, true] {
            for content in [false, true] {
                for label in [
                    "native-source",
                    "encoder-in",
                    "shared-source",
                    "encoder-out",
                ] {
                    assert_eq!(
                        enabled(trace, content, label),
                        trace && content && matches!(label, "native-source" | "encoder-in")
                    );
                }
            }
        }
    }

    #[test]
    fn content_fingerprint_ignores_stride_and_tail_padding() {
        let packed = fingerprint(4, 4, &[7; 16], 4, &[9; 8], 4).unwrap();
        let mut y = vec![255; 40];
        for r in 0..4 {
            y[r * 8..r * 8 + 4].fill(7);
        }
        let mut uv = vec![254; 24];
        for r in 0..2 {
            uv[r * 8..r * 8 + 4].fill(9);
        }
        assert_eq!(Some(packed), fingerprint(4, 4, &y, 8, &uv, 8));
    }

    #[test]
    fn content_fingerprint_covers_last_luma_and_chroma_bytes() {
        let mut y = [7; 16];
        let mut uv = [9; 8];
        let a = fingerprint(4, 4, &y, 4, &uv, 4).unwrap();
        y[15] += 1;
        assert_ne!(Some(a), fingerprint(4, 4, &y, 4, &uv, 4));
        y[15] -= 1;
        uv[7] += 1;
        assert_ne!(Some(a), fingerprint(4, 4, &y, 4, &uv, 4));
    }

    #[test]
    fn content_fingerprint_includes_geometry() {
        assert_ne!(
            fingerprint(4, 4, &[7; 16], 4, &[9; 8], 4),
            fingerprint(8, 2, &[7; 16], 8, &[9; 8], 8)
        );
    }

    #[test]
    fn content_fingerprint_declines_invalid_layouts() {
        for (w, h, ys, uvs) in [
            (0, 4, 4, 4),
            (4, 0, 4, 4),
            (3, 4, 4, 4),
            (4, 3, 4, 4),
            (4, 4, 3, 4),
            (4, 4, 4, 3),
            (usize::MAX - 1, 4, usize::MAX, usize::MAX),
        ] {
            assert!(fingerprint(w, h, &[7; 16], ys, &[9; 8], uvs).is_none());
        }
        assert!(fingerprint(4, 4, &[7; 15], 4, &[9; 8], 4).is_none());
        assert!(fingerprint(4, 4, &[7; 16], 4, &[9; 7], 4).is_none());
    }

    #[test]
    fn content_history_distinguishes_content_from_timestamps() {
        let mut h = History::new(8);
        h.begin(1);
        assert!(h.observe(1, Some(30), [1; 16]));
        assert!(matches!(h.resolve(Some(30), [2; 16]), Match::Missing));
        assert!(matches!(
            h.resolve(Some(0), [1; 16]),
            Match::Unique(Sample { pts: Some(30), .. })
        ));
        assert!(matches!(h.resolve(None, [1; 16]), Match::Unique(_)));
    }

    #[test]
    fn content_history_never_guesses_ambiguous_repeated_content() {
        let mut h = History::new(8);
        h.begin(1);
        h.observe(1, Some(30), [1; 16]);
        h.observe(1, Some(60), [1; 16]);
        assert_eq!(h.resolve(None, [1; 16]), Match::Ambiguous);
        assert_eq!(h.resolve(Some(0), [1; 16]), Match::Ambiguous);
        assert!(matches!(
            h.resolve(Some(60), [1; 16]),
            Match::Unique(Sample { id: 2, .. })
        ));
        h.observe(1, Some(60), [1; 16]);
        assert_eq!(h.resolve(Some(60), [1; 16]), Match::Ambiguous);
    }

    #[test]
    fn content_history_bounds_memory_and_clears_on_epoch() {
        let mut h = History::new(2);
        h.begin(1);
        for n in 1..=3 {
            h.observe(1, Some(n), [n as u8; 16]);
        }
        assert_eq!(h.samples.len(), 2);
        assert_eq!(h.resolve(Some(1), [1; 16]), Match::Missing);
        h.begin(2);
        assert_eq!(h.epoch, 2);
        assert_eq!(h.resolve(Some(3), [3; 16]), Match::Missing);
        assert!(!h.observe(1, Some(4), [4; 16]));
        assert!(h.observe(2, Some(5), [5; 16]));
        assert!(matches!(
            h.resolve(Some(5), [5; 16]),
            Match::Unique(Sample { id: 4, .. })
        ));
    }

    #[test]
    fn content_history_zero_capacity_and_unowned_are_disabled() {
        let mut h = History::new(0);
        h.begin(1);
        assert!(!h.observe(1, None, [0; 16]));
        let mut h = History::new(8);
        assert!(!h.observe(0, None, [0; 16]));
        assert_eq!(h.resolve(None, [0; 16]), Match::Missing);
    }
}
