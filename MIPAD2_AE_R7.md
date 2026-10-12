# R7 continuous highlight-constrained AE candidate

2026-10-12. R18 kernel remains unchanged. Previous r6 capture/quit lifecycle
and all sensor-native ranges/one-control-per-update semantics are retained.
No kernel-timing/gain calibration/RAW/Intel AIQ layout guess, AWB or HDR change.

Actual r6 front gain1 journal repeatedly reports mean94..104 and clip~10%,
alternating exposure-up/down. Scene was unlabelled: this alone is not causal
or fixed-scene stability proof. The source has conflicting hard branches and
a forced minimum5% nudge. A compiled old-source regression independently
fails the clipping-boundary shared-deadband requirement.

Keep existing mean target120, deadband8 and clipping limit0.10. Compute
brightness gain and highlight constraint gain, choose their minimum, use one
common relative deadband for both direction and magnitude. Damp the clipping
branch further because clipped fraction is not linear frame luminance.
Never compensate dark midtones by increasing beyond the highlight constraint.
The current settled highlight-limited mean may be below120; this does not
restore unrecoverable highlights or claim full image-quality/AIQ equivalence.

Seven existing algorithm tests and four new regressions pass (11 total).
New tests retain bounds/quantization, strong highlights prevent increasing,
threshold conflict has a shared deadband, and32 toy delayed/noisy scene cases
x300 updates settle. These are source/control-model tests, not real sensor
calibration, physical HDR or pixel-quality proof. First proposed square-root
clipping candidate FAILED the unchanged delayed-scene regression; it was not
applied or deployed. Further-damped candidate passes the same test. Failed
candidate logs and original-source runtime-red baseline are retained.
Steady log now includes clipped fraction so real constraint behavior can be
observed rather than inferred from mean alone.

Full workspace framework tests/release resources and real front/rear preview,
changing-scene recovery, quit/reopen, photos/recording remain to be run before
claiming R7 deployment/validation. Device currently runs r6, default launcher
unchanged. General kernel/flash objective remains active and incomplete.

Primary design context (not copied algorithm/calibration):
https://docs.libcamera.org/master/internal-api/classlibcamera_1_1ipa_1_1AgcMeanLuminance.html
https://docs.libcamera.org/master/internal-api/structlibcamera_1_1ipa_1_1AgcMeanLuminance_1_1AgcConstraint.html
