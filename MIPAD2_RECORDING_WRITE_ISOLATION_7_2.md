# 7.2 recording write isolation: bounded queue experiment

This follows the [actual rational VA bitrate fix](MIPAD2_VA_FRAME_RATE_7_2.md).
It does not change the kernel, default camera, source caps/PTS, 1080p resolution,
requested 15 Mbps CBR, reference count or B-frame policy. All runs below load the
same fixed full-recipe VA plugin and i965 driver independently observed in process.
No audible tests, lock-policy bypass or CPU/GPU frequency policy writes.

## Trace boundary / observed stall

The diagnostic C [pad probe](contrib/mipad2/va-frame-rate/pad-cadence-probe.c)
records only scalar buffer metadata into preallocated memory at source, encoder
output and parser output. It writes a CSV only after pipeline shutdown. Actual
metadata/per-frame timing and recordings remain device-private, not repository
fixtures. The tool requires elements named `capture`, `encoder`, `parser` and
takes two arguments: a quoted GStreamer pipeline and a NEW private CSV path.
Compile against matching GStreamer 1.28.7 development libraries; not a production
launcher or a replacement for uninstrumented application qualification.

Without an encoded writer queue, a 1800-frame run stalls near input indices600
and1500 for about509/689 ms; source PTS gaps follow eight buffers later, about302
and482 ms. Source and encoder relative PTS match exactly (max error0ns): the
timestamp discontinuities already exist before encoding. This alone does not
identify which downstream stage blocked the source or prove physical sensor loss.

A separate strace write-only probe observed one688 ms `writev` overlapping a source
arrival stall. HOWEVER tracing reduced cadence to24.65fps with CPU ceilings
varying1.84/2.24GHz. Treat it as confounded diagnostic observation, not a clean
performance benchmark or proof that storage is the sole root cause. Wall/monotonic
alignment was approximated after capture, not precisely calibrated in capture.
No write-buffer contents were logged. No inference that parser byte offsets are
frame numbers: only raw-source offsets have possible frame-sequence meaning.

## Whole-clip experiment (all startup frames retained)

The sole pipeline change for the queue-on run was inserting:

```text
h264parse name=parser ! queue name=encoded_writer_queue max-size-buffers=64 max-size-bytes=16777216 max-size-time=2000000000 leaky=no ! mp4mux
```

This is encoded data, not a large raw-frame queue. All three limits are finite;
the queue blocks instead of dropping when full. It decouples downstream work
including muxing and file writing, so this does not uniquely attribute the effect
to filesystem writes. No `videorate`, PTS rewrite, quality reduction or leaky mode.

| Run | Decoded frames | Whole-clip fps | Max PTS interval | >50ms intervals | Actual bitrate | Max temperature |
|---|---:|---:|---:|---:|---:|---:|
| Queue off, 60s | 1800 | 29.6553 | 481.667 ms | 2 | 15.013 Mbps | 61 C |
| Queue on, 60s | 1800 | 30.0017 | 34.000 ms | 0 | 15.186 Mbps | 60 C |
| Queue off recheck, 30s | 900 | 29.6716 | 380.333 ms | 1 | 15.187 Mbps | 59 C |

Every packet decoded; no adjacent exact decoded repeats. Relative source/encoder
PTS errors remain0ns. A source offset step of2 at startup remains in these trials;
this is NOT physical sensor-frame conservation proof. No startup exclusion.
The queue-on source arrival maximum was54.339 ms; encoder output arrival still
bursts (max69.234ms,57 intervals>50ms). Thus stable stored PTS is NOT proof of
uniform live delivery or smooth preview. The 60s queue-on sample is a limited
positive result, not sufficient to promote an application/default or prove
unbounded reliability. Read the recheck result in the table, not an assumed outcome.

## Integration remains open

The normal Snapshot app was NOT modified or exercised with this new queue. Its
existing camerabin/encodebin internal queues/topology must be inspected first;
do not blindly transplant this video-only pipeline into application code.
Normal app preview + audio, dual camera/native stills, full-scene AE/AWB/focus,
repeat sustained recordings, reopen/resume and cold-boot gates remain open.
No default launcher, rootfs package or kernel image promotion performed.

Two obsolete first-comparison MP4s (136575645 bytes) were removed under the prior
cleanup authorization only after stronger same-build v2 pair evidence existed.
All their logs/metadata and the v2 pair, source baseline, normal recording and
latest experiment media remain private. Filesystem accounting settled after sync;
no protected installed file, photograph, log or kernel removed.

Final postflight: no active camera, sensors/ISP suspended usage0, no kernel fault
markers, original controls/settings restored,686 protected files and35 private
aliases/bytes unchanged, stable boot fallback preserved and no new reboot.
Scalar evidence: [summary](tests/mipad2-recording-write-isolation-7.2-summary.json).
Full MiPad2 task and normal app recording cadence remain incomplete.
