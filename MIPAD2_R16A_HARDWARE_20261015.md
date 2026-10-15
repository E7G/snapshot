# R16a actual hardware ledger — 2026-10-15

Serial-attached one-shot boot into the already verified R19 kernel succeeded.
R16a was installed in its immutable private prefix, not promoted. This supersedes
the earlier **host-only** deployment status, not its historical results.

## Results and retained failures

* First tracing-off 30s control crashed at stop: SIGSEGV in the gstglcontext
  thread, libgallium +0x3dc53a. MP4 finalized and video/audio decoded. Core stayed
  on device. Specific Arch debuginfod Build-ID GETs returned404; no source-line
  attribution or Mesa fix is claimed.
* Old R15c 15s front baseline passed. R16a default renderer then passed front/rear
  30s runs at about30fps, max packet PTS gap34ms. One later successful trial does
  not erase the initial intermittent failure.
* Full active-pixel MD5 diagnostics matched all723 front and700 rear encoder
  inputs uniquely to native content, without repeated/reused/ambiguous/missing
  raw hashes. But encoder-side checksum cost averaged about13ms per frame,
  **reducing recording to24.06/23.22fps**. Performance gate is RED. This probe
  must not become normal configuration or establish normal pipeline performance.
* Rear decoded adjacent identical frames persisted:20 in default control,
  28 with content diagnostics,26 with requested OpenGL renderer. Instrumented
  raw input remained unique. Lossy quantization of near-static/noisy frames is
  still possible; these counts alone do not prove dropped physical motion.
* A separate per-process OpenGL arm passed both30s runs but did not remove rear
  decoded repeats. It is neither a proven crash fix nor a global workaround.
* Content cleanup's old >850-frame assertion failed because the slowed clips
  contained723/700frames. The app had already quit cleanly, controls were restored,
  both sensors suspended/PM0. Separate scalar-only inspection decoded both clips;
  the original failed gate/report is preserved, not relabeled green.
* Post-camera10s RTC suspend resumed in the same boot; measured S0I3 increase
  **9,713,472us**. USB serial/Wi-Fi returned, no new kernel errors.

No audio playback/tones. Camera media, individual hashes and core stayed on the
device. Prior binaries, r6 launcher, normal settings, cached sensor controls and
stable GRUB default are unchanged. Test processes and bounded serial capture ended.
R20/R21/R22 were not deployed. No visible-motion or Android parity claim.

See the adjacent JSON for scalar results. Next: replace the perturbing diagnostic
with a measured low-cost full-content method; retain the GL-stop crash separately,
then compare controlled motion without claiming that packet cadence proves UX.

## Primary references

* [GTK renderer debugging options](https://docs.gtk.org/gtk4/running.html):
  GSK_RENDERER is per-process diagnostic configuration, not a supported global fix.
* [GStreamer1.28.7 GL synchronization source](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst-libs/gst/gl/gstglsyncmeta.c):
  reference only; the captured stripped stack does not establish this file as cause.
