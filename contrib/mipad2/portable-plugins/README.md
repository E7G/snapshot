> 7.2 rational VA frame-rate fix: [actual source, hardware bitrate proof and FAILED long cadence gate](../../../MIPAD2_VA_FRAME_RATE_7_2.md). Private candidate only; no default promotion.

> Optional rootfs integration now exists: [source payload merge, native ABI tests and remaining foreground/display gates](../../../MIPAD2_CAMERA_ROOTFS.md). Default camera and boot remain unchanged; no full image qualification.

> Source Snapshot app build and separate four-source-plugin launcher package now exist: [scope, tests and remaining foreground gate](../../../MIPAD2_SOURCE_APPLICATION.md). Not a promoted default or complete image.

> Optional source-built AAC now available: [build/ABI requirements](AAC.md) and [scoped device evidence](../../../MIPAD2_SOURCE_AAC_DEVICE.md). The three-plugin default below remains unchanged; complete app/launcher/image integration is still pending.

# Functional plugin source bundle (candidate, not an image package)

`build_plugins.py` builds three functional plugins against the
official GStreamer **1.28.7** source archives:

- base/encoding: reset stale streamcombiner EOS across camerabin READY reuse;
- good/videofilter: initialize identity videoflip passthrough correctly;
- bad/VA: retain the opt-in `MIPAD2_VA_ASYNC_DEPTH=2` live encoder delay proposal,
  and pack bounded rational frame-rate parameters instead of truncating them.

Archive, patch and resulting changed-source SHA-256 values are pinned in the
recipe. Official archive URLs are recorded in each receipt. The VA setting is
still opt-in, not globally enabled. No Snapshot app, AAC plugin, V4L2 diagnostic
plugin, tracer, launcher or CPU/GPU policy is installed. The working four-clip
R17h stack still contains a private V4L2 plugin with diagnostics OFF; stock-V4L2
combination has since passed a scoped recording trial (below); broader qualification remains pending.

## Usage

Download the three pinned archives from their official source directories into
an input directory, retaining their original names. Check each against the
official adjacent `.tar.xz.sha256sum`; the script independently verifies the
pinned archive hashes again and never fetches network inputs implicitly.

```sh
python3 contrib/mipad2/portable-plugins/build_plugins.py \
  --archive-dir /absolute/path/to/official-archives \
  --output /absolute/path/to/new-private-output --prepare-only
```

For compilation, use a fresh output and omit `--prepare-only`. Requires Python
3.12+, git, Meson, Ninja, C compiler, patchelf, pkg-config, GStreamer core/base/video/bad
development packages reporting **exactly1.28.7**, and VA/DRM development inputs.
Meson uses release builds, `--wrap-mode=nofallback`, disabled auto-features/tests/
examples/docs/NLS, and builds only each named plugin target. Default `--jobs 2`;
accepted1..32. No `meson install`, package-manager mutation or `/usr` write.

Inputs are validated before creating output; existing output is rejected, not
deleted or overwritten. Archives reject traversal, links, special entries,
duplicates and oversized contents. Git's inherited repository environment is
isolated; a successful no-op patch cannot pass the resulting-source hash gate.
Failures retain receipts/logs; retry only into a new directory after inspection.
Successful source preparation is explicitly distinct from compilation success.

```sh
python3 -m unittest discover -s contrib/mipad2/portable-plugins -v
python3 -O -m unittest discover -s contrib/mipad2/portable-plugins -v
```

21 contract tests now pass normally/`-O`, including real ELF export tests. The
original source-only checkpoint rejected the host's mixed1.28.4/1.28.6 ABI;
that failure is retained. A private1.28.7 SDK has since been built, all three
functional plugins compiled, and build-tree RPATH removed from exported copies
without changing the original build artifacts. Explicit target relocation and
gst-inspect checks pass. A fresh dual-camera quiet recording trial using the
new three plugins and stock V4L2 passes whole-clip30fps/maxgap34.333ms checks.
See [the scoped build/device evidence](../../../MIPAD2_PORTABLE_PLUGIN_DEVICE.md).

This is still not a complete camera package. Snapshot, AAC and launcher are not
included; AAC remains an external previously verified plugin. The first build's
RPATH-bearing binaries and old source-only receipt remain historical evidence,
not a deployable bundle. New exports require patchelf and reject any remaining
RPATH or dependency filenames containing path separators. Foreground, full-native
photo, reopen/resume and cold-boot/image qualification remain pending. No default
launcher, system package or kernel was changed by this packaging checkpoint.
