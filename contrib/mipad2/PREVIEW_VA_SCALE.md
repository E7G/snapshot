# Experimental preview-only VA NV12 scaling (default OFF)

This extends the existing opt-in GL fit branch with a VA NV12 scaler BEFORE GL
upload and color conversion. It does not replace the current daily application,
driver, kernel or selected plugin. Nothing enables the new option by default.

## Process-local prerequisites

- Mi Pad 2 DMI and `MIPAD2_ANDROID_VIDEO_PROFILE=1`.
- Existing `MIPAD2_PREVIEW_GPU_SCALE=1` and its glshader factory gate.
- New exact `MIPAD2_PREVIEW_VA_SCALE=1` and a vapostproc factory.
- A private hash-verified VA plugin from the locked producer, including the
  [allocation CAPS invalidation patch](va-caps-cache/README.md). Load/register
  that exact plugin before viewfinder construction. Factory presence alone
  does NOT identify the fix. The default system registry has no VA factory.
  Tests explicitly load the private plugin and assert factory plugin identity.

The device-tested locked VA ELF SHA-256 is
`f7ff03158cb1aacc35bf93bbcf9d1fe5f694963f3d420ff0c26ca31509454faf`.
It is not the recipe-directory hash or the earlier isolated experimental ELF.
Do not run this renegotiation path with the old stale-allocation-CAPS plugin.

## Geometry and scope

Native NV12 CAPS are inspected on a serialized downstream CAPS probe. The
layout uses checked integer arithmetic and eight-pixel-aligned coded dimensions
inside 1280x720. Explicit reduced pixel aspect ratio preserves the exact native
display aspect ratio. A 720x1280 portrait input uses 408x720, PAR 135/136, then
the existing GL fit shader produces a 405-pixel-wide active image. Driver
add-borders is disabled; the GL shader provides opaque aspect-preserving borders.
The output is DMA_DRM NV12, Intel Y-tiled modifier 0x0100000000000002, DMABuf
memory, not the previously rejected ordinary-memory VPP route.

Only changed layout CAPS are assigned. The probe retains a weak reference to
the filter; invalid format/layout posts a negotiation error, not a silent
stretch or a quality/resolution downgrade. Ghost pads expose both input and
output. Existing preview frame-rate, rotation and camerabin conversion policy
remain unchanged. Native capture/JPEG/encoder caps, bitrate, quality, clocks,
driver, AE and audio paths are not modified.

## Validation and open gates

The exact compiled Rust test binary passes all 11 GL/VA preview tests on the
tablet, without skips. They cover all opt-in scopes, checked layout, non-square
pixels, serialized CAPS, weak lifetime, rejected invalid format/bus error,
existing GL/rotation links and conversion flags. The ACTUAL production VA and
GL bins run one synthetic pipeline through nine native input/DAR transitions,
including 1080p, both full-resolution photo sizes and portrait. All quadrants,
right/bottom full-field-of-view markers, opaque borders, active dimensions and
nine real DMABuf output buffers pass. This is not a live GTK/camera test.

A separate 21-case synthetic quality comparison used the exact locked plugin:
all nine neutral-field comparisons are exact; the 1080p ramp is exact, and the
two full-resolution ramps have maximum RGB error 2. Fine luma/chroma patterns
differ substantially from CPU resampling (worst luma mean absolute error about
79.81/255). This establishes differences, not which path is closer to an ideal
resampler or Android. Raw frames and diagnostic logs stay private.

Still open: full application build/packaging, normal physical camera preview,
native photo/video transitions, reopening/rotation/resume, sustained recording
and frame pacing, real-scene color/detail, thermal/power comparison and OEM
quality parity. Earlier Python prototype failures and the stale-pool negative
control remain failures. No system plugin or daily launcher is replaced.
