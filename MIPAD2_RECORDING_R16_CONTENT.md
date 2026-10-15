# R16 same-process content diagnostic candidate

Requires BOTH MIPAD2_TRACE_CAPTURE=1 and MIPAD2_TRACE_CONTENT=1. Disabled
by default. Only native-source and encoder-in map readable NV12 frames;
other trace boundaries and ordinary timing-only tracing do not map pixels.

GstVideoFrame readable maps honor per-buffer GstVideoMeta plane offsets and
strides. MD5 covers geometry and every active Y/UV row, excludes padding.
Only aggregate counters/costs are logged. No digest, pixel or media export,
buffer reference retention, write mapping, camera control or pipeline change.
This is a diagnostic fingerprint, not a security/integrity protocol.

A bounded256-entry native history contains scalar IDs/PTS/digests only.
It resolves matching content using PTS when possible, treats repeated-image
ambiguity as ambiguity, and permits PTS rebasing without assuming duplication.
Native segment/flush/pad changes clear history, stale owners cannot append.
Encoder segment epochs reset counters; EOS reports final partial intervals.
Missing/ambiguous matches are NOT evidence of dropped frames. Source index
skips, reuse and backwards movement need contextual inspection before a claim.

Mapping+hashing overhead is measured per boundary; frame rate/cadence must
be verified against a diagnostic-off control. No performance claim follows
from build/host tests. No new runtime observations have been made yet. Device
has reverted to stable bqdiag1; do not mix its evidence with the R19 baseline.
Use a new immutable R16 private prefix, preserving r6 launcher and R15c.

Reference: https://gstreamer.freedesktop.org/documentation/video/video-frame.html

## Pre-deployment review: R16a

Initial R16 full framework/release compiled, but extra exact-source review
found a stale old-pad begin/flush could steal a newer source epoch. A new
RED test reproduced it; begin now rejects older/zero owner IDs. The first
R16 build was not packaged or installed. R16a adds real GstVideoMeta-offset/
stride mapping and NO_REF-map lifetime tests, beyond pure hash/history tests.
No device reboot or candidate launch is performed while the user stays on
the old stable kernel. Pixel fingerprints remain diagnostics, not a fix.
