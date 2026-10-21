# Functional plugin source bundle (candidate, not an image package)

`build_plugins.py` builds only three existing functional changes against the
official GStreamer **1.28.7** source archives:

- base/encoding: reset stale streamcombiner EOS across camerabin READY reuse;
- good/videofilter: initialize identity videoflip passthrough correctly;
- bad/VA: retain the opt-in `MIPAD2_VA_ASYNC_DEPTH=2` live encoder delay proposal.

Archive, patch and resulting changed-source SHA-256 values are pinned in the
recipe. Official archive URLs are recorded in each receipt. The VA setting is
still opt-in, not globally enabled. No Snapshot app, AAC plugin, V4L2 diagnostic
plugin, tracer, launcher or CPU/GPU policy is installed. The working four-clip
R17h stack still contains a private V4L2 plugin with diagnostics OFF; stock-V4L2
equivalence and this newly built bundle require separate device validation.

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
3.12+, git, Meson, Ninja, C compiler, pkg-config, GStreamer core/base/video/bad
development packages reporting **exactly1.28.7**, and VA/DRM development inputs.
Meson uses release builds, `--wrap-mode=nofallback`, disabled auto-features/tests/
examples/docs/NLS, and builds only each named plugin target. Default `--jobs2`;
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

13 contract tests pass in each mode; all three official archives/checksums and
patch results were actually verified on2026-10-21. The current WSL build host
reports GStreamer1.28.4 and was rejected before output creation. Source-only
preparation succeeded; full compilation and portable-bundle device qualification
remain pending. Existing older private plugin build/device evidence does not
qualify this new packaging path. Do not point the default launcher or rootfs at
this output until matching-ABI build and isolated dual-camera/recording/photo/
reopen/resume tests have passed. Keep stable recovery and launcher unchanged.
