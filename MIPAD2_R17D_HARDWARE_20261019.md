# R17d native allocation boundary: same-binary hardware repeat

Candidate remains opt-in; no kernel, bootloader or default launcher changes. This supersedes the earlier host-only status, not its retained negative evidence.

## Fixed protocol

Same R17d executable SHA256 `3cedebfad142c80a6f21c53630fe2ae24afb9b5f7a7ae53f3fed58aefa2e92f6`; identical private plugins and full-quality launchers except namespace, guard flag and expected activation assertion. Native recording1920x1080/30, GPU preview1280x720/30, requested H26415Mbps/AAC192kbps48kHz stereo, default GSK renderer, VAasync2, COPY1/DECOUPLE0/CONTENT0. Each arm records both cameras for30s, with no excluded warmup or deferred recording start. Read-only cooldown <=55C and >2GiB root reserve before the repeated arms;80C stop ceiling/1GiB reserve during capture. Sequential on/off/on, NOT randomized.

| Run | Camera | fps | max PTS gap ms | Frames | Driver-owned-zero overlap ms | Peak C | Cadence |
|---|---|---:|---:|---:|---:|---:|---|
| first_on | front | 30.000650 | 34.334 | 925 | 0.000 | 73 | PASS |
| first_on | rear | 30.010788 | 34.000 | 919 | 0.000 | 73 | PASS |
| off | front | 28.479626 | 124.667 | 846 | 8879.659 | 76 | FAIL |
| off | rear | 29.914909 | 153.667 | 901 | 131.101 | 79 | FAIL |
| repeat_on | front | 28.668578 | 49.000 | 861 | 8835.777 | 76 | FAIL |
| repeat_on | rear | 30.013295 | 42.333 | 904 | 0.000 | 79 | PASS |

## Independent checks

Exact source PTS/offset and successful same-TID ordered DQ fences -> unique indexed/stamped VB2 ownership/completion. Availability scope first..last RECORDED BUFFER COMPLETION, not optical SOF or the entire button-intent interval.

Actual encoder-source EOS epochs, input/output/both recording parsers/MP4/decode counts and exact native lineage are checked. Encoder relative PTS must match exactly; mux tolerance is one declared1/3000-second tick, not fitted clock drift. Full video/audio decode and aggregate exact-repeat checks run on-device after capture stops. These do not prove physical A/V alignment.

- first_on: source ALLOCATION copied-native-identity: count13, max0.010300ms, results{'0': 13}. Kernel records170219, owner joins{'unique': 3520, 'ambiguous': 0, 'missing': 0}; loss gateTrue. ConservationTrue, all recorded packets joinedTrue. Copy failures0. RTC S0I3+9.247424s, functional resumeTrue. Pacing/control right-censored calls34/42; never called all-calls-balanced.
- first_on exact recorded-endpoint source coverage: cycle0: native925/encoder925, omitted0, driver30.000690fps; cycle1: native919/encoder919, omitted0, driver30.010895fps. The residual mean-fps deficit is already present in driver completion stamps, not an MP4-only drop.
- off: source ALLOCATION decodebin3-multiqueue: count9, max477.967802ms, results{'1': 7, '0': 2}. Kernel records169782, owner joins{'unique': 3499, 'ambiguous': 0, 'missing': 0}; loss gateTrue. ConservationTrue, all recorded packets joinedTrue. Copy failures0. RTC S0I3+9.267904s, functional resumeTrue. Pacing/control right-censored calls38/47; never called all-calls-balanced.
- off exact recorded-endpoint source coverage: cycle0: native846/encoder846, omitted0, driver28.479548fps; cycle1: native901/encoder901, omitted0, driver29.914789fps. The residual mean-fps deficit is already present in driver completion stamps, not an MP4-only drop.
- repeat_on: source ALLOCATION copied-native-identity: count13, max0.004012ms, results{'0': 13}. Kernel records171798, owner joins{'unique': 3518, 'ambiguous': 0, 'missing': 0}; loss gateTrue. ConservationTrue, all recorded packets joinedTrue. Copy failures0. RTC S0I3+9.275264s, functional resumeTrue. Pacing/control right-censored calls34/42; never called all-calls-balanced.
- repeat_on exact recorded-endpoint source coverage: cycle0: native861/encoder861, omitted0, driver28.668664fps; cycle1: native904/encoder904, omitted0, driver30.013143fps. The residual mean-fps deficit is already present in driver completion stamps, not an MP4-only drop.

## Residual source-thread buffer push blocking

For every encoded frame, unique exact PTS/offset and same-TID nested intervals independently qualify the actual source push, named guard sink chain and decodebin3/multiqueue sink buffer chain. These are buffer push/chain durations, not ALLOCATION query durations. No syscall parser is needed for this ABI2 control timing. Durations are nested/nonadditive; prefix includes copy probe plus pad/tracer overhead, not isolated memcpy CPU time.

| Run/camera | Native push median ms | Pre-guard prefix median ms | Multiqueue chain median ms | Frames |
|---|---:|---:|---:|---:|
| first_on/front | 2.967732 | 2.713372 | 0.074826 | 925 |
| first_on/rear | 2.092168 | 1.899831 | 0.046764 | 919 |
| repeat_on/front | 34.900966 | 3.834421 | 30.665193 | 861 |
| repeat_on/rear | 32.990719 | 4.032398 | 28.344867 | 904 |

Repeated ON source ALLOCATION is fast, but native push still blocks in the downstream multiqueue BUFFER chain: this is a separate remaining backpressure path, not evidence that the allocation guard was not enabled. Whole45s sampled CPU-frequency medians differ (~1.84GHz first ON versus ~1.60GHz repeat ON); retain thermal/order confounding and do not label hardware throttling proven. A new bounded-copy scheduling fix needs separate host/device tests; the rejected8-buffer queue must not simply be promoted.

## Scope and next work

Sequential nonrandomized on/off/on; temperature/order/workload confounds retained. Allocation maxima include startup/preview unless a recording case is separately exactly qualified. Preview/transition native gaps and right-censored lifecycle calls remain; no general all-lifecycle or whole-device parity claim.

The repeated ON front camera fails the mean-fps gate even if its individual gaps remain below50ms. Preserve that failure; removing long allocation waits is not a complete sustained-cadence fix. The repeat-ON strict syscall parser also rejected a repeated entry before an exit; that failed log is retained. Its independent control-only-v2 result provides allocation-query timing, NOT validated syscall durations or fabricated lock-wait explanations.

Keep the guard opt-in until photo/full-resolution, repeated reopen/switch/flush, foreground recording/rotation, physical A/V alignment and long-run memory/thermal gates are independently met. Kernel SP/CSS DMA reset ownership fences, parameter/buffer lifetime and Mesa/GstGL stop safety remain open; camera cadence alone cannot complete the Linux6.14/flash integration goal.

Official behavior: [identity drop-allocation](https://gstreamer.freedesktop.org/documentation/coreelements/identity.html) and [matching1.28.7 query implementation](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gstreamer/plugins/elements/gstidentity.c). The identity rejects ALLOCATION locally with FALSE; it does not drop image buffers. These sources specify API behavior, not tablet efficacy.

Allowlisted scalar timing/counts/code hashes only. No real media, cores, per-frame pixel hashes, raw kernel owner tokens, raw traces, syscall pointer arguments or private media filenames.
