# Experimental Mi Pad 2 preview scaling (default OFF)

## Current application candidate

The tested application source is `f99033b04f22fcabb13dca126fd77fad89608703`.
Enable its GL scaler only in an explicit private process with
`MIPAD2_PREVIEW_GPU_SCALE=1`. Mi Pad 2, Android-video-profile and `glshader`
availability gates must also pass. No launcher enables this option by default.
The previous CPU branch remains the normal path.

The display-only branch is videorate -> GL upload/color conversion -> RGBA
shader fit -> existing GPU orientation/display sink. Serialized RGBA CAPS update
the shader's source display aspect ratio, including pixel aspect ratio. Its
built-in width/height uniforms describe the output, not the input. The shader
adds opaque black borders instead of stretching 4:3 or portrait content.
Invalid input CAPS post a negotiation error. A GL context/shader failure is not
silently hidden by a dynamic CPU fallback.

Only this opt-in branch sets camerabin's registered
`no-viewfinder-conversion` flag while NULL. Existing flags are preserved;
audio, recording and still-image conversion flags are not disabled. This is
necessary because camerabin's internal `vfbin-videoscale` otherwise keeps doing
the CPU resize even after a shader has been added downstream.

Recording/JPEG source caps, bitrate, clock and thermal policies are unchanged by
this candidate. That is not a claim that recording or photos have been qualified
with the new preview branch.

## Actual-device evidence, 2026-10-25

Tests used the integrated 7.2 r5 kernel and GStreamer 1.28.7. Detailed traces,
cores, frames, process identifiers and logs remain device-private.

- Six real-device Rust unit tests pass: all 32 enable-gate combinations, valid
  and invalid DARs, serialized CAPS changes, actual bin-to-rotation linkage and
  its missing-src negative control, eight combinations of other camerabin
  flags, idempotence, and late-state/wrong-element rejection. These tests do not
  put a camera into READY/PLAYING.
- Same application binary, GL ON then OFF: both physical cameras deliver about
  30 fps at native 1920x1080. GL ON removes all measured software converter
  calls. Front/rear process CPU is about 34.8%/35.5% lower in this bounded pair,
  while render-engine activity increases. This is not a total-power result.
- Reverse-order OFF then ON with a stronger protocol: require three seconds of
  stable native 1920x1080 before each measurement window. Both cameras remain
  about 30 fps, with no nonpositive or >50 ms PTS intervals in those windows.
  OFF has 1641 NV12 resize calls; ON has zero, with complete trace coverage.
  Front/rear process CPU is about 36.2%/39.0% lower. The protocol differs from
  the first pair, so this is not a statistically controlled identical-protocol
  repeat. No forced clocks were used.
- An earlier reverse-order run starts the front window at 1280x720, changes to
  1920x1080, and has a 133 ms display PTS gap. It is explicitly excluded from
  steady-1080p qualification, not relabelled as a success.
- Five real-GPU quadrant/border cases cover 1920x1080, 2560x1920, 3264x2448,
  portrait 720x1280, and non-square pixels. Geometry passes. A separate 21-case
  synthetic NV12 comparison finds exact neutral black/gray/white agreement and
  small ramp differences, but large fine-luma/chroma differences from the old
  CPU bilinear path. Neither pixel identity nor superiority of either filter
  follows from those differences. Real-scene/OEM quality remains unqualified.

## NV12 hardware-scaler investigation (prototype only)

No VPP application branch has been installed or enabled. Synthetic pipelines
using the already-selected VA plugin establish the following:

1. Plain VPP `add-borders=true` renders five geometry cases, but portrait content
   occupies 408 pixels where 405 are expected. Independent edge measurements at
   25%, 50% and 75% amplitude confirm a true extent difference, not a blur halo.
2. Scale without VA borders into an 8-aligned NV12 coded rectangle. Set its PAR
   to `source_DAR * coded_height / coded_width`, preserving DAR exactly, then
   use the existing GL shader for final fitting. All five geometry cases pass;
   portrait coded 408x720 with PAR 135/136 becomes exactly 405x720 of content.
   Both VA output and shader-input CAPS are checked, rather than trusting an
   assumed uniform value alone.
3. Ordinary-memory VA output is too slow: a synthetic 1080p30 source retains
   all 750 timestamps but takes about 16.17 wall-clock fps to deliver them.
   A 30 fps timestamp span alone would incorrectly accept this path.
4. The selected driver advertises NV12 with Intel Y-tiled modifier
   `0x0100000000000002`. Preserve that modifier in
   `video/x-raw(memory:DMABuf),format=DMA_DRM` between VA and GL upload; do not
   incorrectly advertise these surfaces as linear NV12. All five geometry
   cases pass, and every VA output buffer is verified as DMA-BUF memory.
5. A separate synthetic DMA-BUF run delivers all 750 frames at about 30.00
   wall-clock fps, with zero non-DMA buffers. Approximate process CPU/render
   busy fractions in the synthetic probes are:

   | Pipeline | CPU cores | Render busy | Wall-clock fps |
   |---|---:|---:|---:|
   | CPU NV12 scale -> GL | 0.696 | 0.037 | 30.00 |
   | GL RGB scale | 0.313 | 0.196 | 30.00 |
   | VA -> ordinary memory -> GL fit | 0.852 | 0.120 | 16.17 |
   | VA -> DMA-BUF -> GL fit | 0.338 | 0.113 | 30.00 |

   These are synthetic-source/appsink probes, not GTK, camera, encoder or power
   qualification. The ordinary-memory utilization also corresponds to fewer
   wall-clock frames, so it must not be compared as equal-throughput work.
   DMA-BUF transport is verified, but complete end-to-end absence of internal
   copying is not proven. There is no pixel readback in these throughput tests.

An initial telemetry run aborts before completing the CPU case. Its strict
duplicate-DRM-descriptor counter equality can reject valid drifting reads;
the exact old traceback was not retained, so that attribution is not asserted
as proven. The replacement merges maximum counters per DRM client/engine,
passes 24 descriptor-order permutations and a missing-client negative control,
and preserves full private tracebacks for future failures. The old run is not
used as a CPU baseline.

## Retained construction/test failures

The first GL candidate omitted its src ghost pad and aborted before streaming.
The empty converter output from that abort is not zero-call evidence. Its core
and partial trace are retained, including original signal 6/controller status 1.
The following source-pad correction links, but still performs 1315 CPU resize
calls; it is not a demonstrated optimization. Only the later NULL-state
camerabin-flag correction removes the internal scaler.

The host SDK lacks a GL plugin. An initial on-device CAPS fixture also used an
untemplated sink pad and failed. Both failures are retained; a real RGBA/GLMemory
pad template fixes the fixture without skipping the runtime tests. The actual
tablet video library is ORC-enabled; an ORC-disabled SDK library is not evidence
that the normal tablet process lacks ORC.

## Still required before promotion

Real-scene fine-detail/chroma and OEM calibration, source transitions, photos,
rotation, reopen/resume, full native 1080p recording, sustained and repeated
thermal/power/latency tests, and an uninstrumented application deployment remain
open. The VPP path also needs application integration and negotiation tests,
including dynamic DAR/PAR changes and error handling. Geometry and synthetic
throughput must not substitute for these gates. Defaults remain OFF.
