# Mi Pad 2 R17i one-buffer input queue: rejected, production binding retired

Actual2026-10-21 device, same R26 boot, built kernel6074419168fbb70aa5685e976874da6cf86d1fae.
Candidate code2512e41e699ceb866ff6393bac42e26d089c3602, private binary SHA256
b4cb184be549cdae4f4e731fc6cb832d1321d9672acc20a0f66d86b3ba36ea4c.

The real video ghost-target hook works: ON DOT snapshots show the actual
video-encodebin queue2 max-size-buffers1/bytes0/time0, current depth1 and3110400B.
No false activation claim from host tests alone. Nevertheless the candidate
fails the full recording gate and is NOT promoted. The Viewfinder runtime
binding/OnceCell have been removed from production, restoring viewfinder.rs
byte-for-byte to8eea07bef25a3cbbff302ff5b33b1cd552779d52. The module is retained
under cfg(test) only, for reproducible negative bench coverage.102 full release
workspace tests pass after removal. The historical private binary and stable
launcher are unchanged; no automatic rollback/reinstall or kernel change.

## Full-quality same-binary comparison, not cross-cohort cherry-picking

OFF then ON, not randomized. Soleflag MIPAD2_ENCODE_INPUT_ONE0/1;
CURRENT_CAPS1/RAW_DIRECT1/RAW_ASYNC0/COPY1/GUARD1 in BOTH. No source producer
queue. Two approximately30s clips/front+rear per arm. Actual1920x1080/30NV12,
1280x720/30GL preview,15MbpsH264+192kbps48kHzstereoAAC, original thermal guard
and thresholds, controlled1.6GHz ceiling. Exact480..2240MHz schedutil restored
after each arm. Sampled temperatures OFF68/69C, ON74/76C. No quality downgrade,
frame dropping, synthetic frames, PTS rewriting or warmup trimming.

|Arm|Camera|MP4fps|max gap|Frames|Cadence|source->encoder median|
|---|---|---:|---:|---:|---|---:|
|OFF|front|29.901770|92.000ms|899|RED|2.766ms|
|OFF|rear|30.011886|34.333ms|910|PASS for this clip|2.670ms|
|ON|front|29.853723|117.333ms|899|RED|65.675ms|
|ON|rear|29.948603|78.333ms|910|RED|65.252ms|

The prior R17h ON median166ms is a DIFFERENT cohort/binary, NOT this comparison's
control. It is not honest to claim166->65ms as a paired win. This new control
also shows the earlier scoped R17h cadence PASS does not establish repeatable
ordinary camera performance. Control p95/p99 are154.495/164.779ms front,
4.101/129.008ms rear; ON73.875/82.525ms front,75.466/81.121ms rear. Tail/median
tradeoffs do not override both ON cadence failures.

All3618 recorded frames have exact unique native PTS/offset joins, complete
named raw-direct/actual input-queue witnesses, equal input/output/MP4/decode
counts and conserved relativePTS within the declared mux tick. No omitted native
frames between recorded endpoints or adjacent identical decoded frames.12 offline
stages0; strict metadata suite45 tests passes normal and-O. Recorded-window
zero-driver-owned overlap OFF172616000/0ns, ON617862000/103122000ns. This software
ledger is not unique physical CSS starvation. One ON gap case remains unconfirmed;
global right-censored control calls42/42 retained, selected data calls complete.

## Consumer measurements change the next action

Actual input-queue sink-arrival->worker medians are OFF0.231/0.201ms versus
ON60.990/60.378ms; these include blocked enqueue, not isolated residence.
Encoder sink-chain inclusive medians OFF11.101/10.137ms versus ON32.996/32.343ms.
Same-task nested encoded-output pushes total median only0.433/0.385ms OFF,
0.419/0.384ms ON; their maxima are below8.2ms. ON remaining inclusive wall
median32.511/31.835ms is NOT proven CPU time, hardware wait or a unique lock.
Nested output can belong to a prior asynchronous input; zero/two-output call
histograms are retained, not forced to one output per call. The first encoder
second contains large chains, up to111.208ms ON/front; no first-second data is
removed to improve the gate. Some new gaps have no long source-task CAPS query
and coincide with long source off-CPU/backpressure intervals. Repeatedly blaming
only the earlier source CAPS intersection is no longer supported.

Next: inspect actual encoder-worker/VA scheduling and compare ordinary power,
uninstrumented full-quality recordings, rather than shrinking another queue or
raising CPU/GPU limits without evidence. No claim of resolved optical latency,
physical frame loss, foreground smoothness or Android/Windows parity.

## Retained failures and restored state

Initial controller stopped before ON dispatch: cooldown observation<=55C raced
to56C at unchanged preflight. OFF had already completed0/restored/archived.
The original failure receipt remains unchanged. Resume cools to<=52C before
the ORIGINAL<=55C gate, fresh preflight namespace, one ON dispatch and NO OFF
repeat. Both parents, tracing finalizers, decoders and managed serial controllers
exit0. Separate read-only audit joins the completed pair, not a rewritten success.

Both post-record10sRTCs resume functionally: S0I3+9196448/+9312288us. Serial
controllers run around both cycles; actual host handle state inside suspend
remains unproven. This R26 boot now has eight testedRTCs; earlier zero-residency
counterexamples still prevent a universal PM-fix claim. No new reboot or tone.

53 scalar aliases byte/access verified,47readable/six originally restricted;
old35/43/45/49receipts and permissions unchanged. To reserve space, eight OWN
older R17d/R17e test MP4s now superseded by later baselines were deleted:
458997181B. Nineteen later/current MP4s, two latest nativeJPEGs and84ancillary
files verified unchanged. First cleanup attempt stopped BEFORE receipt/deletion
on a guessed total-size bound; exact read-only inventory confirmed all eight
intact, then a stronger exact458997181B guard was used. No logs/photos/source/
kernels/raw traces deleted. All source/binary metadata, raw traces, graph,
journal, owner tables, media and individual pixel hashes remain tablet-private;
public data contains explicit scalar aggregates only.

Sensors+ISP usage0/suspended, original controls/settings restored, thermal+antiidle
active, GPU200..600MHz, no tracing instances/kprobes or live recorder/serial/cap
workers. Selected whole-boot BUG/WARNING/Oops/GPU-HANG/sensor-ID-NACK/stop-timeout
counts0. Stable R6 launcher and GRUBbqdiag1 unchanged. New R17i photo helper was
PREPARED BUT NOT EXECUTED after this performance rejection: previous R17h JPEG
results are not relabeled as R17i coverage. Full kernel/flash/OEM goal remains OPEN.
