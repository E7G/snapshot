# VA upload-pool allocation CAPS invalidation

Pinned to GStreamer 1.28.7. This changes only gst_va_base_transform_set_caps:
discard the cached upstream allocation CAPS when negotiated CAPS change, along
with the upload pools already discarded by that method. A source can deliver a
new CAPS event without a new allocation query; rebuilding an upload pool from
the old proposal can silently crop a larger frame or reject a smaller frame.
The next allocation query may populate the cache normally again.

## Device validation

Two isolated plugins were built from the same copied source/build directory and
compiler options; only this patch differed. The previously selected plugin and
all system libraries remain unchanged. Separate private worker processes avoid
cross-loading VA plugin versions. The same synthetic NV12 input sequence uses
one pipeline, real DMA-BUF VA output, GL import/conversion and preview fitting.

The negative control completes only two cases. When input changes from
2560x1920 to 3264x2448, it recreates a 2560x1920 upload pool and loses a right-edge
field-of-view marker. Coarse quadrant colors alone had missed that cropping.
Without the marker check, a later portrait input fails as an invalid video
buffer. The patched plugin completes all nine input/DAR changes, with every
upload-pool geometry matching its current native input. All quadrant, opaque
border, aspect, right-edge and bottom-edge markers pass. Nine DMA-BUF buffers
and seven distinct output-layout updates are observed.

This is a bounded synthetic renegotiation regression, not complete camera,
recording, power, HDR, crop-meta, decoder, or OEM-quality qualification. Raw
frames and debug logs stay private. No application option is enabled by this
patch. H.264 encoding sources, async-depth controls, bitrate, driver and clocks
are unchanged. Application integration and broader regression tests remain open.

The full locked three-plugin producer was also rebuilt from the pinned official
archives. Its exact VA artifact was staged privately and passed the same nine
transitions and field-of-view checks; it was not replaced by the isolated
experimental binary. All 42 plugin/application/AAC producer contract tests
pass. The shared-helper hash pin used by the AAC producer was refreshed; AAC
build logic and FFmpeg ABI requirements are unchanged. The source-validation
receipt is a fresh prepare-only result and does not claim compilation itself.
