# Opt-in source-stage diagnostics, not a driver fix

See ../../../MIPAD2_CAPTURE_DECOUPLE_DEVICE_20261019.md and aggregate evidence.
Helper is LGPL-2.1-or-later, matching the enclosing GStreamer plugin.
Patch applies to the prior private stamp diagnostic source, NOT pristine upstream.
Exact before/after hashes and build provenance remain in the local build receipt.
It changes neither PTS/caps/flow nor camera controls. Never replace system plugins;
use an exclusive private plugin directory and registry, exact hash and ldd/factory
guards. A source build/pass is not an installation or hardware efficacy claim.

For the standalone helper test, copy the included helper as wait-helper.inc next
to test-wait-helper.c; compile against GStreamer/GLib with cc -std=c11 -O2 -Wall
-Wextra -Werror and pkg-config --cflags --libs gstreamer-1.0. Separate processes:
MIPAD2_V4L2_WAIT_DIAG=0 -> zero lines;
MIPAD2_V4L2_WAIT_DIAG=1 -> exactly two slow lines, no fast/uninitialized-scope line;
MIPAD2_V4L2_WAIT_DIAG=1 MP2_HELPER_CASE=cap -> exactly 128 slow lines.
This uses actual GStreamer objects and cleanup behavior, not camera imagery.
Scopes include scheduling/nesting; parent allocation normally waits for frames.
Only exact recording/driver/VB2 lineage with matching clock domains permits gap
attribution. No raw frame buffers or pixel digests may be exported.
