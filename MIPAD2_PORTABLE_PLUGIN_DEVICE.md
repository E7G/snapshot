# Source-built functional plugins: matching SDK, relocatable export, device checkpoint

2026-10-21. Partial camera/image integration, not full Android/Windows parity.

## Matching development environment, without replacing system libraries

The initial host had mixed GStreamer development versions (core/base1.28.4,
video/bad1.28.6), while the tablet has1.28.7. The exact build-ABI guard remains
unchanged. Built a NEW private host SDK from official1.28.7 core/base/bad tarballs
using release Meson builds, disabled automatic features/tests/examples/docs/NLS/
introspection and `--wrap-mode=nofallback`. Core tools enabled; base/bad tools
disabled. `--libdir=lib`, explicit private `--prefix`; installation ONLY into
that new prefix. No system package/library replacement or global environment
change. Core official tarball SHA-256:
`787329b2c5758e228a71d926a6dcf960bceaacca3cadd63874ba665dfcda013e`.

Private command environment prepends its `lib/pkgconfig`, `lib` and `bin` through
PKG_CONFIG_PATH, LD_LIBRARY_PATH and PATH. All four required pkg-config entries
independently report1.28.7 and the private prefix; private gst-inspect reports
1.28.7. Four original system-library hashes remain unchanged.

## Actual build and discovered export defect

The first actual exact-ABI build compiled all three existing functional patches,
but exported copies retained Meson build-tree RUNPATH entries, including a private
SDK path. That original receipt and binaries remain retained; they were NOT
deployed or labeled portable. The source builder now removes RPATH from each
new exported copy using patchelf, requires no remaining path, rejects dependency
filenames containing separators, and verifies the original build artifact's hash
is unchanged. Source archive/patch/resulting-source pins are unchanged.

17 host contracts pass normally and with Python `-O`, including real compiled
ELF export, untouched original binary, existing-output/non-ELF rejection, and
failure when RPATH remains. A fresh full build using the corrected recipe passes;
all three exports independently have no RPATH/RUNPATH or unresolved host deps.
Recipe SHA: `854e33239c0e358e708efc334d103cd4da31c743cc8feff461f0f7e325c3915e`.

| Plugin | Corrected exported SHA-256 |
|---|---|
| encoding | `7d20cbb4ec467dc50b4109789804bb7f1f402724d3a94f8a613415ac556cc006` |
| videofilter | `0715ad22b3afb7974cb5cfd68484746dcd7a6a219865444dac390e43e956d4a3` |
| VA | `ff4f8702f1d521e8845e4b395d79328bbacde2dccc59ffa02366fcd761cb9b38` |

## Target isolation and actual two-clip recording

Installed ONLY these three plugins into a NEW versioned candidate prefix.
Target-native `ldd -r` relocation checks and explicit gst-inspect loading pass;
no build paths remain. 292 protected files, stock V4L2 and old app/plugins remain
unchanged. No system package installation or launcher/default mutation.

Same retained R17h app/binary as earlier evidence, kernel6.14 integrated r26 built
from `6074419168fbb70aa5685e976874da6cf86d1fae`. This fresh process uses the new
three functional plugins plus **stock V4L2 1.28.7**, NOT the old private V4L2
diagnostic plugin. Actual mappings checked for all relevant new plugins/stock
V4L2, absence of old private functional/diagnostic mappings, and VA+i965 driver.
AAC still uses the previously verified external matching plugin; the tablet has
no system gst-libav package. AAC is NOT included in this three-plugin bundle.

Native NV121920×1080/nominal30, requested H.26415Mbps plus AAC192kbps/48kstereo;
rear→front,30seconds each, per-frame diagnostics/profilers OFF, natural original
thermal/governor policy, no CPU/GPU policy writes or quality reduction.

| Sensor | Packets = decoded frames | PTS-derived fps | Max PTS gap | Actual video/audio bps |
|---|---:|---:|---:|---:|
| rear | 916 | 30.011808 | 34.333ms | 14988702 / 191868 |
| front | 899 | 30.001670 | 34.333ms | 15287999 / 191974 |

Both ENTIRE saved clips pass fps>29.5/maxgap<50ms, zero gaps>50/100ms, zero adjacent
exact decoded repeats, matching packet/decode counts and AAC NULL-decode0. No
startup trimming/PTS rewrite/fitting. Positive explicit rational native clocks
verified before/after, rear30/1, front78125/2604. Eight harness contracts pass
normally/`-O`. Peak sampled temperature63°C. App exits normally0; original controls
and normal settings restored. Independent root checkpoint confirms sensors/ISP
0/suspended, no tracing/profiler owners, stable GRUB/launcher/guards preserved and
selected whole-boot kernel error counters0. No new reboot/RTC/tone.

## Packaging stage and remaining scope

Flash tooling now has a manual `stage_camera_plugins.py`: an explicit build
receipt SHA-256 is mandatory, not a guessed latest artifact. It independently
validates version/ABI, exact three components/bytes/hashes, x86-64 ELF, absent
RPATH and manifest-matching dependency filenames before creating a NEW package
root. Stages only three versioned plugins and a provenance manifest; no app/AAC/
launcher/system library replacement. Nine contracts pass normally/`-O`; staging
of these ACTUAL built plugins succeeds. This is not a built/flashed rootfs image.

The new tests jointly change three rebuilt functional plugins and stock V4L2:
they establish that complete selected combination's scoped recording result,
not isolated performance causality for each change. Foreground preview cadence,
exact active encoder factory, actual preview negotiated caps, native-to-encoder
conservation, moving-scene optics/AE/AWB/focus, full-native photos, repeat/open/
resume and cold boot still require qualification for this bundle. Raw media/
frame hashes/traces stay private. Full Snapshot/AAC/launcher source packaging,
image build/flash and broad hardware/Android-Windows parity remain OPEN.
