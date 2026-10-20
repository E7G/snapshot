# R17f same-binary full-quality RAW0/RAW1 recording: direct candidate rejected

Actual device: deployed Linux 6.14 integrated R19, same boot and private R17f
binary (SHA in adjacent scalar evidence). No new kernel/default/package change.
Each route recorded front/rear ~30 seconds: native NV12 1920x1080/30,
1280x720 preview, GPU rotation, H264 requested 15 Mbps, stereo AAC 192 kbps/
48 kHz. COPY1/GUARD1, source queues0, content tracing0, unchanged VA delay2.
Both arms used the same cooperative thermal-cap V3 worker (<=1.6 GHz); existing
sensors/thermal thresholds/hysteresis remained intact. Exact original policies
and protected software/default/thermal/GPU/GRUB state verified after recording.

| route | camera | MP4 fps | largest PTS gap | driver-owned-zero time |
|---|---|---:|---:|---:|
| RAW0 decodebin3/MQ | front |30.0003|34 ms|0|
| RAW0 decodebin3/MQ | rear |30.0102|34 ms|0|
| RAW1 direct | front |29.6338|183.333 ms|450.859 ms|
| RAW1 direct | rear |29.9145|106 ms|187.799 ms|

The direct route FAILS the unchanged full-clip cadence gate. It is NOT promoted.
RAW0 was morning, RAW1 afternoon (~3-hour gap after an SHM preflight abort and
byte-identical tablet-local trace archival). These are sequential, not randomized
or a unique-cause intervention. Do not infer Android/optical parity or a universal
RAW0 guarantee from these two clips. No trimmed warmup or timestamp repair.

## What the real traces establish

All four clips have exact native PTS/offset -> encoder input/output -> parser
conservation and MP4 packet counts, within one declared MP4 time-base tick.
Counts: RAW0 897/915, RAW1 888/921. Video/audio decode succeeds; aggregate exact
decoded repeats0. Every native push between actual recorded endpoints was
forwarded. Thus no observed post-native app/encoder/mux discard explains these
specific RAW1 gaps; this is NOT a physical exposure/frame-loss measurement.

Every recorded packet joins a unique successful V4L2 dequeue and same-owner
VB2 completion. RAW1 has six/front and ten/rear zero-driver-owned intervals.
Exact temporal partition, milliseconds (not per-frame attribution/CPU time):

| camera | inside source guard chain | source push outside guard | outside source push, control calls | other |
|---|---:|---:|---:|---:|
| front |263.678091|16.855922|169.020740|1.304247|
| rear |68.703574|43.745952|74.023614|1.325860|

Selected RAW1 source pushes reach137.380/123.255 ms and guard downstream calls
127.550/120.431 ms; RAW0 guard maxima30.046/1.861 ms. Control traces include
long caps/event/allocation calls and native-task futex waits; those nested wall
times do NOT name a unique mutex or prove which negotiation is redundant.
At the existing guard ALLOCATION itself returns false in <=4.3 us for RAW1;
downstream source-task allocation queries still reach167.95 ms. Removing MQ
removed an asynchronous boundary as well as bookkeeping. The next design must
preserve capture/consumer decoupling and correct query/event semantics, not
simply delete buffering or drop full-quality frames.

## Measurement coverage failure retained, not fitted away

The strict V3 direct-output analysis FAILED: the old control tracer's factory
whitelist never recorded GstCapsFilter, including mipad2-native-raw-output.
V3 intentionally still rejects that absent witness. It has 18 validation tests
and matches all12 previous V2 scalar fields on four REAL legacy MQ clips.
A distinct 8-test common source-boundary analyzer uses only actually observed
source/guard/encoder calls; it does not invent the unseen tail or route proof.
Both suites pass normal and optimized Python on Windows/Linux (26 each).

New gstmp2control_named.c adds ONLY our named GstCapsFilter to the existing
whitelist, retains ABI2, bounded mmap/overflow rejection and metadata-only hooks.
Self-contained Linux host tests compile with -Wall -Wextra -Werror: original
tracer misses the witness; new tracer captures all12 named pushes with exact
source identity/nesting; unrelated capsfilter remains untracked; real delayed
event/query and overflow-negative tests pass. This tracer is NOT installed or
hardware-qualified yet. Original tracer/binary/traces remain unchanged.

Run from contrib/mipad2/source-control:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest -v test_mipad2_frame_arrivals_v3 test_mipad2_source_boundary
PYTHONDONTWRITEBYTECODE=1 python3 -O -m unittest -v test_mipad2_frame_arrivals_v3 test_mipad2_source_boundary
PYTHONDONTWRITEBYTECODE=1 python3 test_named_control_host.py
```

The [official multiqueue documentation](https://gstreamer.freedesktop.org/documentation/coreelements/multiqueue.html)
describes bounded producer blocking. [decodebin3](https://gstreamer.freedesktop.org/documentation/playback/decodebin3.html)
constructs dynamic decoding/stream-selection pipelines. Those semantics plus
our actual traces motivate retaining an explicit capture boundary; they do not
prove a new bounded-queue implementation will pass. Real imagery, raw syscall
arguments, owner tokens and per-frame pixel digests stay tablet-local.

Full Linux kernel/flash objective remains OPEN, including recovery/DMA fencing,
unprofiled foreground smoothness, recording latency, all PM conditions and
Android/Windows image-quality parity. R24 host/full-build evidence is separate;
these R19 recordings do not validate that candidate kernel.
