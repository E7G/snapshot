# Native capture stalls on serialized allocation query; opt-in boundary candidate

This follows the rejected source-queue experiment. No kernel/launcher defaults
changed. New behavior below has host tests only, NOT hardware efficacy proof.

## Real full-quality capture and actual SOURCE task evidence

The same R17c binary runs COPY1/DECOUPLE0 at native1080p30, preview720p30 GPU,
H26415Mbps/AAC192k48k2ch. Front29.936716fps/max93.333ms, rear30.013899/max57.667ms:
both cadence RED. All914/908 video frames decode, no consecutive exact repeats,
audio decodes. Actual encoder/EOS/parser/MP4 counts914/908 agree; exact input to
encoder relative PTS and unique native lineage, mux residual below one declared
1/3000-second tick. No warm-up excluded. This does not establish physical AV sync.

A new private ABI2 scalar tracer uses EXPLICIT ABSOLUTE monotonic time, not the
old relative hook timestamp. It pairs event/query/state PRE/POST calls. Matching
1.28.7 hook signatures verified against primary source; actual host GStreamer
tests detect injected60ms event/query delays, preserve12 test buffers and reject
overflow. Device trace338816 records/226 objects complete. An own kernel trace
adds scheduler/syscalls for the actual source TID, validated by executable hash,
process environment, comm and Tgid, not just ISP IRQ thread528. Kernel170087
records, no losses;3531 unique successful owner joins,missing0/ambiguous0.

| Camera | Allocation query | Source FUTEX_WAIT_PRIVATE | Source sleep | Wake-to-run | Driver gap | Zero-owned overlap with query |
|---|---:|---:|---:|---:|---:|---:|
| Front |193.957ms|193.907ms|193.854ms|0.358ms|93.367ms|68.189ms|
| Rear |163.528ms|163.510ms|163.461ms|0.037ms|57.751ms|32.780ms|

The source thread waits inside an ALLOCATION query on decodebin3/multiqueue sink;
both eventually return FALSE. This is observed serialized-query sleep overlapping
independently verified native buffer depletion, not guessed CPU starvation.
The earlier source-method timers missed it because the call is outside those
subclass method scopes. We do not claim the identity of every internal mutex.

IMPORTANT: a separate front50.634ms driver gap has ZERO driver-owned-zero overlap;
the earlier allocation query is only in its extended context, not the actual gap.
Do not attribute that gap to this query or pretend a single complete root cause.
Forty-two thumbnail/preview calls remain right-censored at exit; one syscall exit
is left-censored and one entry right-censored at filter/capture boundaries. No
invented completion durations or pointer arguments exported. Post-camera RTC
S0I3+9.851136s/IRQ9; prior zero-S0I3/thermal/stop failures remain retained.

## Minimal new functional candidate: allocation boundary after full copy

Requires ALL MiPad2 DMI + physical v4l2src + MIPAD2_CAPTURE_COPY=1 +
MIPAD2_CAPTURE_ALLOCATION_GUARD=1. Defaults unchanged. A named identity element
immediately AFTER the exact native copy probe, before any optional queue/caps/
decodebin3, sets drop-allocation=true. It returns FALSE for allocation locally,
not a fake successful allocation, so native capture can use its own pool rather
than wait for downstream multiqueue. Full copies isolate downstream memory leases.
No buffers, CAPS, timestamps or EOS discarded/repaired, no lower caps/fps/bitrate,
no recording-start delay or ignored warmup. Keep rejected DECOUPLE flag OFF.

Actual Rust/GStreamer tests prove scope gating, a real blocked downstream query
negative control versus no forwarded allocation with boundary, CAPS passthrough,
and full12-buffer copy+boundary byte/PTS/duration/offset preservation plus real EOS.
Existing padded NV12 layout/meta/native-lease tests remain. Focused7 tests pass;
release/all-features/all-targets workspace67 tests pass. A compile namespace error
was repaired with gst::glib::BoolError; failed log retained, not called a pass.

The candidate is NOT yet privately packaged/installed/hardware-tested. Require
full-profile device allocation-query/driver-availability/cadence/conservation,
no copy failures, memory/thermal, stop/reopen/switch/photo/flush/AV/foreground/PM
gates before promotion. If copy allocation fails, original frame is retained/logged
and isolation is unproven: never silently drop it. Full kernel/flash parity and
kernel DMA-fence/Mesa stop safety remain separate unfinished requirements.

Primary references: [identity allocation boundary](https://gstreamer.freedesktop.org/documentation/coreelements/identity.html),
[matching hook ABI and relative timestamp origin](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gstreamer/gst/gsttracerutils.h).
They specify behavior/signatures, not tablet efficacy. No private media, cores,
individual pixel hashes, raw kernel traces or syscall pointer arguments published.
