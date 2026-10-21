# Mi Pad 2 R17h: cadence passes, input-queue latency still unresolved

Actual device date: 2026-10-21. Kernel `6.14.0-mipad2-integrated-r26`, built
from `6074419168fbb70aa5685e976874da6cf86d1fae`. Application code
`8eea07bef25a3cbbff302ff5b33b1cd552779d52`; actual private binary SHA256
`9edb3036db463f770cb31599ed5d3455f0ae5d3070b754210c9b3733a0b1063c`.
This is **not** a complete camera-performance fix or stable-default promotion.

## Same-binary controlled crossover

One boot, OFF then ON, not randomized. Only application flag difference:
`MIPAD2_CAPTURE_CURRENT_CAPS=0/1`. Both arms use RAW_DIRECT=1, RAW_ASYNC=0,
capture copy/allocation guard; no added source producer queue. Each arm records
front and rear for approximately 30 seconds. Actual native 1920x1080/30 NV12,
1280x720/30 GL preview, requested 15 Mbps H264 and 192 kbps 48 kHz stereo AAC.
No bitrate/resolution/fps downgrade, warmup trimming, timestamp rewrite, synthetic
frames or frame-dropping fix. Ordinary default-power and foreground tests remain
separate requirements.

Controlled CPU ceiling 1.6 GHz, with original thermal sensors, thresholds,
hysteresis and protection retained. Exact original 480 MHz..2240 MHz schedutil
policies restored after each arm with finally plus independent ExecStopPost.
Sampled thermal maxima OFF 67/69 C, ON 71/77 C: not thermally matched trials.

| Arm | Camera | Measured MP4 fps | Maximum PTS gap | Frames | Cadence |
|---|---|---:|---:|---:|---|
| OFF | front | 29.768468 | 171.666 ms | 901 | RED |
| OFF | rear | 29.977326 | 93.334 ms | 900 | RED |
| ON | front | 30.001306 | 42.334 ms | 920 | PASS |
| ON | rear | 30.018406 | 44.667 ms | 898 | PASS |

Unchanged cadence gate: >29.5 measured fps, maximum gap <50 ms, no >100 ms gap.
All 3,619 recorded frames have unique exact native PTS/offset joins, complete
named raw-direct route witnesses, equal encoder-input/output/MP4/decode counts,
and encoder-relative PTS equality. MP4 residual stays within one declared
333333.333 ns tick. No native frame between recorded endpoints is omitted from
the encoder; offsets increment by one. No adjacent identical decoded frames.
This is not an exhaustive count of physical sensor exposures inside the ISP.
All twelve existing offline analysis stages finish successfully.

## Why this is NOT accepted as the complete performance fix

Exact native-push-begin to encoder-chain-begin median rises from OFF
2.752/2.745 ms to ON **166.976/166.346 ms** (front/rear); ON maxima
230.216/199.378 ms. Every ON recorded frame exceeds 100 ms at this boundary.
Cadence alone would conceal this regression. These are software stage-arrival
intervals, not measured optical latency or isolated queue residence.

New strict metadata analysis identifies the actual named
`GstEncodeBin:video-encodebin/GstQueue:queue2` sink and worker source on every
recorded frame, not a queue inferred from an opt-in flag. Enqueue is nested in
the named raw output on the source task, and the encoder is nested on that
queue's different worker task. Each frame's source-to-encoder interval partitions
exactly into ordered, non-negative measured pieces.

Queue sink-arrival to worker-push medians are OFF 0.217/0.218 ms versus ON
**162.369/161.885 ms**. Worker-to-encoder medians are only 0.031/0.031 ms ON.
The interval includes blocking enqueue: do not label all of it isolated queue
residence or CPU time. Two retained ON recording DOT snapshots show four raw
buffers, 12,441,600 bytes, approximately 133 ms in this queue. DOT omits default
properties; omission is not explicit zero, and snapshots do not establish every
instant's queue depth. No queue property was changed during the experiment.

