# Reset stale combiner EOS across READY reuse

See [measured evidence](../../../MIPAD2_ENCODING_EOS_RESET_20261019.md).
This is an encoding-plugin companion fix to the opt-in live VA depth2 change.
Private repeated-recording tests conserve all observed input frames and remove
the premature-EOS/drain failure. Post-resume throughput and deep residency remain
RED; this is not a default-promotion or Android-parity claim.

Apply the patch to a pristine, verified gst-plugins-base1.28.7 source directory.
Compile the64-epoch regression before and after applying it:

```sh
cc -std=c11 -O2 -Wall -Wextra -Werror -Wno-unused-parameter \
  -Igst/encoding /absolute/path/to/combiner_eos_reuse_test.c \
  gst/encoding/gststreamcombiner.c -o /absolute/private/test \
  $(pkg-config --cflags --libs gstreamer-video-1.0)
/absolute/private/test
```

Original exit1 is the reproduced bug, not a passing regression. Patched exit0
must account for all64 buffers and defer EOS until both branches finish.
Build the complete private plugin with release Meson configuration, all optional
features/tests/tools/docs/examples/introspection/nls disabled except encoding;
Ninja target `gst/encoding/libgstencoding.so`. Never run a global install.
Verify target dependencies, factory filename, runtime module and hashes first.
Keep original/fixed plugins in separate fresh process-only plugin paths/registries.
The patch resets only after successful parent deactivation and never masks flow
errors or bypasses buffer lifetime/fences. Broader seek/flush/concurrent workload
coverage and end-to-end packaging are not claimed by this regression.

## Subsequent copy-path companion

The EOS conservation fix remains distinct from a newly reproduced default
videoflip identity-copy issue. See [copy fix and retained rear cadence failure](../../../MIPAD2_CAMERA_IDENTITY_COPY_20261019.md).
Both companions remain private candidates; no default/flash promotion.
