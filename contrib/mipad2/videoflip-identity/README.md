# Default identity must be actual passthrough

See [measured results and remaining RED gates](../../../MIPAD2_CAMERA_IDENTITY_COPY_20261019.md).
Apply the patch to a pristine verified gst-plugins-good1.28.7 tree, then build the
COMPLETE videofilter plugin. Do not install globally or substitute a flip-only
plugin that removes other factories.

```sh
git apply --check /absolute/path/0001-initialize-default-identity-passthrough.patch
git apply /absolute/path/0001-initialize-default-identity-passthrough.patch
meson setup /absolute/private/build /absolute/source/gst-plugins-good-1.28.7 \
  --buildtype=release -Dauto_features=disabled -Dvideofilter=enabled \
  -Dtests=disabled -Dexamples=disabled -Ddoc=disabled -Dnls=disabled
ninja -C /absolute/private/build gst/videofilter/libgstvideofilter.so
GST_PLUGIN_PATH=/absolute/private/build/gst/videofilter \
GST_REGISTRY=/absolute/private/fresh-registry.bin python3 rotation-regression.py
```

Keep original/fixed builds, processes and registries separate. Expected original
exit1 is the reproduced default-copy failure; fixed exit0 must preserve all16
synthetic pixel/geometry cases and identity memory aliasing. The script reports
the ACTUAL plugin filename/SHA256; verify them against your built artifact.
`--public-api-control` is an explicit diagnostic control, not patched-code proof.
No camera/display/media output or global setting is touched by this regression.

Target dependency relocation, all four factories, actual module map and protected
stock hashes must be checked before hardware use. This fix does not authorize
unsafe buffer reuse/export, resolve the remaining74ms rear gap, or establish
Android/Windows parity. Seek/tag/concurrent lifecycle coverage remains broader
than this small regression. Keep full-quality/whole-clip gates and rollback paths.
