# Optional source-built AAC/libav plugin

`build_aac.py` builds the official gst-libav1.28.7 source, not a copied temporary
plugin. The archive SHA-256 and helper exporter SHA-256 are pinned. No implicit
downloads, system installs, SDK-library deployment or launcher mutation.

The current tablet uses FFmpeg9.0.1 (codec/format63.1.101, filter12.1.101,
util61.1.101). The ordinary host development files are FFmpeg8.x and must NOT be
substituted. The recipe enforces the four exact FFmpeg pkg-config versions plus
the matching1.28.7 GStreamer development ABI. A private source-built FFmpeg9.0.1
header/link SDK and the preceding private1.28.7 GStreamer SDK supply those inputs.
The FFmpeg archive signature was verified with the official release signing key
in an isolated keyring; no global keys/packages/libraries were replaced. See
[official release verification](https://ffmpeg.org/download.html#releases).

The private FFmpeg SDK is ONLY a build input. Its minimal component configuration
is not shipped or used to restrict the target's codecs. Exported libgstlibav.so
uses normal SONAME dependencies resolved against the tablet's existing full
FFmpeg9.0.1 libraries. Do not deploy private SDK libraries to the tablet.

```sh
env PKG_CONFIG_PATH=/absolute/ffmpeg9-sdk/lib/pkgconfig:/absolute/gst1287-sdk/lib/pkgconfig \
  LD_LIBRARY_PATH=/absolute/ffmpeg9-sdk/lib:/absolute/gst1287-sdk/lib \
  python3 contrib/mipad2/portable-plugins/build_aac.py \
  --archive /absolute/gst-libav-1.28.7.tar.xz \
  --output /absolute/new-aac-build --jobs 2
```

`--prepare-only` verifies/extracts pinned source without claiming ABI/build
acceptance. Full builds use release Meson/nofallback/noauto/doc+testsdisabled,
one explicit plugin target, and the existing patchelf exporter. Output must be
new; failed receipts/logs remain. Host-source-only receipt state is intentionally
distinct from separate target validation. The helper script itself is hash-pinned;
updating that helper requires deliberate review and an AAC pin update.

11 AAC contracts plus17 existing exporter contracts pass normally/`-O` (28 total).
The actual source plugin compiled, has no RPATH/RUNPATH, loads and relocates on
the target and passes synthetic-noise AAC-LC48k/stereo/192kbps encoding+decode.
Synthetic testing uses no microphone or speaker. The subsequent real camera
process uses all four source-built plugins and stock V4L2, with no old temporary
AAC mapping. Both full-quality saved clips pass the whole-clip cadence/decode
gates. See [device evidence](../../../MIPAD2_SOURCE_AAC_DEVICE.md).

Flash tooling supports explicit optional AAC staging alongside the three
functional plugins, with a separately pinned AAC receipt. Snapshot, desktop
launcher and a complete flashed image are still not included/qualified. Keep
stable defaults while completing foreground/photo/reopen/resume/cold-boot and
full application/image qualification.
