# Mi Pad 2 adaptive preview exposure (2026-10-10)

## Evidence and scope

Android sensor sources pinned at `d2afb1de9a761ff01dafdf9efe4736e521ab71f6`:

- [OV5693](https://github.com/latte-dev/android_kernel_xiaomi_latte/blob/d2afb1de9a761ff01dafdf9efe4736e521ab71f6/drivers/external_drivers/camera/drivers/media/i2c/micam/ov5693.c): integration time is in lines, analog and digital gains have distinct register paths, and exposure must respect VTS/margin.
- [T4KA3](https://github.com/latte-dev/android_kernel_xiaomi_latte/blob/d2afb1de9a761ff01dafdf9efe4736e521ab71f6/drivers/external_drivers/camera/drivers/media/i2c/micam/t4ka3.c): exposure and global gain are clamped independently; do not reuse OV5693 register units.
- The local reference copies match the pinned Git blobs byte-for-byte.

Windows reference package from [mipad2-win10-howto](https://github.com/brianwoo/mipad2-win10-howto) contains four Intel AIQB/CPF tuning containers, not the full 3A algorithm source. These binaries were inventoried and hashed, not executed or treated as decoded exposure algorithms. Unknown record fields are not guessed. Their contents are not embedded in this patch.

The existing Linux OV5693 driver shifts the public analog-gain control by four before writing the register. The T4KA3 driver uses different native codes. Query each sensor's actual V4L2 control range; never transplant Android's raw gain register values into a Linux control or change the kernel ABI for one application.

Architecture follows the separation described by [libcamera](https://docs.libcamera.org/master/libcamera_architecture.html): drivers implement sensor controls; userspace algorithms consume frames/statistics. [The upstream AtomISP experiment](https://lists.libcamera.org/pipermail/libcamera-devel/2025-May/050239.html) also uses decoded frame statistics for AEC/AGC because ISP2 statistics are undocumented. This implementation is a Snapshot-owned bridge, not a replacement system-wide libcamera IPA.

## Refactor

- Remove the front-camera fixed startup/reopen preset, rather than choose another hard-coded brightness.
- Meter the existing decoded preview before rotation: I420/YV12/NV12/NV21/GRAY8, YUY2, and supported 32-bit RGB layouts. Map actual video-plane stride and negotiated color range; skip corrupt/GAP/decode-only frames.
- Sparse center-weighted luma statistics (about 96x72 samples), normalized to full-range luma, plus clipped-highlight fraction. No saved images, second capture fd, shell loop, gst-launch process, or background global service.
- Bounded one-packet mailbox drops frames instead of blocking preview. Native control ioctls run in a worker, at most one update per 400 ms; 800 ms startup/switch warmup, stale-packet rejection, deadband and damped steps prevent feedback from assuming instantaneous register effects.
- Exposure-first in darker scenes, gain-first reduction in bright scenes, both strictly bounded/quantized by live driver queries. No VBLANK, digital-gain or white-balance changes. Feedback on native gain codes is not claimed to be a calibrated T4KA3 physical gain curve.
- Stop/unmap/dispose cancels the session before pipeline shutdown; switches invalidate old statistics. Control writes are serialized with session changes. Errors disable AE for that session rather than issue repeated invalid controls or close the preview.
- Rear contrast-AF sweep holds AE; old hold guards cannot release a new camera session. Existing OTP/color calibration remains unchanged.

## Tests completed before device activation

14 workspace tests passed (11 new, 3 existing): real GStreamer frame/caps mapping, padded strides and limited/full ranges, invalid buffers, highlights, bounded mailbox, session epochs/pause/hold, driver ABI structure sizes and request values, limits/quantization (183040 cases), and a delayed-feedback synthetic light model. Release build, rustfmt and git diff checks passed. First closure-syntax build failure was repaired; its log is retained, not counted as a pass.

These tests do not establish real optical convergence, 50/60 Hz flicker suppression, motion blur policy, AWB, ISP2 statistics, full 3A, or compatibility for other camera applications. Hardware acceptance and CPU/latency measurements remain necessary. The separate kernel r9 has not been booted for this work.
