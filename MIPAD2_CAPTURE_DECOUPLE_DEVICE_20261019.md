# Same-binary device test: reject copied-source queue candidate

This supersedes the untested-candidate status in the earlier report, not its
measurements. No kernel or launcher default changed; full objective remains open.

## Protocol

Same d7e1a9d/r17c executable, R19 kernel, private companion plugins, default GSK
renderer, H264 15 Mbps / AAC 192 kbps 48 kHz stereo, native 1080p30 and full
720p30 GPU preview. Content tracing off. Both arms copy native frames; only
MIPAD2_CAPTURE_DECOUPLE changes 0 to 1. Native pool indices are 0..3 for both
cameras/arms. Two 30-second clips per arm, cooldown before each pair. This is
sequential, not randomized or temperature-matched; do not infer a population
effect size from one run. No recording warm-up frames excluded.

| Queue | Sensor | FPS | Maximum MP4 gap | Packets/decode | Peak C |
|---|---|---:|---:|---:|---:|
| control | front | 29.966082 | 54.667 ms | 911/911 | 72 |
| control | rear | 29.975848 | 71.333 ms | 907/907 | 73 |
| enabled | front | 29.721135 | 54.000 ms | 892/892 | 73 |
| enabled | rear | 29.773907 | 115.000 ms | 882/882 | 77 |

All four clips fail the strict <50 ms maximum-gap gate. Audio/video decode
completely, with zero consecutive identical decoded frames. The enabled queue
is NOT recommended or placed in the normal launcher.

## Capture-side evidence, not guessed clock matching

Read-only driver diagnostics bridge exact adjacent native/encoder PTS+offset
to raw driver stamps and uniquely associated indexed VB2 completions. Only
those exact matches are called recording-confirmed. Two additional enabled-arm
sparse anomalies have no adjacent encoder match: retained as unconfirmed, not
relabeled recorded losses. Raw traces/owner tokens remain on-device.

control: driver gap 54.736 ms / driver-owned-zero overlap 32.082 ms, driver gap 71.226 ms / driver-owned-zero overlap 48.485 ms.
enabled: driver gap 53.909 ms / driver-owned-zero overlap 30.826 ms, driver gap 103.445 ms / driver-owned-zero overlap 80.891 ms, driver gap 114.800 ms / driver-owned-zero overlap 91.983 ms.

The queue really activates. All 3519 queue input/output scalar keys match
uniquely, no missing/ambiguous pairs. Sink-chain entry to src-push entry p95
309.899 ms, maximum 327.496 ms. Matched admitted-but-not-dispatched peak 9
includes enqueue/scheduling time: it is NOT the GObject queue occupancy counter,
nor optical/AV latency. Eight queued buffers plus a blocked admission is bounded
but not a cure for downstream pressure. Native push p95/max are 2.924/18.077 ms
without queue and 35.555/43.833 ms with queue. Exact source wait site remains
unproven; do not inflate kernel buffer minima or hide gaps with timestamp repair.

## Actual full-clip conservation

Real encoder-source EOS epochs delimit recordings, not guessed pauses. Every
encoder input has a unique exact native offset/PTS predecessor. Per clip, input,
encoder output, both recording parser outputs, MP4 packet count and full decode
count agree (911/907 off; 892/882 on). Relative encoder PTS match input exactly;
MP4 residual is below one declared 1/3000-second mux tick. This is not a fitted
kernel clock tolerance, a guarantee of all preview frames being recorded, or
physical capture-time/AV-sync/image-quality proof. The missing intervals exist
before encoding. Thirty-four preview/thumbnail calls remain right-censored at
each app exit; complete trace rings do not mean every lifecycle call balanced.

## Follow-up source-stage timer (private, not a fix)

A separate GStreamer 1.28.7 V4L2 plugin times negotiate, decide-allocation,
create, parent allocation, buffer-pool processing and metadata/control phases.
MIPAD2_V4L2_WAIT_DIAG=1 is required. No buffer/caps/PTS/flow changes; only scopes
over 40 ms emit, capped at 128 lines per process. Cleanup uses an initialized
sentinel before any goto error exit. The first compiled variant was never
installed; v2 corrects that initialization hazard. Actual GStreamer-object host
tests cover disabled output, fast scopes, slow/early-return scopes, error-before-
metadata cleanup and the 128-line cap. Cold start is not recording attribution.

Follow-up cycle 0: driver gap 79.517 ms; matching slow scopes overlapping the verified zero-owned window: ; an empty list does not establish a leaf cause.

## Recovery, retained failures and unfinished work

All own camera/trace/serial/build jobs finish; trace loss counters zero, instances
removed/global state restored, sensor controls and PM0 restored. Post-pair RTC
S0I3 increments are 9.500 s off and 9.547 s on. Earlier zero-S0I3/thermal failures
remain. Protected apps/plugins, GRUB, GPU bounds, anti-idle and R19 boot unchanged.
Only six explicitly approved older diagnostic MP4s were removed; logs, photos,
current buffer/copy/queue/EOS samples and kernels retained. Btrfs delayed frees
were observed after sync rather than falsely claiming immediate space recovery.

A host-only sparse-tail NameError and generator/signature failures were retained
and repaired without rerunning completed recordings or overwriting measurements.
No media, cores, individual pixel digests, raw owner tokens or kernel traces
published. The evidence file contains aggregate metadata and source hashes only.

Open: source-task control-plane/event/scheduling wait location, kernel failed-reset
DMA ownership fence and buffer/parameter lifetime, Mesa/GstGL stop crash, full
photo/flush/reopen/foreground UX, AF/WB/exposure/physical AV sync, and full Linux
6.14 kernel/flash Android/Windows parity. This is not camera-only completion.

Primary references: [GStreamer queue behavior](https://gstreamer.freedesktop.org/documentation/coreelements/queue.html),
[VB2 ownership API](https://docs.kernel.org/driver-api/media/v4l2-videobuf2.html),
[matching release wrapper source](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/gst/camerabin2/gstwrappercamerabinsrc.c).
The first describes bounded blocking/separate source tasks, not guaranteed hardware
efficacy; the last is a source-inspection lead, not proof of a particular wait.
