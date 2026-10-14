# R15 coarse-gain auto-exposure correction candidate

Actual R14d native front processed-frame journal records494 filtered gain1/2
updates,493 alternating pairs at exposure1976. Mean luma approximately92.2
at gain1 and132.2 at gain2, clipped0.031/0.088. Source-extracted RED reproduces
the exact cycle: dark sample choosesGain2, bright sample forcesGain1. No
visible image/scene calibration or Android optical parity inferred.

References checked before changing algorithm:
- Latte Android micamOV5693 commitd2afb1de9a761ff01dafdf9efe4736e521ab71f6,
  exposure.gain[0] is written raw into0x350a/b, no Linuxwhole-unit shift:
  https://github.com/latte-dev/android_kernel_xiaomi_latte/blob/d2afb1de9a761ff01dafdf9efe4736e521ab71f6/drivers/external_drivers/camera/drivers/media/i2c/micam/ov5693.c
- Current Linux/R19 and upstreamOV5693 analoguecontrol1..127 writesgain<<4,
  masksbits10..4, drops the fractional4bits on read. Units are different;
  do NOT pasteAndroid rawgain values into existing Linuxcontrol.
- WindowsCPF data isIntelAIQB binary tuning, not available3A algorithm source.
  No decoded/proprietaryWindows3A execution or calibration parity claimed.

R15 preserves all existing kernel gain/exposure controlunits/limits/steps.
Gain-first reduction now uses normal quantization, NOT a forced adjacent-code
nudge when the damped reduction rounds back to the presentcode. It then
uses the available fine exposure control; no50% gain2->1 jump for a4.7%
requested correction. Underexposed maximum-exposure gainincrease remains,
fine rear gain and strong resolvable gain reductions retain priority. At
both fine-exposure floor and gain-rounding deadzone it reports nolegalfine
update ratherthan fabricating convergence. Native fractional gain is a
separate future ABI-compatible driver design, not an unannounced unit change.

One control per update, queried bounds/steps/readback, settling/deadband,
highlightconstraint, epoch/cancel/AFholds and nativephoto/record pipelines
are unchanged. An old highlightfixture requiringGain-only was corrected to
accept a bounded Exposure reduction when the gain target is unresolvable;
it still rejects increase orNone for those feasible cases.

New host regressions: actual logged inputpair choosesFineExposure on bright
side;24 fitted lowgain/delay/start/light toy scenarios converge with bounded
lateupdates/no gain reversals; fine-rear/strong-gain priority and true precision
floor cases. Source-extracted RED3fail -> GREEN3pass. Initial GREEN harness
compiled the newlyintegrated fixture twice; its duplicate-module compiler
failure was a harness error, no runtime/hardware mutation. External probe
module renamed; originalRED preserved, correctedGREEN passes. These toy/host
cases do not prove real optical convergence or complete image quality.

Full release/framework and actual native front/rear exposure convergence,
reopen/photo/video/suspend regression gates are required before promotion.
Use a new immutable R15 private prefix; r6 default/R14d artifacts preserved.
Overall kernel/flash/boot/DSP/compatibility and Android-quality scope remains.
