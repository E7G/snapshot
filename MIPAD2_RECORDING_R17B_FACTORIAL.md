# R17b bitrate/audio isolation, with retained failures

Same private R17b binary, same R19 boot, native1080p and full preview path.
MIPAD2_RECORDING_QUALITY still requests the joint15Mbps/AAC target. Only the
explicit diagnostic-action gate enables legacy/video-only/audio-only ablations;
unknown axes do not silently lower quality. All62 framework tests/full build/
private packaging passed. No default launcher or tablet packages changed.

## Actual30s clips, front/rear fps

| Diagnostic arm | Front | Rear |
|---|---:|---:|
| Legacy2Mbps/MP3 |30.002|30.011|
|2Mbps/AAC192k48k2ch |30.000|30.010|
|15Mbps/MP3 |**26.104**|**27.320**|
|15Mbps/AAC, PMU on |**26.219**|**27.359**|
|15Mbps/AAC, PMU off |**26.232**|**27.116**|

All ten video/audio streams decode. Higher-video-rate arms remain throughput
RED; AAC-only did not reproduce the slowdown in this round. This narrows the
next investigation to video/preview/VA flow, not a proven single root cause.
Neither decoded uniqueness nor a lower-quality ablation constitutes success.

Initial controller halted after video's old >850 decoded-frame assertion,
**after** the app had quit and restored controls. Original reports remain RED;
separate inspection decoded the remaining clip. Fresh joint helpers compare
decoded frames with packet count, while unchanged whole-clip fps/gap gates still
fail. Decode integrity and performance must not block each other's inspection.

The first PMU/serial runs reached their bounds after the controller halted. They
were verified terminal before unique later joint monitoring. No completed arm
was restarted. These are separated time windows, not an interleaved randomized
or optically controlled experiment. PMU-off still has scalar application trace
probes and serial capture; it is not a claim that all instrumentation is absent.

## GPU/CPU and the next hypothesis

Read-only i915 PMU counting used no perf package, sysctl changes, RPS writes or
ptrace. Whole-system counters and instantaneous frequency samples were analyzed
only3–27s into recording; whole-clip cadence gates remain separate. Video-only
and joint sampled600MHz throughout those samples. No guessed frequency-floor
override was performed. Mean device I/O busy was modest, but I/O latency spikes
are not excluded. A joint scalar thread sample observes i915_request_wait_timeout.
The complete JSON includes counters, process CPU and device I/O measurements;
system-wide counters do not establish per-process causal attribution.

Upstream1.28.7 live H264 uses preferred_output_delay0, so output waits can serialize
submission; its non-live path uses4. A proposed private opt-in depth2 reuses the
existing ready-output/drain/pool-latency machinery without changing bitrate,
quantization/profile, geometry or frame rate. See
[unbuilt proposal](contrib/mipad2/va-async-depth/README.md).
Its2040 decision/count-model cases and patch syntax pass are **not** a full VA
plugin build, surface-ownership proof, EOS/latency proof or runtime/performance
fix. Full source retrieval is incomplete: truncated1MiB archive, ranged/TLS
failures and native/WSL Git failures are retained. No proposal plugin was built,
installed or run. WSL-only libva development packages were added, not tablet
packages. Complete exact source/build validation is the next gate.

Post-camera10s RTC resume measured S0I3+9,248,672us, same boot, no new kernel
errors. Both PMU processes and bounded serial captures ended. Original sensor
controls, PM0/suspended, normal settings, GPU200/600MHz limits, protected prior
binaries and r6 launcher are verified. Stable GRUB default remains unchanged.
No playback/tones or private media/core transfer. Earlier Mesa/startup failures,
kernel failed-reset/SP/DMA safety, full flash/default integration and Android
optical/motion parity remain unfinished. No completion or promotion claim.

Primary references:
* [Linux6.14 PMU UAPI](https://github.com/torvalds/linux/blob/v6.14/include/uapi/linux/perf_event.h)
* [Linux6.14 i915 PMU](https://github.com/torvalds/linux/blob/v6.14/drivers/gpu/drm/i915/i915_pmu.c)
* [GStreamer1.28.7 H264](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvah264enc.c)
* [GStreamer1.28.7 VA output/drain](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvabaseenc.c)
