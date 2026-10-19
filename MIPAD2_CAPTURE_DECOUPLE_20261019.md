# Buffer depletion measured; opt-in copied-source queue candidate

This is an experimental source-thread decoupling candidate, NOT a verified tablet
fix, new kernel or default promotion. Full goal remains unfinished. It follows
[IRQ timing evidence](MIPAD2_KERNEL_PACING_20261019.md) on unchanged Mi Pad2 R19.

## Actual standard kernel queue metadata

Own trace instances add standard V4L2/VB2 QBUF,DQBUF,queue/done timestamps,buffer
indices and driver-owned counters to the already filtered IRQ/scheduling capture.
Only scalar metadata, no payload or pointer dereferencing. A successful indexed
dequeue join uses PID,index,raw timestamp (timeval truncation accounted for) and
bounded event ordering. Baseline3502/control3508 joins unique,ambiguous0,missing0;
two owner queues each. Raw owner tokens and kernel traces stay private. All ring
overrun/commit-overrun/dropped counters0, own instances removed/global state restored.

The baseline two30s recordings are front30.006170fps/max34ms and rear30.014339fps/
max45.667ms, peak69C. All video/audio decode. Native pools show7 and6 buffer indices.
Some >40ms source-kernel candidate gaps overlap driver-owned0 periods22.368ms/
19.303ms; camera switches/preview are NOT automatically MP4 failures. A broad
kernel-to-native clock/order lineage attempt fails a single-domain assumption at
camera switch (~59.778s), and its per-epoch100us fit also fails (241860ns residual).
These failures are retained: no nearest-frame or relaxed-threshold whole-stream
lineage proof is substituted. Baseline recording gate passes but does not erase
earlier intermittent recording failures.

## Copy-only control is a real negative result

Only existing MIPAD2_CAPTURE_COPY changes0->1; same R17b binary/kernel/private
companion plugins,1920x1080/30 native,1280x720/30 preview,H26415Mbps,AAC192k48k2ch.
Actual activation log present, copy failure logs0. Sequential conditions differ,
and pool allocation changes7/6 ->4/4; this is NOT a fixed-pool randomized A/B.

Front29.970128fps/max67.666ms and rear30.012992fps/max62.333ms are both cadence RED,
peak67/69C;924/902 frames decode with no consecutive exact repeats, audio decodes.
These recording anomalies are independently tied by source/encoder exact metadata
and sparse V4L2 driver diagnostics to kernel before/after completed-buffer stamps.

| Recording | Driver timestamp gap | Driver-owned0 interval inside gap |
| --- | ---: | ---: |
| Front | 67.587ms | 44.746ms |
| Rear | 62.151ms | 39.539ms |

Here all four buffer indices complete, driver-owned count reaches0 while queued
count remains4, then successful DQBUF/QBUF resumes. Thus copying frame memory alone
does NOT keep the capture source pumping when its source thread stops dequeuing.
Steady-state requeue p99 improves to3.121/3.642ms, but the startup failure remains.
We do not claim a unique blocking primitive, sensor loss, hardware fault or a
fixed thread scheduling/negotiation cause; the exact source wait site is unresolved.

## New source candidate: copy THEN bounded non-leaky queue

Existing source path is unchanged unless ALL opt-in conditions hold:
Mi Pad2 + native v4l2src + MIPAD2_CAPTURE_COPY=1 + MIPAD2_CAPTURE_DECOUPLE=1.
The new queue is directly after the native source's exact deep-copy pad probe,
before capsfilter/decodebin/camerabin. A separate streaming task lets source
dequeues proceed while downstream prepares recording. Eight copied frames bound
the reservoir (~25MB for observed padded1080p NV12); byte/time limits disabled so
the specified frame budget, not an implicit10MB/1s default, governs it. Leaky=no.
No lower caps/fps/bitrate, discarded warm-up, duplicated frame or forced-OK flow.
The queue cannot eliminate unbounded backpressure; filling it, allocation failures,
latency, renegotiation/flush/photo and physical A/V/foreground behavior need real
device validation. Copy allocation failure still logs and retains the original
frame rather than silently dropping it; such a test cannot prove lease isolation.

Actual compiled Rust/GStreamer tests compare a blocked-consumer negative control
against this bounded queue. Without queue at most one synthetic native lease
releases; with queue at least eight release before consumer unblocks. Afterwards
all twelve exact synthetic frames/PTS arrive in order and EOS completes. Existing
full padded NV12 byte/layout/timing/deep-copy lease test still passes. Source gating
and buffer/time/byte limits are tested. Full workspace release/all-targets/all-
features64 tests pass. These are real pipelines, not a decision-model simulation.
They are NOT device cadence, thermal, image-quality or Android/Windows parity proof.

## Deployment and broader gates

The new binary has not yet been packaged/installed/hardware-tested. Default r6,
installed R17b, stock plugins, GRUB/GPU limits and kernel are untouched. Candidate
flags are private-test-only. Source files changed only in aperture capture-copy
and viewfinder integration; protected prior artifacts remain. Both original and
copy-only recording runs close gracefully, restore controls/sensor PM0, preserve
ordinary settings and stop serial/trace units. RTC deep-residency results and
earlier zero-S0I3 failures are retained in the scalar evidence. No media, cores,
individual pixel digests or raw kernel/owner-token data are published.

Next build a private distinct-prefix candidate and require measured native queue
availability, full-clip/input/output conservation, preview30fps, no copy failures,
bounded queue depth/latency, cold/reopen/switch/stop/flush/photo/AV-sync/PM and visible
UX gates before any promotion. Kernel SP failed-reset DMA fence, Mesa stop crash,
AF/WB and full kernel/flash Android/Windows parity remain open. Do not replace
that objective with this small candidate or report the system fully perfected.
