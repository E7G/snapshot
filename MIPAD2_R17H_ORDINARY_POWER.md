# R17h: ordinary-power, per-frame-tracing-OFF recording checkpoint

2026-10-21. Scoped performance evidence, not Android/Windows parity or image acceptance.

Kernel `6.14.0-mipad2-integrated-r26`, built from kernel source
`6074419168fbb70aa5685e976874da6cf86d1fae`; retained R17h Snapshot source
`8eea07bef25a3cbbff302ff5b33b1cd552779d52`, binary SHA-256
`9edb3036db463f770cb31599ed5d3455f0ae5d3070b754210c9b3733a0b1063c`.
R17i's rejected input-queue hook is NOT enabled or restored.

## Actual four-clip results

Two fresh application processes, first front→rear, second rear→front. Each
recorded both physical cameras for approximately 30 seconds at native NV12
1920×1080 nominal30, requested H.264 15 Mbps plus AAC 192 kbps/48 kHz/stereo.
Saved files independently report H.264 1920×1080 and AAC 48 kHz/stereo. Actual
video rates 14.77–15.29 Mbps; audio 191.8–192.3 kbps. No lower quality profile.

| Process | Sensor | MP4 packets = decoded frames | PTS-derived fps | Maximum PTS gap |
|---|---|---:|---:|---:|
| 1 | front | 897 | 30.001339 | 34 ms |
| 1 | rear | 913 | 30.011518 | 34 ms |
| 2 | rear | 917 | 30.011140 | 34 ms |
| 2 | front | 899 | 30.001002 | 34 ms |

All four entire saved clips pass fps>29.5/maxgap<50ms. Zero gaps>50ms,
zero adjacent exact decoded-frame repeats, video frame counts agree with MP4
packets, AAC NULL-decode succeeds. Integer packet PTS and exact stream timebase
are used, not guessed `r_frame_rate`, rewritten timestamps, trimming or fitted
frame alignment. Raw media, frame hashes and logs stay device-private.

Native mode is read before/after each recording, not by ioctl during the clip.
Front hardware reports 78125/2604 fps (30.001920...), rear30/1. A first harness
attempt incorrectly demanded literal30/1 and stopped BEFORE recording any clip.
That zero-clip rejection remains retained. The corrected parser accepts explicit
positive rationals within nominal30±0.1%; 15fps/720p/YUYV/unknown physical input,
bad PTS and changed audio/video formats remain rejected. Eight harness contract
tests pass normally and with Python `-O`.

## What changed versus earlier diagnostic cohorts

Same tested R17h binary and functional private plugins; CURRENT_CAPS1,
RAW_DIRECT1, RAW_ASYNC0, COPY1, ALLOCATION_GUARD1, no source/encode input queue
experiment. No GST_TRACERS, per-frame capture/content trace, V4L2 wait/stamp
logging, perf/ftrace/probes, DOT dumping or CPU/GPU policy writes. Original
thermal guard and schedutil policies remain active; coarse five-second health
reads only. Sampled peaks65°C and67°C; observed maximum CPU policy1.84–2.24GHz.

This differs from earlier instrumentation AND the cooperative1.6GHz test cap:
it does NOT prove tracing alone caused previous stalls. Those negative cohorts
and the R17i rejection are not overwritten or relabeled. Conversely, slower
instrumented/capped timings alone are not proof ordinary-user recording fails.

## Independent post-test checkpoint

Same boot, zero new reboots/RTC tests/tones. 292 protected files, 64 R17h files
and four private plugin hashes match. Original exposure/gain restored; both
sensors and ISP have runtime_usage0/runtime_status=suspended. No camera owner,
perf process, tracing instance, kprobe or uprobe remains; global events disabled.
Original thermal guard/no-idle active; GPU200–600MHz, saved stable GRUB entry and
empty one-shot entry preserved. Selected whole-boot BUG/WARNING/Oops/GPU-hang/
sensor-NACK/stream-stop-timeout counts are zero. No default launcher changed.

## Still open

These background/locked automated captures do NOT independently establish
visible foreground preview cadence, actual negotiated720p preview, exact active
hardware encoder factory, native-to-encoder frame conservation, moving-scene
optics, AE/AWB/focus quality or Android parity. Four successful clips are a
repeatability checkpoint, not a long-duration or cold-boot qualification.

The image currently installs stock distribution Snapshot. The tested app and
private plugins are not yet a portable, image-integrated camera package. The
new `contrib/mipad2/portable-plugins` source tool pins three existing functional
patches and official1.28.7 archives, excludes diagnostic plugins, and rejects
wrong build ABI. Its 13 tests pass normally/`-O`; all three real source patches
apply with exact resulting hashes. Full host build was intentionally rejected:
host pkg-config reports1.28.4, required1.28.7. No portable binaries were built,
installed, device-tested or promoted by that source-preparation checkpoint.
