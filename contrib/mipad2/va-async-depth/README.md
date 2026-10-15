# Unbuilt/unapplied VA asynchronous-output proposal

**Not deployed. Not a validated fix.** Actual R17b diagnostics isolate higher
video bitrate as the failing arm; AAC-only remains about30fps. Sampled GPU
frequency is already600MHz, with modest device I/O busy time; do not force
frequency or lower quality as a guessed fix. A joint recording thread sample
also observes i915_request_wait_timeout. This is not full causal attribution.

GStreamer1.28.7 gstvah264enc.c sets preferred_output_delay=0 for live sources
and4 otherwise. gstvabaseenc.c blocks pushing an output when the list exceeds
that delay, tries already-ready outputs, and its existing finish/drain flushes
the whole output list. The proposal opts into depth2 for live H264 only when
MIPAD2_VA_ASYNC_DEPTH is exactly2. Missing/invalid values keep0; non-live stays4.
Bitrate, quantization/profile, source/preview resolution and frame rate are not
lowered. Existing pool/latency accounting adds the delay; that accounting still
needs actual pool/EOS/lifecycle validation on the device.

The included C test is only a decision and FIFO-count model. It does not prove
that upstream source accepts the patch, that the full plugin compiles, or that
surface ownership, retained VB2 buffers, EOS, image quality or throughput work.
Do not copy a model pass into those gates. Source retrieval has failed: first
archive was truncated to1MiB; ranged attempts and both WSL/native Git TLS failed.
Those failures and the original partial archive are retained. No incomplete
archive was extracted or used to build a plugin. WSL-only libva development
dependencies were installed; tablet packages were not changed.

Next: obtain and verify complete pinned1.28.7 source; apply/check; build a fresh
private plugin; run its flag-off baseline and flag-on joint15Mbps/AAC tests with
the same R17b binary, no PMU, actual reported delay/pool/latency, complete frame
and audio decode, EOS/reopen/resume, and protected original plugin hashes. Keep
all earlier slow recordings and Mesa/startup failures. Never promote this patch
solely on model tests or because an old non-live path already used buffering.

Primary references:
* https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvah264enc.c
* https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvabaseenc.c