Native push wall-time median also rises to about 33.24 ms. Recorded-window
zero-driver-owned overlap is retained, not hidden: OFF 0.251/0.070 seconds,
ON 7.077/6.246 seconds. This software bookkeeping does not prove unique physical
CSS starvation, but it prevents claiming all ownership/power gates are green.
The next targeted experiment is this **encoder-input queue**, not another
unbounded/native producer queue or lowering recording quality. Preserve audio,
EOS and backpressure behavior; do not turn it into a leaky recording queue.

The queue analyzer passes seven focused tests in normal and optimized Python;
the full 45-test metadata suite also passes in both modes on the tablet. Missing
or duplicate stages, wrong ancestry/task, reversed clocks, invalid identities and
bad nesting fail closed. Overlapping enqueue/worker activity is permitted and
non-OK flows are retained. Outputs contain aggregate timings only.

## Actual full-native photo and power regression

With CURRENT_CAPS=1, actual front 2560x1920 and rear 3264x2448 JPEGs pass file
finalization, exact native sensor/ISP mode observation, metadata and full decode.
Both restore 1920x1080 preview, four live preview phases pass, process closes
with status 0, no video/photo leftovers are mistaken for the next capture.
Private shutter sound disabled; no audible tones or playback. This is not a
white-balance/exposure/focus or optical-parity result.

Both post-recording ten-second RTC cycles resume functionally, S0I3 deltas
9,717,472 and 9,723,200 us. Serial-capture controllers run around both cycles;
exact host handle state during USB suspend is not proven. Same R26 boot now has
six tested RTC cycles. Earlier zero-residency counterexamples remain valid;
intermittent active-debug residency is not declared fixed.

Original controller is retained as FAILED: after all four recordings and CPU
restoration, ON trace archival hit its >2 GiB plus next-copy storage reserve.
No capture rerun or weakened reserve repairs this bookkeeping. Six explicitly
obsolete own R16 test MP4s were deleted (46,283,104 bytes); 27 formal/current
recordings and 54 ancillary files verified unchanged. Only then were ON scalar
traces archived byte-identically, with a separate read-only completion audit.
Failed receipt/log remains unchanged.

All 49 scalar aliases and stored SHA256/access policies verified, including six
originally restricted files. Old 35/43/45-entry receipts remain unchanged; a new
49-entry explicit restoration helper supplements them. Raw traces, ownership
tables, journal/DOT, media, syscall arguments and individual pixel hashes remain
tablet-private. Global control-call right-censor counts OFF 47 / ON 42 are
retained; no selected recorded data call is unfinished.

Both sensors and ISP return to runtime_usage=0/suspended, controls and ordinary
settings restored, anti-idle and original thermal guard active, GPU 200..600 MHz,
no tracing instances/kprobes or live recorder. Selected whole-boot BUG/WARNING/
Oops/GPU-HANG/sensor-ID-NACK/stream-stop-timeout counts all zero. Kernel, default
launcher and GRUB saved entry unchanged; no reboot.

## Primary semantics and remaining scope

- [GStreamer queue](https://gstreamer.freedesktop.org/documentation/coreelements/queue.html):
  queue bounds block by default; leaking drops data. Read actual level properties.
- [Encodebin properties](https://gstreamer.freedesktop.org/documentation/encoding/encodebin.html):
  input queue limits are configurable; do not mistake output/mux queues for it.
- [Exact 1.28.7 implementation](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst/encoding/gstencodebasebin.c#L1573-L1602)
  sets input-queue limits before adding it and exposing the ghost sink pad.
- [Exact query tracer caveat](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gstreamer/gst/gstpad.c#L4140-L4155):
  handled-probe query POST trace occurs before its final result becomes TRUE.
  A traced false value on this path alone is not proof of failed negotiation.
  This is source-verified semantics, not a separately measured caller-return test.

Still open: this persistent latency, ordinary/unprofiled/default-power and visible
foreground motion, dynamic photo/record/reopen/resume combinations, optical
AE/WB/focus, DMA/reset fencing, S5 cold reliability and complete kernel/flash/OEM
qualification. CURRENT_CAPS remains opt-in, stable launcher remains untouched.
