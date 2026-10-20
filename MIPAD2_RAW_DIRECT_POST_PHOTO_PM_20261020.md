# R17f delayed post-full-photo RTC and dual-camera reopen,2026-10-20

This adds a separate PM observation to MIPAD2_RAW_DIRECT_PHOTOS_20261020.md,
whose original photo trial had no subsequent RTC test at publication time.
The delay between completing those two photos and starting this controller
was1821.57seconds: this is NOT an immediate shutter-to-suspend stress test.

On the same deployed6.14 integrated R19/boot, a10s RTC wake succeeded;
S0I3 residency increased9374976us, wake IRQ9, USB remained configured,
exact front1976/127 and rear2486/128 exposure/gain retained, sensors PM0.
After resume the same private R17f RAW_DIRECT1/COPY1/GUARD1/queues0 binary
emitted actual NV12/1920x1080 output from both physical sensors at native
and shared-source stages; switching and normal close exit0, copyfail/critical0.

Fresh post-close checkpoint verifies sensor/ISP PMusage0/suspended, original
controls and protected default/app/system/plugin/thermal hashes. Anti-idle,
original thermal guard, GPU200..600 and stable GRUB unchanged; explicit selected
new kernel error patterncounts0. No new photo/video/playback/tones, no package
or default/kernel change, no reboot. All real imagery stays on the tablet.

Lifetime PTS/offset gaps from the complete observation phases remain in the
adjacent scalar JSON, including launch/switch periods. No resetting, trimming
or zero-drop inference. This does not prove foreground smoothness, image
quality, recorded cadence/latency, all PM conditions, safe physical DMA stop,
or Android parity. It is R19 hardware evidence, NOT an R24 boot/fault test.
The full kernel/flash objective and recording acceptance remain unfinished.
