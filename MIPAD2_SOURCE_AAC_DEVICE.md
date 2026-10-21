# Source AAC / four-source-plugin device checkpoint

2026-10-21. Partial Linux6.14 camera/image integration, not full Android/Windows parity.

## Build provenance

The host's FFmpeg8 development ABI does not match the tablet's FFmpeg9.0.1.
The tablet has headers but no C compiler/pkg-config/Meson/Ninja, so no target
development packages were installed. Instead a private FFmpeg9.0.1 header/link
SDK was built from the official signed archive; signature verified against
FCF986EA15E6E293A5644F10B4322F04D67658D8 in a NEW isolated keyring. Tarball SHA:
cf38e0e28c7e5605942c4a77755349b0145804a397af37eb1fb4c77cb237f635.
No host/target system library replacement. Minimal SDK libraries NOT deployed;
the target keeps its full original FFmpeg9.0.1 runtime.

New build_aac.py pins gst-libav1.28.7 archive
58da51dd39ecf1cf6faade34cc6412001be2e2e145bca8ae0f45336f60a36ab2,
exact GStreamer1.28.7 and codec/format63.1.101/filter12.1.101/util61.1.101,
plus the existing RPATH-safe exporter. It never silently substitutes host8.x.
Actual release/nofallback source build succeeded. Export SHA:
b122c917871b769cff7d2aa2d6fee8eedaf170fd92f33f36ea534953aa392d61.
No RPATH/RUNPATH. 11 new AAC+17 existing exporter contracts pass normally/`-O`.

## Target tests

Installed only one new versioned source AAC plugin, preserving all old artifacts
and system packages. Native target ldd-r and explicit avenc_aac loading succeed.
A synthetic-noise pipeline (no microphone/speaker) produces AAC-LC48k/stereo,
101packets/2.154437s/188922bps and decodes without errors. This is not by itself
proof of microphone capture or real video recording.

Then a NEW camera process uses all four source-built plugins (encoding,
videofilter, VA, AAC) plus stock V4L2. Actual source AAC module mapped and old
temporary AAC/functional/diagnostic modules absent. Same retained R17h app/kernel
r26 as prior trials, no new application binary or kernel build. All native input
and full-quality settings retained: NV121920x1080 nominal30, H.26415Mbps requested,
AAC192kbps/48kstereo. No per-frame traces, source queue experiment, power policy
write, quality reduction, startup trimming or timestamp rewrite.

| Sensor | MP4 packets = decoded frames | PTS-derived fps | Max PTS gap | Actual AAC bps |
|---|---:|---:|---:|---:|
| front | 911 | 30.001319 | 36.000ms | 192373 |
| rear | 923 | 30.011718 | 34.000ms | 192417 |

Both ENTIRE saved clips pass fps>29.5/maxgap<50ms, zero gaps>50/100ms, zero adjacent
exact decoded repeats and AAC decode0. Actual H.264 rates15.207–15.267Mbps; peak
sampled66C. Native clocks before/after front78125/2604/rear30; eight harness tests
pass normally/`-O`. Raw recordings, decode hashes and logs stay device-private.

## Post-test state and a retained stale-reference rejection

The first root checkpoint demanded old boot-test gains8/128 and FAILED. A separate
read-only observation shows the current trial had already captured gains4/250
BEFORE testing; after testing, controls exactly match that actual pretest state
(front1976/4, rear2486/250). No cause is inferred for the earlier change. The failed
old expectation remains retained; verification did NOT write controls to force
those old values. A new independent root checkpoint compares against the actual
trial snapshot and all other protection/PM/trace/error gates remain unchanged.

292protected/64R17h/threefunctional/sourceAAC/stockV4L2 hashes pass, both sensors
and ISP0/suspended; normal user settings preserved, no camera/profiler/traces
left, stable GRUB/launcher/thermal/no-idle unchanged. Selected whole-boot kernel
BUG/WARNING/Oops/GPUhang/NACK/stop-timeout counts0. No new reboot/RTC/tone.

## Package stage, not a finished camera image

Flash's existing three-plugin stager now optionally accepts a separately pinned
source AAC build root+receipt SHA together. Default three-plugin mode remains.
AAC mode independently verifies exact source state/ABIs/ELF/dependencies/hash and
stages one additional versioned plugin, never SDK/system libraries.14 stager
contracts pass normally/`-O`; actual FOUR source-plugin payload staging succeeds.
Full unittest discovery22cases passes normally/`-O` (separate older standalone
kernel preflight remains a distinct test).

This package tree still excludes Snapshot and desktop launcher and has not been
flashed. Same binary hashes were tested at isolated candidate prefixes, not as
a complete image. Source app/launcher packaging, foreground visible preview,
full-native photos, reopen/resume/cold boot and optical AE/AWB/focus/moving-scene
qualification remain open. Exact active hardware video encoder factory, actual
negotiated preview caps and native→encoded frame conservation are not measured
by these quiet trials. No full-camera, Android parity or overall-goal completion
claim; stable defaults remain.
