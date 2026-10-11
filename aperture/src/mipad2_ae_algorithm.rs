// SPDX-License-Identifier: GPL-3.0-or-later
//! Frame metering and bounded feedback in native V4L2 control units.
//! No ISP2 statistics layout or proprietary Intel AIQ algorithm is assumed.

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub mean: f64,
    pub clipped: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct Range {
    pub min: i32,
    pub max: i32,
    pub step: i32,
}

impl Range {
    pub fn valid(self) -> bool {
        self.min >= 0 && self.max >= self.min && self.step >= 1
    }

    pub fn quantize(self, value: f64) -> i32 {
        let steps = ((value - self.min as f64) / self.step as f64).round();
        let max_steps = (i64::from(self.max) - i64::from(self.min)) / i64::from(self.step);
        let steps = steps.clamp(0.0, max_steps as f64) as i64;
        (i64::from(self.min) + steps * i64::from(self.step)) as i32
    }

    fn nudge(self, current: i32, factor: f64) -> i32 {
        let next = self.quantize(current as f64 * factor);
        if next == current {
            self.quantize(
                current as f64
                    + if factor > 1.0 {
                        self.step as f64
                    } else {
                        -(self.step as f64)
                    },
            )
        } else {
            next
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Update {
    Exposure(i32),
    Gain(i32),
}

/// One control per update: no partially applied exposure/gain pair.
/// Favor longer exposure over noise, and reduce gain before reducing exposure.
/// Query the driver again after a mode change; never extend VBLANK here.
pub fn next(stats: Stats, exposure: i32, gain: i32, er: Range, gr: Range) -> Option<Update> {
    if !er.valid()
        || !gr.valid()
        || !stats.mean.is_finite()
        || !stats.clipped.is_finite()
        || !(0.0..=255.0).contains(&stats.mean)
        || !(0.0..=1.0).contains(&stats.clipped)
        || !(er.min..=er.max).contains(&exposure)
        || !(gr.min..=gr.max).contains(&gain)
    {
        return None;
    }
    const TARGET: f64 = 120.0;
    const DEAD_BAND: f64 = 8.0;
    let reduce = stats.mean > TARGET + DEAD_BAND || stats.clipped > 0.10;
    let increase = stats.mean < TARGET - DEAD_BAND && stats.clipped <= 0.10;
    if !reduce && !increase {
        return None;
    }
    // Damped, bounded feedback plus a settling interval in the caller.
    // Native codes need not be a calibrated physical-gain model: feedback
    // checks the real next frame rather than pretending to know T4KA3's curve.
    let mut factor = (TARGET / stats.mean.max(1.0)).sqrt().clamp(0.80, 1.25);
    if reduce {
        factor = factor.min(0.95);
    }
    if increase {
        factor = factor.max(1.05);
    }
    if reduce && gain > gr.min {
        let value = gr.nudge(gain, factor);
        return (value != gain).then_some(Update::Gain(value));
    }
    if increase && exposure < er.max || reduce && exposure > er.min {
        let value = er.nudge(exposure, factor);
        if value != exposure {
            return Some(Update::Exposure(value));
        }
    }
    if increase && gain < gr.max {
        let value = gr.nudge(gain, factor);
        return (value != gain).then_some(Update::Gain(value));
    }
    None
}

/// Samples at most approximately 96x72 points, center weighted, stride aware.
/// `rgb` means three channel byte offsets; None means the first byte is luma.
pub fn meter(
    data: &[u8],
    width: usize,
    height: usize,
    stride: usize,
    pixel_stride: usize,
    rgb: Option<[usize; 3]>,
    limited: bool,
) -> Option<Stats> {
    if width == 0 || height == 0 || pixel_stride == 0 {
        return None;
    }
    let channel = rgb.map_or(0, |v| *v.iter().max().unwrap());
    if channel >= pixel_stride {
        return None;
    }
    let row_end = (width - 1)
        .checked_mul(pixel_stride)?
        .checked_add(channel + 1)?;
    if stride < row_end {
        return None;
    }
    let end = (height - 1).checked_mul(stride)?.checked_add(row_end)?;
    if end > data.len() {
        return None;
    }
    let mut sum = 0.0;
    let mut weight_sum = 0.0;
    let mut clipped = 0.0;
    for y in (0..height).step_by(height.div_ceil(72)) {
        for x in (0..width).step_by(width.div_ceil(96)) {
            let at = y * stride + x * pixel_stride;
            let mut value = match rgb {
                Some([r, g, b]) => {
                    0.2126 * data[at + r] as f64
                        + 0.7152 * data[at + g] as f64
                        + 0.0722 * data[at + b] as f64
                }
                None => data[at] as f64,
            };
            if limited {
                value = ((value - 16.0) * (255.0 / 219.0)).clamp(0.0, 255.0);
            }
            let weight =
                if x >= width / 4 && x < width * 3 / 4 && y >= height / 4 && y < height * 3 / 4 {
                    4.0
                } else {
                    1.0
                };
            sum += value * weight;
            weight_sum += weight;
            if value >= 250.0 {
                clipped += weight;
            }
        }
    }
    Some(Stats {
        mean: sum / weight_sum,
        clipped: clipped / weight_sum,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    const E: Range = Range {
        min: 1,
        max: 2070,
        step: 1,
    };
    const G: Range = Range {
        min: 1,
        max: 127,
        step: 1,
    };
    fn stats(mean: f64) -> Stats {
        Stats { mean, clipped: 0.0 }
    }
    #[test]
    fn rear_zero_minimum_exposure_is_valid_and_not_absorbing() {
        // Live bqdiag1 T4KA3 QUERYCTRL: exposure 0..2486, gain 128..2047.
        let er = Range {
            min: 0,
            max: 2486,
            step: 1,
        };
        let gr = Range {
            min: 128,
            max: 2047,
            step: 1,
        };
        assert!(er.valid());
        assert!(!Range { min: -1, ..er }.valid());
        assert_eq!(next(stats(30.0), 0, 128, er, gr), Some(Update::Exposure(1)));
        assert_eq!(
            next(stats(30.0), 800, 128, er, gr),
            Some(Update::Exposure(1000))
        );
        assert_eq!(
            next(stats(200.0), 800, 128, er, gr),
            Some(Update::Exposure(640))
        );
        assert_eq!(next(stats(0.0), er.max, gr.max, er, gr), None);
        assert_eq!(next(stats(255.0), er.min, gr.min, er, gr), None);
        for exposure in 0..=er.max {
            for gain in [128, 129, 512, 2047] {
                for mean in [0.0, 30.0, 120.0, 200.0, 255.0] {
                    match next(stats(mean), exposure, gain, er, gr) {
                        Some(Update::Exposure(v)) => assert!((er.min..=er.max).contains(&v)),
                        Some(Update::Gain(v)) => assert!((gr.min..=gr.max).contains(&v)),
                        None => (),
                    }
                }
            }
        }
    }

    #[test]
    fn dark_then_bright_priorities() {
        assert_eq!(next(stats(30.0), 320, 8, E, G), Some(Update::Exposure(400)));
        assert_eq!(next(stats(200.0), 800, 8, E, G), Some(Update::Gain(6)));
        assert_eq!(next(stats(30.0), 2070, 8, E, G), Some(Update::Gain(10)));
        assert!(matches!(
            next(stats(200.0), 800, 1, E, G),
            Some(Update::Exposure(_))
        ));
    }
    #[test]
    fn no_invalid_or_saturated_updates() {
        assert_eq!(next(stats(120.0), 320, 8, E, G), None);
        assert_eq!(next(stats(0.0), E.max, G.max, E, G), None);
        assert_eq!(next(stats(255.0), E.min, G.min, E, G), None);
        assert_eq!(next(stats(f64::NAN), 320, 8, E, G), None);
        assert_eq!(next(stats(30.0), 320, 0, E, G), None);
        assert_eq!(next(stats(30.0), 320, 8, Range { step: 0, ..E }, G), None);
    }
    #[test]
    fn highlights_prevent_increase() {
        assert!(
            matches!(next(Stats { mean: 90.0, clipped: 0.2 }, 320, 8, E, G), Some(Update::Gain(v)) if v < 8)
        );
    }
    #[test]
    fn stride_formats_and_ranges() {
        let mut padded = vec![255u8; 8 * 4];
        for y in 0..4 {
            padded[y * 8..y * 8 + 4].fill(126);
        }
        assert!((meter(&padded, 4, 4, 8, 1, None, true).unwrap().mean - 128.08).abs() < 0.1);
        assert_eq!(
            meter(&[200, 30, 10, 255], 1, 1, 4, 4, Some([2, 1, 0]), false)
                .unwrap()
                .clipped,
            0.0
        );
        assert!(meter(&padded[..30], 4, 4, 8, 1, None, true).is_some());
        assert!(meter(&padded[..27], 4, 4, 8, 1, None, true).is_none());
        assert!(meter(&[], 0, 0, 0, 0, None, true).is_none());
        assert!(meter(&[0; 4], usize::MAX, 2, 4, 4, None, false).is_none());
        assert!(meter(&[0; 4], 1, 1, 4, 4, Some([0, 1, 4]), false).is_none());
    }
    #[test]
    fn driver_step_never_exceeds_bounds() {
        let er = Range {
            min: 3,
            max: 1001,
            step: 7,
        };
        let gr = Range {
            min: 128,
            max: 2047,
            step: 1,
        };
        for e in (3..=997).step_by(7) {
            for g in [128, 129, 256, 1000, 2047] {
                for y in 0..=255 {
                    if let Some(u) = next(stats(y as f64), e, g, er, gr) {
                        let (r, v) = match u {
                            Update::Exposure(v) => (er, v),
                            Update::Gain(v) => (gr, v),
                        };
                        assert!((r.min..=r.max).contains(&v));
                        assert_eq!((v - r.min) % r.step, 0);
                    }
                }
            }
        }
    }
    #[test]
    fn delayed_feedback_converges_without_wild_oscillation() {
        for light in [0.0003, 0.001, 0.005, 0.02, 0.2, 1.0, 20.0] {
            let (mut e, mut g) = (320, 8);
            let mut delayed = 120.0;
            for _ in 0..250 {
                let measured = (light * e as f64 * g as f64).min(255.0);
                if let Some(u) = next(stats(delayed), e, g, E, G) {
                    match u {
                        Update::Exposure(v) => e = v,
                        Update::Gain(v) => g = v,
                    }
                }
                delayed = measured;
            }
            let final_y = (light * e as f64 * g as f64).min(255.0);
            let achievable = light * E.max as f64 * G.max as f64 >= 112.0 && light <= 128.0;
            if achievable {
                assert!(
                    (95.0..=145.0).contains(&final_y),
                    "light={light} y={final_y} e={e} g={g}"
                );
            }
        }
    }
}
