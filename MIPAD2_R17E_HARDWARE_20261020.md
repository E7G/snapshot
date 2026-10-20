# Mi Pad 2 R17e guarded two-buffer scheduling: hardware evidence, 2026-10-20

Progress, not completion. Same Linux6.14 integrated R19 kernel and boot; no reboot,
kernel replacement, default launcher change or production package replacement.
Functional source is `b0c767c6a4ecfae7dc82b8213ba2a35ad6e89778`; this commit adds
only qualified scalar evidence. The earlier small-queue source document records
the historical host-only status; this document supersedes its hardware status.

## Mechanism and existing host gates

The tested opt-in `MIPAD2_CAPTURE_DECOUPLE_SMALL=1` requires MiPad2, physical v4l2src,
native-frame copy and the allocation-query guard. The nonleaky two-buffer queue
follows the full native copy and named allocation guard. It moves downstream
BUFFER work off the source task; it cannot manufacture consumer throughput.
Two buffers means queued buffers, not total in-flight memory or an optical/AV
latency bound. Hardware acceptance below REJECTS this two-buffer candidate for
default deployment: keep SMALL disabled. Legacy eight-buffer DECOUPLE also remains
disabled/rejected. Mechanically passing host tests do not overrule this regression.

Current actual-source V7 host tests: focused10, release/allfeatures/alltargets
workspace70, source-filter helper4. They cover native bytes/metadata/PTS/EOS,
native lease release, bounded backlog, task split, real allocation query
rejection, responsive flush/cancellation and strict rejection of the queued old
tail after flush. Earlier failed/mock tests remain recorded; no failed test is
relabeled PASS. See `MIPAD2_SMALL_QUEUE_20261019.md` and its evidence.

## Hardware protocol

Natural first ON pair was executed on Oct19. Controlled OFF then ON pairs were
executed on Oct20 (namespace suffix20261019 is the inherited protocol identifier,
not a claim they ran on Oct19). Same private executable SHA
`9c2ed1e6a4b7f5251ddc8d876e52326d5182d7f3069a6575045236b0bb687c65`,
plugins, native1080p30, GPU720p30 preview/default GSK Vulkan, H26415Mbps,
AAC192k/48kHz/stereo, VAasync2, COPY1, allocationGUARD1, content tracing0,
legacyDECOUPLE0. No dropped warmup, delayed record-start, lower caps/bitrate/FPS,
playback, operator hearing test or exported camera media.

Only namespace, SMALL flag and activation assertion differ between controlled
launchers. Every one of45 one-second samples per clip verifies all four CPU policy
ceilings exactly1600000kHz, with original minimum, governor and driver preserved.
The identical thermal guard sensor function, all thresholds and hysteresis remain
active: a temporary runtime ExecStart applies min(original thermal limit,1.60GHz).
The original guard script/unit/stop-timeout drop-in are preserved byte-for-byte.
The final ON root lease uses a bounded service, finally/ExecStopPost, mixed signal
handling and a stop-flag terminal wait. The earlier OFF automated cleanup FAILED
after capture/RTC: systemd killed its restore subprocess after unlink but before
daemon-reload. Explicit manual reload/restart restored the exact original guard
and limits before offline analysis; failed controller/journal are retained. This
is not an automated rollback PASS. Both captures retain all45 exact ceiling
samples. Each arm is restored before heavy offline analysis; final checkpoint
verifies original service, policy ranges, GPU limits, controls, sensor/ISP PM,
protected app/plugin hashes, serial closure and unchanged global tracing.
Initial gates<=55C, root>2GiB/shm>150MiB; recording<80C/root>1GiB retained.

## Actual whole-clip results

| Cohort | Sensor | fps | max MP4 PTS gap ms | Frames | Cadence gate | Recorded zero-driver-owned s | Source push median ms | Source-to-encoder p99 ms |
|---|---|---:|---:|---:|---|---:|---:|---:|
| small-queue | front | 30.000325 | 34.333 | 923 | True | 0.000000 | 1.995 | 151.414 |
| small-queue | rear | 30.010742 | 34.000 | 923 | True | 0.000000 | 2.033 | 115.841 |
| caplease2-off | front | 30.001627 | 34.334 | 923 | True | 0.000000 | 2.419 | 344.348 |
| caplease2-off | rear | 30.012243 | 36.000 | 908 | True | 0.000000 | 2.507 | 299.157 |
| caplease2-on | front | 29.837221 | 45.333 | 901 | True | 6.679523 | 32.986 | 464.964 |
| caplease2-on | rear | 30.011480 | 34.334 | 916 | True | 0.000000 | 2.416 | 380.783 |

