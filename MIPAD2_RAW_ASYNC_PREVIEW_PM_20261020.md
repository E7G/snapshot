# R17g private asynchronous native raw preview/switch/RTC smoke

The actual source candidate in MIPAD2_RAW_ASYNC_CAPTURE_20261020.md is now
privately built/installed and has real dual-camera preview/switch/close evidence
before and after a10s RTC suspend. This supersedes ONLY that source publication's
not-installed/not-preview-tested status: NO recording-cadence acceptance yet.

Source `4a9f455749366bb631a8f209bf3b06179b981246`, executable SHA
`6298e0b37e28f563a5ee5e97d57edb0c4594cc3dd67cf31334b2422508209c73`. Deployed kernel stays6.14 integrated R19,
same boot. RAW_DIRECT1/RAW_ASYNC1/COPY1/GUARD1, old producer queues0 plus the new
four-copy tail queue. Actual native/shared output NV12 1920x1080 with requested
30/1 from both physical sensors; default renderer and full preview profile.
The harness only uses switch-camera and quit: no photograph, recording, playback,
tones, new kernel boot or default launcher/system-plugin replacement. No added
CPU-frequency cap in these preview-only runs; original thermal policy retained.

Before/after runs close MainPID0/Resultsuccess/ExecMainStatus0; exact exposure/gain
restored, both sensors/ISP PMusage0/suspended. Matching installed binary, live
ASYNC1 environment, actual async graph log and both physical input0/1 frame
stages verified. Fresh checkpoint also verifies protected defaults/prior-private
and system hashes, named tracer/R17f/R17g hashes, anti-idle, original thermal
guard/drop-in, GPU200..600, saved GRUB/empty next-entry and no tracefs instance.
Explicit selected new kernel-error pattern counts0, not all-error absence proof.

RTC resumes functionally; S0I3 increases9563328us, wake IRQ9, same boot;
then actual both-input reopen/switch/close again. This is not every suspend
condition or immediate post-photo/recording PM validation.

## Retained rough edges and harness correction

Before/after cumulative PTS maxima are104558500/108292488ns;
offset counters and all samples remain in adjacent scalar evidence. Observation
windows start after launch, full journals stay tablet-local; these counters
include launch/switch and are NOT reset, trimmed or assigned a harmless cause.
Do not infer foreground smoothness, zero drops, optical or Android parity.

The initial hardware run completed both physical preview phases and normal
close, but its finalizer FAILED on an incorrect >=2 async-activation-log gate.
Camera switching reuses the source graph, so exactly one construction log is
valid. A read-only audit checked that activation and real frame output for BOTH
inputs, idle process, exact controls/PM, settings and no media; original failed
on-device receipt/host stderr retained. No repeat merely to replace that failure.
Future finalizer requires a real activation plus existing both-input frame gates.

New named-only control tracer is installed privately, dependency/factory path
verified, old tracer unchanged. GST_TRACERS was OFF in these runs: it has NOT yet
captured real hardware-frame output witnesses. Real same-binary ASYNC0/1 full-
quality recording, all-frame source/encoder/MP4 conservation, recorded driver
availability/latency and full-photo hardware remain REQUIRED and unperformed.
The full original kernel/flash/Android-Windows objective stays OPEN.
