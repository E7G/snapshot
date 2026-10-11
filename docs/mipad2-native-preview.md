# Mi Pad 2 native preview and startup repair

## Reproduced evidence (2026-10-10)

Device still runs 6.14.0-mipad2-cachyos-navkeys-bqdiag1; r9 has not booted.
Kernel syscall tracepoints (private ftrace instance, no ptrace delay) show
PipeWire STREAMON -> STREAMOFF -> STREAMON during GStreamer live-source
startup. The second STREAMON succeeds but no DQBUF follows before timeout.
With ptrace delays both 25 and 30 fps start; without delays both time out.
This is a timing-sensitive stop/restart interaction, not proof of one specific
firmware defect. CSS -22 after stop is not sufficient evidence by itself.

Direct GStreamer v4l2src MMAP 1080p/30fps captured eight frames on six
consecutive opens without tracing. Direct decoded grayscale output also works.

Primary implementation references:
- https://github.com/PipeWire/pipewire/blob/1.6.8/src/gst/gstpipewiresrc.c
- https://github.com/PipeWire/pipewire/blob/1.6.8/spa/plugins/v4l2/v4l2-utils.c
- https://gstreamer.freedesktop.org/documentation/video4linux2/v4l2src.html

## Changes

- Only Xiaomi Inc / Mipad2 + AtomISP /dev/video0 uses native v4l2src MMAP.
  PipeWire/portal discovery is retained; other devices keep their old backend.
  No undocumented PipeWire property or permanent tracing workaround is used.
- GTK state waits no longer join a blocking thread on the UI loop. Stop and
  switch invalidate stale asynchronous waits before they alter the pipeline.
- AEC/AGC continues to meter decoded frames and write bounded sensor controls.
- Autofocus receives five fresh grayscale frames from the existing preview,
  scores the last three using the old contrast metric, and never opens a second
  exclusive ISP owner. Requests are bounded, timeout, and cancel on stop/switch.
- Existing OTP presets and AE freeze during AF are retained. Sampling changes
  from GStreamer scaling to deterministic nearest-neighbor grayscale sampling;
  AF quality is not declared hardware-verified merely because unit tests pass.

## Boundaries

This is a Snapshot-owned device-specific capture/AEC/AGC bridge, not a complete
AtomISP/libcamera IPA or an AWB implementation. Kernel stream restart/multiple
open issues remain separate kernel work. No kernel ABI or system camera plugin
is changed. Installation must be isolated and rollback retained. Actual preview,
AE convergence, camera switching, photography, and reopen require device tests.

## 2026-10-11 hardware follow-up

The r4 candidate produced 7658 AE log samples over roughly 55 minutes before
losing its Wayland compositor connection. The user confirmed visible preview.
The log also records front -> rear -> front switching and a completed rear
OTP-bounded AF sweep (237..366, selected 333). This proves live sampler/control
operation, not objective sharpness quality. Rear AE incorrectly disabled because
the running T4KA3 exposes exposure minimum 0 (maximum 2486), while Range::valid
required minimum >=1. Accept nonnegative native ranges, preserve all driver
limits/steps, and test dark recovery from zero plus exhaustive rear bounds.
No gain-unit conversion or undocumented Android/Windows tuning is guessed.

Current-day GUI reopen, photo capture and rear AE convergence remain pending.
The default launcher remains r2 until candidate validation is complete.
