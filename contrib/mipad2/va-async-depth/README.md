# Opt-in VA queue experiment: no default promotion

The corrected patch now applies to exact upstream gst-plugins-bad1.28.7 and
the full VA plugin was built and tested privately on Mi Pad2/R19. It improves
uninstrumented high-quality throughput to about30fps, but rear camerabin drain
returns EOS with a two-frame input/packet mismatch. This lifecycle gate is RED.
The2040-case C decision model and synthetic120-frame cases do not cover that
failure. Keep the feature off by default; no global installation/promotion.

See [full evidence](../../../MIPAD2_VA_QUEUE_20261019.md) and its JSON for exact
source/archive/plugin/patch hashes, cases, failure traces and remaining work.
The older patch revision failed real apply despite passing syntax-only checks;
that error is preserved in history. This revision fixes source context and adds
effective queue/pool/latency INFO logging. Exact environment value2 changes
only live H264 delay to2; unset/invalid/0 remains0, non-live remains4.

Reproduction build, from a pristine verified1.28.7 release source directory:

```sh
git apply --check /absolute/path/to/0001-proposed-opt-in-live-two-frame-output.patch
git apply /absolute/path/to/0001-proposed-opt-in-live-two-frame-output.patch
meson setup /absolute/private/build . --wrap-mode=nofallback --buildtype=release \
  -Dauto_features=disabled -Dva=enabled -Ddrm=enabled -Dtests=disabled \
  -Dtools=disabled -Dexamples=disabled -Ddoc=disabled -Dintrospection=disabled -Dnls=disabled
ninja -C /absolute/private/build -j2 sys/va/libgstva.so
```

Do not run a system-wide install. Target dependency closure/factory/hash checks
and fresh per-process plugin path/registry are required before any private run.
Next investigate downstream EOS flow and per-stream recording boundaries;
never mask errors or substitute a cadence/decode-only test for conservation.

## Subsequent encoding companion fix

The old-encoding rear failure above is preserved. A later matched-source
streamcombiner READY/EOS reset patch fixes the observed frame-conservation
failure in a private candidate. See [follow-up](../../../MIPAD2_ENCODING_EOS_RESET_20261019.md).
Post-resume front cadence and final S0I3 are still RED; no default promotion.
