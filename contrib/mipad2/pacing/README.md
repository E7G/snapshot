# Bounded scalar pacing diagnostics (Linux/amd64)

These tools diagnose, not fix, cadence. See the source-linked
[real device evidence and retained failures](../../../MIPAD2_CAMERA_PACING_20261019.md).

## Tracer build and parser regressions

```sh
cc -std=c11 -O2 -g -shared -fPIC -Wall -Wextra -Werror gstmp2pacing.c \
  -o /absolute/private/plugin/libgstmp2pacing.so $(pkg-config --cflags --libs gstreamer-1.0)
python3 -m unittest discover -s /absolute/path/contrib/mipad2/pacing -p test_mp2_pacing_decode.py -v
```

Use a fresh registry and absolute, nonexisting private tmpfs file for an OWN test
process. Do not set these globally or change the ordinary launcher:

```sh
GST_PLUGIN_PATH=/absolute/private/plugin GST_REGISTRY=/absolute/private/fresh-registry.bin \
GST_TRACERS=mipad2-pacing MIPAD2_PACING_FILE=/dev/shm/unique-own-test.bin \
  /absolute/own/test/application
# Only after that producer is terminal:
python3 mp2_pacing_decode.py /dev/shm/unique-own-test.bin
```

Exclusive0600/no-follow file creation. Default524288 fixed64-byte records (~32MiB),
256 bounded pad descriptors; configurable16..1048576 slots. C11 lock-free reserve,
release-committed slots, bounds/loss/incomplete checks. Linux/amd64 little-endian
ABI only. Private process-lifetime mapping; no global tracing, camera data map,
buffer retention, signals or ptrace. Filtered pad metadata is cached once; only
initial allocation and initial notification touch filesystem/stderr.
Instrumentation has overhead. Push/chain intervals include nested downstream
work and are not exclusive CPU costs. Timestamp gaps across camera switches or
segment/state boundaries are not automatically recording losses.

`complete` is **committed-ring integrity only**, not clean lifecycle, physical
frame continuity or balanced calls. Always inspect `unmatched_pre/post`; unfinished
calls are right-censored, never zero-duration. Do not assert clean shutdown from
a complete file. Ten synthetic parser tests cover this distinction, corruption,
short files, invalid IDs/opcodes, incomplete commits/overflow and negative pairs.
Real host/target normal,20ms delayed,early-EOS and deliberately overflowing
pipelines verify hook counts, timings, flow propagation and failure visibility.

## Opt-in driver/pipeline clock diagnostic

Apply `gstv4l2src-stamp-diagnostic.patch` to checksum-verified gst-plugins-good1.28.7
in a private source tree. Build only video4linux2, without touching system files:

```sh
git apply --check /absolute/path/gstv4l2src-stamp-diagnostic.patch
git apply /absolute/path/gstv4l2src-stamp-diagnostic.patch
meson setup /absolute/private/build /absolute/private/source --buildtype=release \
  -Dauto_features=disabled -Dv4l2=enabled -Dtests=disabled -Dexamples=disabled -Ddoc=disabled -Dnls=disabled
ninja -C /absolute/private/build sys/v4l2/libgstvideo4linux2.so
```

Verify dependency relocation, actual v4l2src factory filename/SHA and process maps,
not just GST_PLUGIN_PATH. Add `MIPAD2_V4L2_STAMP_DIAG=1` only to that private test
process. Logs at most100 source PTS gaps >50ms, with previous/current driver stamp,
monotonic and pipeline-clock sample, base-time, offset and delay. No image reads,
timestamp/flow changes or forced success. Exact helper synthetic tests verify
enabled/disabled behavior,100-line cap and unchanged buffer PTS/offset; they are
not simulated hardware or evidence of a physical clock fix.
