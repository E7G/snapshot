# R17g opt-in native raw asynchronous capture boundary — host validated only

R17f direct RAW1 retained COPY1/GUARD1 but removed decodebin3/MQ's worker boundary.
Actual full-quality recordings failed cadence and had zero-driver-owned periods;
see MIPAD2_RAW_DIRECT_RECORDING_20261020.md. Removing asynchronous handoff was
not equivalent to removing unnecessary decoding work. The default remains OFF.

## Actual source change

With validated physical AtomISP /dev/video0 system-memory NV12, COPY1/GUARD1,
producer DECOUPLE/SMALL queues both OFF and RAW_DIRECT1, an ADDITIONAL explicit
MIPAD2_CAPTURE_RAW_ASYNC=1 inserts exactly one named queue in the raw tail:

    v4l2src -> existing deep-copy probe -> allocation guard -> native caps
            -> mipad2-native-raw-queue -> mipad2-native-raw-output -> existing consumers

Queue limits: four QUEUED buffers, bytes/time limits disabled, nonleaky,
flush-on-eos=false, silent=true. This is not an unbounded queue, decoder/MQ
replacement with hidden fallback, extra per-frame copy, quality reduction,
timestamp rewrite, frame drop or EOS truncation. Downstream preview/recording,
H264/AAC profiles, actual geometry/rates/rotation and default path are untouched.
Existing unbuffered RAW_DIRECT1 remains available as the same-binary control.
Invalid ASYNC1 without validated DIRECT1 returns an error, never silently falls
back to legacy producer queues. Non-MiPad2/default operation stays unchanged.

Four queued copied frames (~12 MiB for tightly packed 1080p NV12; padding may
increase this) do NOT mean four total frames: the worker and blocked producer
can each own another copy. No scarce native MMAP lease is intentionally held
by this copied queue. The number is a bounded experimental capacity, NOT a
guarantee against arbitrary stalls or a measured optical-latency target. A
full queue still applies backpressure; CAPS/other synchronous query waits may
still block capture and need independent correction. Do not claim parity yet.

## Host evidence on the actual wired source

17 focused tests and all87 workspace tests pass. They cover explicit policy,
one named/nonleaky/draining queue/no decoder, native-copy release before slow
consumer (first consumer + four queued + blocked producer all hold copies),
exact full payload/order/timestamps, new worker task, padded NV12 metadata,
all12 frames before EOS, real downstream error propagation, allocation/latency
query semantics, video/front full-photo/rear full-photo/reopen, serialized CAPS
ordering while old-geometry frames are queued, flush cancellation and no stale
epoch buffers. The final test binary passes100 repeats of both queue flush
tests (200 cases), not just a flaky single run.

Initial compiler failure in an enum-inspection TEST used owned Glib EnumValue
instead of borrowed &EnumValue; fixed without production behavior changes.
The first full workspace run also exposed a PREEXISTING small-queue flush-test
race: expected producer index3, observed4/Flushing. Original failure retained.
[GstQueue1.28.7 source](https://raw.githubusercontent.com/GStreamer/gstreamer/1.28.7/subprojects/gstreamer/plugins/elements/gstqueue.c)
forwards FLUSH_START before setting srcresult=FLUSHING; the cancelled consumer
can free one slot in between. The test now allows ONLY index3 or4, still demands
Flushing, exact interrupted-index+1 native release accounting, unchanged1s
deadline, no old queued-frame delivery and all12 new-stream frames. The new
four-frame test similarly permits only the corresponding index5/6 interleaving.
This corrects the test specification, not a capture-copy implementation change.

The [official queue semantics](https://gstreamer.freedesktop.org/documentation/coreelements/queue.html)
document its separate source task, bounded producer blocking, and that enabling
flush-on-eos would discard tail data; that property stays false here.

No new binary installed or hardware recording claim at this source publication.
Required next: same-binary ASYNC0/1 real full-quality double-camera recordings,
source/driver availability and source/encoder/MP4 conservation, correct trace
coverage, full-photo/reopen/suspend tests and unprofiled foreground validation.
Original R19 device/defaults remain; R24 kernel is built but not deployed.
The complete Linux kernel/flash/Android-Windows parity objective is OPEN.
