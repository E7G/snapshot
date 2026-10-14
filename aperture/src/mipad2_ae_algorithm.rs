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
/// Favor longer exposure over noise, and prefer a resolvable gain reduction.
/// Use fine exposure when the requested reduction is below one gain-code step.
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
    const CLIPPING_LIMIT: f64 = 0.10;
    // A clipping constraint and the luminance target select ONE feedback gain.
    // Hard clipping-up/down branches with a minimum 5% nudge chatter when a
    // dark midtone and a bright area straddle the clipping threshold. Retain
    // the same target/limit, but use the tighter ratio and one shared deadband.
    // This is processed-frame feedback, not calibrated RAW/Intel AIQ statistics.
    let luminance_gain = TARGET / stats.mean.max(1.0);
    let highlight_gain = if stats.clipped > 0.0 {
        CLIPPING_LIMIT / stats.clipped
    } else {
        f64::INFINITY
    };
    let constrained_gain = luminance_gain.min(highlight_gain);
    let reduce = constrained_gain < TARGET / (TARGET + DEAD_BAND);
    let increase = constrained_gain > TARGET / (TARGET - DEAD_BAND);
    if !reduce && !increase {
        return None;
    }
    // Damped, bounded feedback plus the caller's settling interval. Native
    // codes need not be a physical-gain model; quantization still guarantees
    // a bounded single-control update, including a zero minimum exposure.
    // Clip fraction is not linear luminance: further damp this branch. The
    // delayed high-contrast model rejects square-root-only clipping feedback.
    let factor = if highlight_gain < luminance_gain {
        constrained_gain.powf(0.25)
    } else {
        constrained_gain.sqrt()
    }
    .clamp(0.80, 1.25);
    if reduce && gain > gr.min {
        // Do not force an adjacent gain code when the damped target rounds
        // back to this code. Near unity that "nudge" can halve the sensor
        // gain and form a 1<->2 cycle at maximum exposure. Fine exposure can
        // meet the same target without reversing the required gain increase.
        let value = gr.quantize(gain as f64 * factor);
        if value != gain {
            return Some(Update::Gain(value));
        }
    }
    if increase && exposure < er.max || reduce && exposure > er.min {
        // Exposure also becomes coarse near its floor. A rounded-back
        // reduction must not force an extra line decrease on a delayed
        // sample; that can produce a 2<->4 line cycle at gain2. Keep upward
        // nudges to escape a dark quantization dead-zone, but let reductions
        // stop at the nearest legal actuator code.
        let value = if reduce {
            er.quantize(exposure as f64 * factor)
        } else {
            er.nudge(exposure, factor)
        };
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

#[cfg(test)]
mod highlight_constraint_regression {
    use super::*;
    const E: Range = Range {
        min: 1,
        max: 3000,
        step: 1,
    };
    const G: Range = Range {
        min: 1,
        max: 127,
        step: 1,
    };
    #[test]
    fn highlight_and_luminance_conflict_has_shared_deadband() {
        // Actual r6 journal had Y~94..104 and clipped~0.098..0.107 at gain1.
        // Old hard branch alternated between brightness-up and clipping-down.
        for mean in [30.0, 80.0, 96.0, 110.0, 120.0] {
            for clipped in [0.099, 0.100, 0.101] {
                assert_eq!(
                    next(Stats { mean, clipped }, 1200, 1, E, G),
                    None,
                    "mean={mean}, clipped={clipped}"
                );
            }
        }
    }
    #[test]
    fn strong_highlights_never_increase_even_when_midtones_dark() {
        for mean in [1.0, 30.0, 90.0, 120.0] {
            for clipped in [0.12, 0.2, 0.8, 1.0] {
                assert!(
                    matches!(next(Stats{mean,clipped},1200,1,E,G),Some(Update::Exposure(v)) if v<1200)
                );
                assert!(match next(Stats { mean, clipped }, 1200, 8, E, G) {
                    Some(Update::Gain(v)) => v < 8,
                    Some(Update::Exposure(v)) => v < 1200,
                    None => false,
                });
            }
        }
    }
    #[test]
    fn highlight_limited_but_achievable_scenes_settle() {
        // Toy monotonic scene model, including feedback delay/noise. NOT pixel,
        // physical gain calibration, sensor/ISP HDR, or real image-quality proof.
        for target_e in [120.0, 600.0, 1200.0, 2200.0] {
            for target_mean in [30.0, 80.0, 96.0, 110.0] {
                for delayed in [false, true] {
                    let (mut e, mut g) = (300, 8);
                    let mut prev = Stats {
                        mean: 120.0,
                        clipped: 0.0,
                    };
                    let mut late_updates = 0;
                    let mut min_e = i32::MAX;
                    let mut max_e = 0;
                    for i in 0..300 {
                        let response = e as f64 * g as f64 / target_e;
                        let noise = if i % 2 == 0 { 0.0005 } else { -0.0005 };
                        let observed = Stats {
                            mean: (target_mean * response).min(255.0),
                            clipped: (0.1 + 0.2 * (response - 1.0) + noise).clamp(0.0, 1.0),
                        };
                        if let Some(u) = next(if delayed { prev } else { observed }, e, g, E, G) {
                            match u {
                                Update::Exposure(v) => e = v,
                                Update::Gain(v) => g = v,
                            };
                            if i >= 200 {
                                late_updates += 1;
                            }
                        }
                        assert!((E.min..=E.max).contains(&e) && (G.min..=G.max).contains(&g));
                        if i >= 200 {
                            min_e = min_e.min(e);
                            max_e = max_e.max(e);
                        }
                        prev = observed;
                    }
                    assert!(
                        late_updates <= 4,
                        "target={target_e} Y={target_mean} delayed={delayed}: updates={late_updates}"
                    );
                    assert!(
                        (max_e - min_e) as f64 <= target_e * 0.08,
                        "late swing {min_e}..{max_e}"
                    );
                    let response = e as f64 * g as f64 / target_e;
                    assert!(
                        (0.06..=0.13).contains(&(0.1 + 0.2 * (response - 1.0))),
                        "final clip constraint response={response}"
                    );
                }
            }
        }
    }
    #[test]
    fn unconstrained_deadband_and_control_quantization_unchanged() {
        for mean in [112.0, 120.0, 128.0] {
            assert_eq!(next(Stats { mean, clipped: 0.0 }, 1200, 8, E, G), None);
        }
        for mean in [30.0, 120.0, 200.0] {
            for clipped in [0.0, 0.099, 0.1, 0.101, 0.2, 1.0] {
                let er = Range {
                    min: 3,
                    max: 1001,
                    step: 7,
                };
                for e in (3..=997).step_by(7) {
                    if let Some(u) = next(Stats { mean, clipped }, e, 8, er, G) {
                        let (v, r) = match u {
                            Update::Exposure(v) => (v, er),
                            Update::Gain(v) => (v, G),
                        };
                        assert!((r.min..=r.max).contains(&v));
                        assert_eq!((v - r.min) % r.step, 0);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod coarse_gain_regression {
    use super::*;
    const E: Range = Range {
        min: 1,
        max: 1976,
        step: 1,
    };
    const G: Range = Range {
        min: 1,
        max: 127,
        step: 1,
    };
    #[test]
    fn sampled_front_low_gain_pair_must_not_form_a_two_code_cycle() {
        // Actual R14d processed-frame journal, not a calibrated RAW model.
        let dark = Stats {
            mean: 92.2,
            clipped: 0.031,
        };
        let bright = Stats {
            mean: 132.2,
            clipped: 0.088,
        };
        assert_eq!(next(dark, 1976, 1, E, G), Some(Update::Gain(2)));
        let result = next(bright, 1976, 2, E, G);
        assert!(
            matches!(result,Some(Update::Exposure(v)) if v<1976 && v>1500),
            "R15 fine exposure expected instead of reverse Gain(1): {result:?}"
        );
    }
    #[test]
    fn coarse_gain_with_available_exposure_settles_in_delayed_toy_scene() {
        // Fitted only to the two logged processed-luma values. This is a
        // quantization/delay regression, NOT physical camera quality proof.
        let gamma = (132.2f64 / 92.2).ln() / 2.0f64.ln();
        for delay in [false, true] {
            for light in [0.5f64, 1.0, 2.0, 4.0] {
                for initial in [(1976, 1), (1976, 2), (800, 4)] {
                    let (mut e, mut g) = initial;
                    let mut previous = Stats {
                        mean: 120.0,
                        clipped: 0.0,
                    };
                    let mut late_updates = 0;
                    let mut last_gain = None;
                    let mut late_gain_reversals = 0;
                    for i in 0..250 {
                        let response = light * e as f64 * g as f64 / 1976.0;
                        let stats = Stats {
                            mean: (92.2 * response.powf(gamma)).min(255.0),
                            clipped: (0.031 * response.powf(1.5)).min(1.0),
                        };
                        if let Some(update) = next(if delay { previous } else { stats }, e, g, E, G)
                        {
                            match update {
                                Update::Exposure(v) => e = v,
                                Update::Gain(v) => {
                                    if i >= 200 && last_gain == Some(v) {
                                        late_gain_reversals += 1;
                                    }
                                    last_gain = Some(g);
                                    g = v;
                                }
                            }
                            if i >= 200 {
                                late_updates += 1;
                            }
                        }
                        previous = stats;
                        assert!((E.min..=E.max).contains(&e) && (G.min..=G.max).contains(&g));
                    }
                    let final_y = 92.2 * (light * e as f64 * g as f64 / 1976.0).powf(gamma);
                    assert!(
                        (112.0..=128.0).contains(&final_y),
                        "light={light} delay={delay} initial={initial:?} y={final_y} e={e} g={g}"
                    );
                    assert!(
                        late_updates <= 4 && late_gain_reversals == 0,
                        "late updates={late_updates} reversals={late_gain_reversals}"
                    );
                }
            }
        }
    }
    #[test]
    fn fine_rear_gain_and_strong_reduction_keep_existing_priority() {
        let rear = Range {
            min: 128,
            max: 2047,
            step: 1,
        };
        assert!(
            matches!(next(Stats{mean:132.2,clipped:0.088},1976,208,E,rear),Some(Update::Gain(v)) if v<208)
        );
        assert_eq!(
            next(
                Stats {
                    mean: 200.0,
                    clipped: 0.0
                },
                800,
                8,
                E,
                G
            ),
            Some(Update::Gain(6))
        );
        // A rounding dead-zone at gain2 uses fine exposure; no forced50% jump.
        assert!(
            matches!(next(Stats{mean:200.0,clipped:0.0},800,2,E,G),Some(Update::Exposure(v)) if v<800)
        );
        // At both minimum exposure and a rounded-back gain, report no legal
        // fine update rather than artificially forcing an adjacent gaincode.
        assert_eq!(
            next(
                Stats {
                    mean: 132.2,
                    clipped: 0.088
                },
                1,
                2,
                E,
                G
            ),
            None
        );
    }
}
