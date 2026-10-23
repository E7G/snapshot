# 7.2 normal foreground application: mixed cadence results, no promotion

## Actual stack / source audit

Normal source Snapshot application commit `0a7ce87affa902f33599819aed5c4123a548f1c6`,
binary `16bd4d3ea2d4347a7c049784760b0c305150c666241700671032e9e211bf13d9`,
with isolated source-AAC and the [bounded VA rational fix](MIPAD2_VA_FRAME_RATE_7_2.md).
The exact new VA plugin and i965 driver were observed mapped during recording.
No `MIPAD2_DIAGNOSTIC_ACTIONS`, background-capture flag, LD override or in-process
tracer. Normal window, native permissions and upstream `t` keyboard shortcut;
temporary authorized uinput keyboard destroyed after each run. The second sample
additionally uses an independent private kernel trace instance, explicitly noted.
New private profiles/wrappers only; original candidate and default launcher unchanged.

Source audit changes the next action: **normal encodebin already has a three-second
encoded output queue before muxing**. Its buffer and byte limits are zero, time
limit3s. CameraBin does not remove that queue in its element-added callback.
Do NOT blindly transplant the earlier video-only extra queue or claim it is now
integrated into this application. Existing source/preview queues are separate.

Primary source: [pinned encodebin implementation](https://raw.githubusercontent.com/GStreamer/gstreamer/1.28.7/subprojects/gst-plugins-base/gst/encoding/gstencodebasebin.c).
Source hashes and exact measured stack are in the [scalar summary](tests/mipad2-normal-application-7.2-summary.json).

## Whole-clip normal foreground recordings

Both front-camera recordings are1920x1080 H264 with48kHz stereo AAC, full video
and audio decoding passed, and no adjacent exact decoded repeats. No initial
frames excluded, no frame-rate reduction, PTS rewrite or timestamp smoothing.

| Run | Video frames decoded | PTS rate | Max interval | Nonpositive intervals | >50ms intervals | Max temperature |
|---|---:|---:|---:|---:|---:|---:|
| Normal app, no kernel trace | 996 | 29.4054fps | **556.667ms** | **1 duplicate PTS** | 7 | 79C |
| Normal app, private V4L2 trace | 1009 | 30.0015fps | 40.667ms | 0 | 0 | 75C |

Video bitrates14.65/15.24Mbps, AAC196.75/195.73kbps. The existing strict verifier
correctly rejected the first duplicate PTS; cleanup had already gracefully closed
the app and restored controls before analysis stopped. That SAME finalized file
was then fully analyzed/decoded without re-recording or relaxing its failed gate.

**Overall application cadence is NOT fixed or qualified.** One passing traced
sample does not erase the uninstrumented failure. This is not a controlled A/B:
first-run CPU ceilings were not recorded; second observed1.60/1.84GHz and lower
maximum temperature. Different timing, scene and observation can confound results.
No attribution of the improvement to tracing, VA packing, CPU frequency or cooling.

## Kernel trace: what was actually observed

Separate instance, `mono` clock, minor0 QBUF/DQBUF filters; trace data written to
root-private storage after capture, no streaming trace I/O during recording.
Global tracer/events untouched, instance removed, ring overruns0.

1048 QBUF and1048 DQBUF events include capture start/stop preview transitions,
not a one-to-one MP4-frame count. DQ buffer timestamps max40.652ms, no>50ms;
DQ call arrivals max127.007ms,49>50ms,1>100ms. Timestamp-to-DQ-call delay
max114.181ms/mean66.174ms. Thus a passing stored file does NOT establish uniform
live delivery or smooth preview. Current AtomISP timestamps are software buffer
completion, not sensor SOF; no physical sensor conservation claim.

`vb2_v4l2_buf_done` exists in tracefs but produced **zero** filtered events.
Requested7.2 source contains its definition/export but no call sites. Consequently
there is no direct completion-event-to-DQ correlation from this run. Do not turn
an available tracepoint into supposed hardware proof; a future diagnostic can
use actual emitted generic `vb2:vb2_buf_done` with suitable private filtering.
The failing uninstrumented run has no kernel trace, so its556ms stall is not yet
attributed to kernel scheduling, queue starvation or userspace pipeline behavior.

## Local automatic-lock preference and postflight

At the user's explicit request, only Denial `power.idleLockEnabled` changed from
true tofalse using revision-checked native settings API, with private backup and
disk readback. Existing automatic DPMS and automatic suspend were already false
and remain so. All other native settings unchanged; no password modification,
manual-lock disablement or bypass of an existing lock. Observed persisted and
unlocked after315s without agent input injection; human activity was not monitored.
This is a local user preference, NOT a new rootfs/image security default.

Both application runs exited successfully and sensors/ISP suspended with usage0.
Original controls and normal camera settings restored;686 protected installed
files and35 private aliases/bytes unchanged, no kernel fault markers or failed
system units. Stable GRUB fallback retained, no new kernel build or reboot.

Only obsolete self-generated strace/performance MP4 and first900-frame direct
recipe MP4 removed under cleanup authorization; all their traces, metadata, frame
hashes and logs retained, along with stronger off/on/off and normal-app recordings.
Raw recordings, per-frame timing, kernel traces and frame hashes remain private.
Remaining space2.207GB is above the2GiB floor but below the next full-quality
capture reservation; no further recording without a fresh reserve check.

Open work: reproduce the failing normal run with emitted queue/buffer diagnostics,
rear capture and native stills, optical AE/AWB/focus and controlled-scene quality,
preview/audio pacing, longer repeats/reopen/resume, actual7.2 audio/UCM, S0I3 and
cold boot, and failed-reset DMA quiescence. No full image, OEM/Android parity or
completion of the broader kernel/system task claimed.