REJECT this candidate for default deployment. Cadence alone passes, but the capped ON front recorded-completion window has6.679523s cumulative zero-driver-owned time (OFF zero). ON source-to-encoder arrival medians rise to433.097ms front and370.343ms rear, versus OFF3.497/3.834ms. The ON multiqueue BUFFER-chain medians are33.369/33.036ms and the front source push median32.986ms: the real task split did not remove downstream backpressure. This is a measured availability/latency regression in this sequential pair, not a unique causal proof. Keep SMALL disabled; do not enlarge queues or lower quality to conceal it. The natural first green run is not sufficient.

Actual encoded frames are joined by exact native PTS/offset to same-TID successful
DQ fences and exact kernel completion ownership. Source->encoder input, encoder
output, both parser EOS epochs, MP4 packets and decoded counts are conserved.
Native pushes between actual recorded endpoints are counted, not discarded by
analysis. Mux PTS tolerance is exactly one declared1/3000s timebase tick, not fitted
clock drift. Availability windows mean first..last recorded completion, not optical
SOF or every frame since the UI button. Full distributions, maxima and delay tails
are retained in the adjacent JSON; source-to-encoder arrival is not glass-to-glass
or physical audio/video synchronization. Pacing call right-censor counts are
retained, so this is not an all-calls-balanced claim.

Source syscall filters are now installed exit-first, with verified readback and an
explicit commit marker. Strict per-source pairing passes; boundary censoring is
reported. Old R17d enter-first anomalies were localized before the commit marker;
do not silently repair old raw traces or invent durations.

## Retained unsuccessful preparations

The cap1600 initial runtime-expanded trial had a syntax error before recording;
v2 stopped before camera on premature immediate sysfs readback; v3 first recording
stopped when the ordinary thermal guard replaced the experimental cap with1.84GHz.
That partial clip is NOT fixed-cap evidence. External thermal decisions were not
overwritten. A later preflight refused insufficient SHM and another refused the
previously existing stop-timeout drop-in before any override. The final lease
explicitly preserves that known drop-in. Completed scalar traces were copied and
roundtrip-verified locally, with original paths preserved; no logs were discarded.
Host mock failures and the final actual runtime-expanded AST checks are retained.
No old diagnostic video deletion approval was broadened or reused.

## Remaining acceptance gaps

One device and sequential arms leave order/load/thermal confounds. Both private
tracers add overhead; this is not uninstrumented throughput proof. Preserve the
natural and capped cohorts as distinct conditions. End-to-end latency, physical
AV sync, foreground preview smoothness/rotation, full-resolution photo, autofocus,
AE/WB visual quality, long recording/switch/reopen/flush/PM and Mesa/GstGL lifecycle
still require acceptance. Kernel SP/CSS reset/DMA fencing, parameter/buffer lifetime
and the complete Linux6.14/flash/Android-Windows hardware parity objective remain
open. Next focus is the actual consumer/multiqueue BUFFER backpressure and queued
residence, not another larger producer queue or FPS/bitrate reduction. Root reserve
is now below the2GiB new-recording gate, so do not start another recording pair
without a fresh legitimate space/preflight check. Existing traces support offline
work; keep all current samples and logs. No default promotion; full objective is
not complete.

All real media, raw traces, kernel owner tokens, raw syscall pointer arguments and
per-frame pixel digests remain on the tablet. Public JSON is an explicit scalar
allowlist, not an export of those raw private inputs.


## Additional offline exact-frame consumer arrival attribution

No new recording or quality change. Exact native PTS/offset keys uniquely join
every recorded frame to MQ sink arrival, MQ src push and encoder sink arrival.
The three consecutive arrival differences sum exactly per frame; their aggregate
medians/percentiles need not sum. Units below are milliseconds.

| Capped arm | Sensor | Source->MQ sink median | MQ sink->MQ src median | MQ src->encoder median | MQ src/encoder different tasks |
|---|---|---:|---:|---:|---:|
| off | front | 2.303 | 0.254 | 0.898 | 923/923 |
| off | rear | 2.362 | 0.240 | 1.106 | 908/908 |
| on | front | 99.816 | 167.490 | 167.597 | 901/901 |
| on | rear | 38.204 | 166.095 | 165.885 | 916/916 |

The middle interval includes enqueue wait and internal multiqueue residence; the
last includes downstream task changes and record-path queueing. It is NOT a
synchronous nested chain into the encoder: ALL actual recorded frames switch
tasks after MQ egress. An initial V1 analyzer incorrectly asserted same-thread
nesting and FAILED; its logs are retained. V2 explicitly counts those task changes,
retains exact frame identity/order and does not invent nested call durations.

The ON delay accumulates across both MQ residence and downstream arrival, with
an additional producer-side backlog at the front camera. Do not mistake the
full blocked source push for memcpy CPU cost or infer one unique mutex/CPU cause.
Next inspect actual downstream queue/sink clock/pacing behavior from these
existing traces and pipeline source; another bigger producer queue is not a fix.
The hardware rejection and unchanged-default decision above remain in effect.
