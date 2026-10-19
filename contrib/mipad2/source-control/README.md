# Bounded source control-path tracer

See ../../../MIPAD2_ALLOCATION_GUARD_20261019.md. Explicit absolute CLOCK_MONOTONIC
ABI2, 64-byte records, 512x512-byte object dictionary; never confuse with old pacing
ABI1 relative callback timestamps. No payload map/ref or per-frame file I/O.
Record only types/return values/buffer scalar timing, not event/tag/query contents.
Compile gstmp2control.c -shared -fPIC and gst_control_trace_test.c with GStreamer
pkg-config flags, -std=c11 -O2 -Wall -Wextra -Werror. Use an EXCLUSIVE private plugin
directory/registry. GST_TRACERS=mipad2-control, MIPAD2_CONTROL_FILE=absolute fresh
path, MIPAD2_CONTROL_CAPACITY=65536; MP2_CONTROL_TEST_SLOW=0 normal or1 deliberate
event/query60ms delays. Capacity16 deliberately overflows and must fail integrity.
The decoder inspects ONLY AFTER producer exits; no duration for censored calls.
Application capture should retain root/temperature/storage/serial/trace teardown
guards and validate real source TID/Tgid/exe/env before own-instance filters.
Keep raw syscall args/owner tokens/kernel traces and real camera media on-device;
export only qualified exact-lineage scalar summaries, never nearest-frame guesses.
