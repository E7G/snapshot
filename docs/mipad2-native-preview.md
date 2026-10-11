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

## 2026-10-11 evening validation (supersedes pending status above)

Application code/package commit: b503142298ecd34129a24285754574c257fc78e6.
Device kernel remains 6.14.0-mipad2-cachyos-navkeys-bqdiag1; no reboot or r9 test.

- User confirmed both camera previews and normal rear photo/gallery output.
- Rear AE writes exposure and native analogue-gain controls; 43 of 70 logged
  steady samples reached luma 112..128. Zero exposure minimum no longer disables
  AE. This is bounded application-owned AEC/AGC, not full IPA/AWB/3A.
- Rear AF completed using the existing preview: selected 357 within validated
  OTP range 237..366. Objective focus quality is not yet measured.
- Default launcher now selects isolated native-ae-r5; three graceful close/reopen
  trials verified the exact r5 binary, native V4L2 capture, and metered frames.
- Original launcher is preserved at
  /var/lib/mipad2-camera-native-ae-r5/launcher-before-r5. System Snapshot, r2,
  boot files, stable kernel and anti-idle remain unchanged.
- Direct front/rear/front/rear capture at their common 1280x720 size produced
  eight JPEG frames per trial. Forcing 1920x1080 on a fresh rear source fails
  negotiation because the current rear enumeration does not offer that size;
  this is not a test of a supported rear mode.
- Final front-photo confirmation, dynamic/high-contrast AE stability, objective
  AF quality and complete AWB remain separate validation/work items.

The `Unknown factory: jifmux` log is not proof of a missing JPEG plugin.
GStreamer set_element_properties_from_encoding_profile reports this when its
profile property map lacks a matching factory name; rear photography passed.
The property-map behavior has not been patched in this build. Primary source:
https://raw.githubusercontent.com/GStreamer/gstreamer/1.28/subprojects/gst-plugins-base/gst/encoding/gstencodebasebin.c
