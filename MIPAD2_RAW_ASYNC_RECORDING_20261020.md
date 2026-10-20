# R17g real same-binary ASYNC0/1 recording: full candidate still REJECTED

Same private R17g executable/source4a9f455, RAW_DIRECT1/COPY1/GUARD1, old producer
queues0 IN BOTH arms; only ASYNC0/1 and fresh test namespaces differ. Each arm
recorded both physical inputs~30s at native NV12 1920x1080/30, full1280x720/30
GPU-rotated preview, H26415Mbps/stereo AAC192kbps48kHz/VA delay2/default GSK.
New named-only metadata tracer actually loaded and verified in BOTH processes;
source-filterV2 and cooperative CPU-capV3<=1.6GHz used in both, original thermal
sensors/thresholds/hysteresis retained. Stable R19/same boot/default unchanged.

| ASYNC | input | MP4 fps | largest gap | cadence gate | source->encoder max |
|---|---|---:|---:|---|---:|
|0|front|29.5738|170ms|RED|273.195ms|
|0|rear|29.8799|106ms|RED|264.224ms|
|1|front|29.4846|111.667ms|RED|453.249ms|
|1|rear|30.0027|42.666ms|PASS|363.759ms|

Front still fails; the complete candidate is NOT accepted/defaulted. Added
buffering did not establish reduced source->encoder delay, let alone optical
latency. Cold starts were43C/OFF vs55C/ON; peaks71/73 vs76/77C. These sequential,
not randomized/ABA samples do not isolate a unique performance effect. No
discarded first seconds, lower quality, frame/timestamp repair or easiest-side
substitution. A source->encoder arrival metric is not full A/V/display latency.

## Actual complete-frame and topology evidence

Counts903/904 (OFF),878/893 (ON) conserve exact native PTS+offset -> encoder
input/output -> parser -> MP4 count, within one declared MP4 time-base tick.
Every selected packet uniquely joins native dequeue/VB2 completion; no native
push between recorded endpoints was omitted. Full video/audio decode succeeds;
exact decoded-repeat aggregates retained. This does NOT prove sensor exposure
continuity, scene motion/optical quality, physical A/V sync or Android parity.

New strict named-tail analyzer verifies all3578 real frame witnesses, not a
route-flag assumption: actual named raw output on native task for ASYNC0, and
actual named queue sink/source +raw output on its DIFFERENT worker for ASYNC1.
Every per-frame arrival partition conserves source->encoder total. It permits
consumer/encoder execution after native return, rejects missing/ambiguous names,
wrong tasks/order/identities, and does not interpret sink-arrival->worker-start
as isolated queue residence. Actual ASYNC0 matches all seven strict V3 timing
populations; the old synchronous route correctly rejects actual ASYNC1 nesting.
DOT snapshots independently confirm actual queue4/bytes0/time0/silent/no changed
leak-or-EOS-flush settings. Tail timing alone does not prove queue capacity.

Windows, tablet Python and published Linux-source normal/-O tests all38 PASS
(12 new named-tail tests plus previous26); no asserts-disabled validation escape.
No raw media, owner tokens, raw syscall arguments or per-frame pixel digests
are published. Full logs/traces and real clips remain tablet-local.

## Remaining blocking and retained counter caveats

The ASYNC1 native capture task still performs synchronous CAPS queries THROUGH
the named queue to named raw output, reaching217.093ms in recording-anomaly
contexts (OFF212.577ms). An asynchronous BUFFER handoff is not an asynchronous
control-query boundary. Allocation guard itself remains fast/false; downstream
allocation/control calls still exist. Need exact negotiation/control correction,
not another unverified larger queue or a cache that fabricates downstream caps.

All recorded-window zero-driver-owned VB2 counter periods are preserved:
OFF8.430481/5.384629s, ON8.709293/6.373715s. These include many short per-frame
intervals and are NOT private CSS depth or proof of physical starvation/lost
sensor frames. Even the cadence-passing rear has nonzero counter periods.
Do not erase/relabel the original no-zero gate or claim every interval causes
a video gap. Exact overlap partitions/totals stay in scalar evidence; unique
lock/CPU time/physical cause is not established by nested wall times alone.

Existing [GStreamer queue semantics](https://gstreamer.freedesktop.org/documentation/coreelements/queue.html)
promise a source task for data, not bypassed negotiation. The actual query
traces, not queue size by itself, determine the next investigation.

Both arms close normally, real RTC resumes, serial/capture workers terminate,
exact original CPU policies restored before offline analysis, controls/sensor/
ISP PM0 and original software/thermal/GPU/GRUB/anti-idle verified afterward.
OFF completed scalar logs were archived BYTE-IDENTICALLY ON TABLET before ON
preflight; readable root-owned SHM aliases retained, strict reserve not relaxed,
no security sysctl changes/information discard. Full kernel/flash scope remains
OPEN; this is R19 hardware evidence, not R24 reset/DMA/boot qualification.
