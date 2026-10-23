# 7.2 normal rear long sample: whole-file cadence pass, earlier failures retained

Follow-up to [front long pass and interrupted rear](MIPAD2_NORMAL_ASYNC_LONG_7_2.md).
Same actual7.2 r3 boot, 0a7ce87a app16bd4d3 binary, fe70 private copied-async
launcher, fixed full-recipe VA and sourceAAC. Source identities and exact scalar
results: [summary](tests/mipad2-normal-rear-long-7.2-summary.json).

No new kernel build/reboot, diagnostic action, background capture, ftrace,
in-process tracer, quality/rate reduction, timestamp smoothing or rewriting.
Fresh launch at48C; real normal GTK SwitchCamera button via PID-matched AT-SPI,
then active-window/unlocked/ownership-gated normal `t` recording accelerator.
The new one-shot namespace uses labelled guards and retains a failing sample
before stopping, but this time no recording guard failed.

Requested60s completed; full PTS span**63.841s**, **1917** packets/decodedframes,
**30.012061fps**, max**34.333ms**, min32.333ms; no nonpositive/>50ms/>100ms
intervals or adjacent exact decoded repeats. Native1920x1080H26415.012Mbps,
AAC192159bps48kHzstereo; complete video/audio decode passed without warnings.
Max observed79C, CPU ceiling1.60GHz throughout retained samples. FixedVA+i965
mapped; actual encoder factory not independently observed. No frames excluded.

App normal Quit exit0; original sensor crop/format/controls and global camera
settings restored, temporary keyboard removed, both sensors/ISP suspended usage0.
Kernel fault deltas0. Source/per-frame timing/hashes and media remain tablet-private.

This is a whole stored-file cadence sample, not proof of smooth live preview,
physical sensor frame conservation, optical AE/AWB/focus, prolonged thermal
stability or Android/OEM parity. Not frequency/scene-matched randomized A/B/A.
The previous14.66s rear interrupted sample and556ms/duplicate-PTS normal failure
remain retained and failed; neither is retrospectively relabelled or attributed.
No stable/default copied-async promotion or complete package/rootfs qualification.

## Independent thermal guard correction, chronologically AFTER this recording

The existing guard escalated at most one level per3s poll, even if the current
sample already exceeded the hottest threshold. Source correction selects the
required hot ceiling immediately; temperature thresholds, frequency ceilings,
cooling hysteresis/one-step cooling and sampling period unchanged. Tested source
was atomically installed with original private backup, thermal service-only restart,
18s cool-state observation and seven synthetic policy cases on the installed
helper. No real hot transition was deliberately induced.

**This rear long pass occurred before that correction.** It cannot demonstrate
that the thermal fix caused smoother recording or fixed the older interruption.
The previous3.5h hard freeze and failed-reset DMA quiescence remain unresolved.
Thermal source/report link is pinned by the integration delivery.

Fresh postflight:686 protected installed files unchanged, both guardsactive,
kernel fault counts0, no failed units or active camera/GStreamer clients; both
sensors/ISP suspended usage0. The intentionally changed guard is OUTSIDE that
historical686-file set and separately matches the exact tested source SHA.
UCM8 installed source hashes match; no audio playback or engine reload.
User-selected automatic lock/DPMS/suspend remainOFF, manual locking/login unchanged.

Pinned thermal integration source: [production guard and exact tested scope](https://github.com/E7G/xiaomi-latte-flash_tools/blob/3806e95ffcf9edcaf3002ece7c8e979b3c9d525f/kernel/THERMAL_GUARD_7_2.md).
