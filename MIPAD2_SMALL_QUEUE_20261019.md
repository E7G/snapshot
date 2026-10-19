# Guarded two-buffer copied-source scheduling candidate (R17e)

Candidate only; default OFF. Not hardware-tested or promoted. This is not a
resurrection of the rejected8-buffer queue. See [the real repeated R17d hardware
results](MIPAD2_R17D_HARDWARE_20261019.md) for the still-failing front28.668578fps.

## Evidence and functional change

R17d stops serialized source ALLOCATION waits (repeat13 FALSE queries/max4.012us),
but actual source BUFFER push still blocks inside decodebin3/multiqueue sink.
Every encoded frame has a unique exact PTS/offset/same-TID nested match. Repeat
front native-push median34.901ms, pre-guard copy/probe prefix3.834ms, multiqueue
buffer-chain30.665193ms; rear multiqueue-chain28.344867ms. First ON multiqueue-chain
medians0.074826/0.046764ms. These nested wall times are not additive CPU/memcpy
measurements. Sampling/order/power confounds remain; do not infer unique hardware
throttling or blame the allocation guard for the independent buffer backpressure.

The new explicit MIPAD2_CAPTURE_DECOUPLE_SMALL=1 adds a2-buffer nonleaky queue
AFTER the full native-copy probe and allocation guard, BEFORE caps/decodebin3.
It requires Mi Pad2 + physical v4l2src + COPY1 + ALLOCATION_GUARD1. It schedules
downstream push on a different task so native copy/dequeue can overlap a blocked
downstream chain. It cannot improve throughput beyond the actual consumer.
The rejected MIPAD2_CAPTURE_DECOUPLE=1 legacy8-buffer mode remains separate;
requesting both is an explicit error, never silently falls back to the old queue.
An explicitly requested small queue without required copy/guard is an error for
the native tablet source. Other devices/factories and all existing defaults are
unchanged. No extra renderer override, lower caps/fps/bitrate or startup delay.

The queue bounds queued copies to2; separate producer/consumer and other pipeline
in-flight buffers exist. This is NOT an entire-pipeline memory/latency bound.
No leaking, prefill threshold, timestamp rewriting or EOS-tail discard. Full
queues still backpressure intentionally; this is not an unlimited throughput fix.
Copy-allocation failure still retains/logs the original frame, never silently drops
it, but invalidates native-lease isolation proof for that run.

## Actual host tests

Current source compiles. Focused10 and full release/all-features/all-targets
workspace70 tests pass. Config combinations prove scoping, required guard/copy,
default-off and mutual exclusion. Real GStreamer negative control versus2-buffer
pipeline proves separate consumer/source tasks, exactly4 source pushes with first
consumer blocked (one consumer, two queued, one producer), queue-level2 and native
lease release. It then drains all12 padded NV12 frames, preserving every byte,
VideoMeta strides/offsets, PTS/DTS/duration/offset/flags and real EOS. Allocation
queries before frame push reach the real boundary and return FALSE, never the
downstream consumer. Existing actual slow-query/CAPS/full-resolution-copy tests
remain. No real camera data is used for host pixel comparisons.

A real FLUSH_START reaches a responsive blocked mock consumer, returns FLUSHING
to the full-queue producer in less than1s, releases native leases, then FLUSH_STOP
and a new stream deliver all12 new frames and EOS without old queued tail leakage.
This proves the host flushing protocol with an interruptible consumer, not all
real hardware consumers or camera close/crash safety.

Retained test history: the initial parallel allocation-query expectation while a
buffer held the stream lock was invalid for the actual source-thread sequence.
The test now issues the real source query before frame push; no production query
behavior was weakened to pass it. A first flush test's5s timeout was insufficient
liveness evidence; a stronger test exposed an uninterruptible user handoff mock.
The mock now honors FLUSH_START explicitly, with EVENT_FLUSH subscribed separately
from EVENT_DOWNSTREAM as required by GstPad docs. Earlier failed/weak receipts
remain; only the current V6 actual source/logs support the70-test claim.

## Source-filter installation correction

Offline audit of the already captured R17d repeat trace finds all10 repeated raw
syscall entries BEFORE that source's explicit MP2_SOURCE_TASKS commit marker,
69.336..101.974ms before commit. After per-source commit19276 pairs are valid,
duplicate entries0/mismatched numbers0, left-censored exits2/right entries2.
This supports filter-install boundary censoring; it is not complete-global pairing
and does not invent durations for missing records. The original failed strict
analyzer and independent control-only result remain unchanged.

New contrib/mipad2/source-control/mp2_source_filters_v2.py installs exits before
entries, retains old TIDs, verifies readback, and publishes its commit marker only
after every filter succeeds. Its canonical4 host tests include the actual old-order
negative control and bad-readback/no-commit behavior. The helper has not yet been
used in a fresh device capture. Pair future raw syscalls only after each real
source commit marker, with explicit boundary censoring.

## Still required before promotion

Fresh full native1080p30/preview720p30/H26415Mbps/AAC192k48k2ch device recordings,
all source/native/encoder/MP4 conservation, no native-pool starvation, latency and
physical A/V sync, photo/full-resolution/reopen/switch/flush/foreground rotation,
memory/thermal/PM and actual stop-crash gates. Root reserve>2GiB, recording reserve
>1GiB, <=55C cooldown,80C ceiling, serial/SSH recovery and protected stable defaults
remain required. No hardware pass may be inferred from these host tests.

Full Linux6.14/kernel+flash parity remains open, including kernel SP/CSS reset DMA
fences and buffer/parameter lifetimes, Mesa/GstGL stop safety and broader hardware.
No private media, raw traces, per-frame pixel hashes or syscall pointers published.

Primary references: [queue scheduling/bounds/EOS](https://gstreamer.freedesktop.org/documentation/coreelements/queue.html),
[scheduling](https://gstreamer.freedesktop.org/documentation/additional/design/scheduling.html),
[explicit EVENT_FLUSH probe mask](https://gstreamer.freedesktop.org/documentation/gstreamer/gstpad.html).
They define contracts, not tablet efficacy.
