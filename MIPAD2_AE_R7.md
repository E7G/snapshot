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


## Recording selection repair (2026-10-13, still a candidate)

User's R18/r6 clip saves/plays but recording stutters. Actual VP8/Vorbis clip
has66 packets at1920x1440, effective11.68fps despite30/1 metadata. Device lacks
VA plugin; i965 advertises H264 encoding, but matched signed VA1.28.7-1 plugin
explicitly blocklists i965 for encoding. Default environment is NOT overridden.
Scoped synthetic override produces valid decoded H264, but real-time gates
are NOT all passed. Ball source alone is CPU-heavy; separate SMPTE tests retain
the I420 conversion path's failed wall-time gate. On current stable bqdiag1,
real front NV12 H264 runs28.65fps, while rear fails negotiation; both complete
mutable snapshots restore. This is not R18 real recording/full30fps proof.

Fix source hardware-only H264 eligibility under hardware opt-in (otherwise
VP8 fallback); reset/update encoder ranks for either codec when preference
changes, without duplicate reset. Actual old-source selector compiles then
fails hardware-only enabled case; corrected helper4 cases pass,8-case matrix
added to workspace tests. VP8's target-bitrate was2048bits/s while the shared
constant is2048kbps. Correct VP8/OpenH264 bits/s units and explicitly request
realtime VP8 fallback. Resolution/frame-rate unchanged. Hardware plugin/default
selection, real app recording/preview responsiveness and quality remain to test.
Do not infer real-time speed just from evenly spaced synthetic timestamps.

## 2026-10-13: built candidate and isolated real-camera encode probes

Built/package source commit: `e52fd88d998bd4c105c902f355ed692d6e20ee01`. Complete offline release
build, resource generation and 24 workspace framework tests passed. Package
installed only under `/usr/local/lib/mipad2-snapshot-ae-r7`; verified binary
SHA256 `8bb333692f85709d0d2b73b7b6dbe14738b802fc9505943981bd0d9d1b8cdc3d`.
The r6 binary/default launcher and normal application settings are preserved.
This documentation commit is not the build/package source commit.

The original user's VP8/Vorbis recording at 1920x1440 had 66 video packets,
11.682 fps from packet timestamps and a maximum gap of 547 ms, despite its
nominal 30/1 metadata. This is a recording cadence defect, not proof that the
sensor cannot deliver 30 fps. Private image/audio contents are not published.

Device GStreamer 1.28.7-1 lacks the VA plugin. A matching signed official
`gst-plugin-va` 1.28.7-1 package was signature-verified and extracted privately,
not installed/upgraded system-wide. The i965 encoder is disabled by its upstream
driver filter by default. Diagnostic processes opt in using `GST_VA_ALL_DRIVERS=1`,
`LIBVA_DRIVER_NAME=i965`, a private plugin path and private registry only.
No global filter override or application hardware-encoding default is changed.
Primary encoder contract (NV12 sink and bitrate in kbps):
https://gstreamer.freedesktop.org/documentation/va/vah264enc.html

Kernel: `6.14.0-mipad2-integrated-r18`, production source `86c16f5c75fefb42c5b6c89025d2f5475c912f45`.
Real camera capture uses one MMAP source at 1920x1440@30, not a synthetic source.
Each case encodes 360 H264 packets, has matching dimensions, and decodes with
FFmpeg without error. All sampled writable scalar controls, both sensor
formats/crops, and original video state restore exactly after every case.
No new dmesg messages during this five-case batch.

| Case | PTS fps | Max PTS gap ms | Actual PLAYING s | Whole process s | CPU s |
| --- | ---: | ---: | ---: | ---: | ---: |
| cold-front | 30.002 | 34.000 | 12.214 | 13.797 | 6.306 |
| warm-front | 30.003 | 34.000 | 12.224 | 12.802 | 5.303 |
| warm-rear | 30.012 | 34.000 | 12.180 | 12.814 | 5.222 |
| i420-front | 29.918 | 43.334 | 12.235 | 12.801 | 7.634 |
| i420-rear | 29.928 | 42.666 | 12.209 | 12.792 | 7.942 |

NV12 cases pass camera frames directly to `vah264enc`. I420 cases add CPU
`videoconvert` to NV12, showing the current application's preferred input
format cost without changing its source preference or reducing resolution.
The sustained diagnostic gate is explicit: 360 packets, PTS rate within 1% of
30, maximum gap below 50 ms, measured gst-launch PLAYING execution below 12.6 s.
Cold/warm whole-process overhead is reported separately, not counted as camera
throughput. This is not a GUI + preview + audio performance result.

Retained earlier failures:
- Initial synthetic ball source consumed over seven seconds by itself for
  120 frames; whole encode pipeline failed the six-second wall gate.
- Synthetic I420 conversion case also failed that wall gate.
- Old stable bqdiag1 rear NV12 pipeline failed negotiation.
- First R18 front 120-frame run had 7.757 s whole-process wall versus 4.267 s
  PLAYING time, failing its original six-second whole-wall gate. The report is
  retained as failed; the later longer sustained test does not rewrite it.
- First real-camera fixture lacked a Path import and failed before hardware writes.
- The first private r7 GUI launch lacked copied XDG user directory config.
  It was closed gracefully; only `user-dirs.dirs`/`user-dirs.locale` were copied
  into the private configuration before one explicit relaunch. No normal config
  files were modified. Gallery initialization then found existing media.

A private r7 GUI candidate now has hardware encoding enabled only in its
keyfile-backed private settings; shutter sounds are disabled in that candidate.
Preview, actual selected encoder/caps, audio-bearing recording, sustained GUI
cadence, user responsiveness, and quality remain unverified at this checkpoint.
Merely choosing an H264 profile or listing an encoder is not an actual encode
proof. VA decoder DMABUF allocation-size warnings were observed while loading
the existing VP8 gallery item; no claim that those warnings are fixed.
AE's host noisy/delayed scene tests are not a real illumination/quality proof.
The first SSH-only R18 return did not recover and the user reported backlit
black screen. A later same-image verbose USB boot succeeded, changing multiple
conditions; this does not establish USB as the cause or resolve boot reliability.
Full kernel/camera/flash-image completion remains pending.
