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

## R15 initial broader regression, R15c complete actuator correction

Initial R15 gain-only change FAILED full framework regression:43/44pass,
original wide-light delayed-feedback fixture ends at Y80/e2/g2 forlight20.
No package or installation occurred; failure report/sourcecommit retained.
A relative exposure-step/gain-step fallback prototype also FAILED the same
old fixture (13/14puretests), so it was removed, not accepted as a solution.
Exact trace shows delayed feedback cycling exposure2->3->4->3->2 at gain2:
gainrounding was fixed but forced downward exposure nudge created a second
coarse-actuator cycle. Original convergence expectation was NOT weakened.

R15c also uses normal quantization for Exposure reductions. When a damped
reduction rounds back to the present linecode it does not force an extra
downwardline based on a stale frame. Upward nudge remains to escape dark
quantization deadzones; strong resolvable reductions retain existing policy.
Both actuator bounds/steps/minimum saturation and onecontrol/readback remain.
All14 exact pure-algorithm hosttests nowpass, including the unchanged original
wide-light/delay and highlightconstraint regressions and new lowgain cases.
Fresh fullrelease44framework and real-camera convergence/regressions still
required. Immutable R15c prefix, not overwriting failed/uninstalled R15state.

## Actual R15c bounded hardware gates, 2026-10-14

Source78d677c9c260,44frameworktests/fullrelease/privatepackage; running R19
kernel8c57f2be2 unchanged. Native front seeded only while no app/videoowner,
both sensors' originalexposure/gain cached beforewrites. Two independent
starts at exposure1976, gain1 andgain2; first closedgracefully before seed2.
- seed1: initialgain1, firstY91.3; last20 Y[126.9, 127.0], exposure/gain[[1825, 2]], zero lateupdates or gain2->1 reversal, actualnative1080p30.
- seed2: initialgain2, firstY131.8; last20 Y[126.9, 126.9], exposure/gain[[1819, 2]], zero lateupdates or gain2->1 reversal, actualnative1080p30.

This corrects the measured1/2 gain chatter in the observed scene. Not a
controlled illumination/Android optical benchmark or all-scene 3A proof.
Four actual full-native JPEGs (front2560x1920 twice, rear3264x2448 twice)
decode; physicalownerinput/nativebuffer/sensorgeometry and ISP2/VFPP2 are
observed; everyshot restoresISP3/VFPP0 and fresh1080p30. Four postphoto10s
clips alternate rear/front/rear/front, actualnative1920x1080H264:
- rear cycle0: 323 packets, 30.011185fps, maxPTSgap34.000ms, no>100ms gaps.
- front cycle1: 308 packets, 29.999022fps, maxPTSgap34.000ms, no>100ms gaps.
- rear cycle2: 315 packets, 30.011468fps, maxPTSgap34.000ms, no>100ms gaps.
- front cycle3: 299 packets, 29.997987fps, maxPTSgap34.000ms, no>100ms gaps.

Post-mode-change/recording front AE also has20fresh steady samples in target
deadband with one fixed exposure/gain pair; no lost controller epoch. This is
still a current-scene meter observation, not all-scene optical quality.
All four video/audio softwareNULL decodes pass, no playback/transfer. Graceful
close0, no videoowner, bothsensorPM0/suspended; originalexposure/gain restored
exactly whileidle.10sRTC resumes sameboot, actualS0I3+9,069,088us,
PMdebug restored. Normalsettings/r6launcher/priorbinaries/stablebqdiag1 saved
bootdefault remain. CandidateNOTpromoted; source/reference/evidence published
without private images/video or credentials. PreviousR14/R15failures retained.
See MIPAD2_AE_R15_AUTOMATED_EVIDENCE.json; fullobjective remainsactive.
