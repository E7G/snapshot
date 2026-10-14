# R14 native still-capture candidate, NOT a promoted release

R13c native1080p30 recording throughput is verified but its fixed source limits
still images to1080p. This candidate adds a shot-only full-resolution restart:
OV5693 2560x1920 / T4KA3 3264x2448, NV12/15fps during the short still stream.
Normal preview/recording remains native1080p30 with display-only720p scaling.
These JPEG sizes come from pinned Android camera3_profiles.xml; they are not
evidence of Linux optical quality or a live Android comparison.

Snapshot awaits full-resolution preparation before start-capture. The capture
is reserved across await, so existing switch/record/photo guards cover it.
It first stops the existing stream, fixes source caps before PLAYING, then
requires twelve new native buffers with matching full-resolution caps (bounded
eight-second wait, generation/error/cancellation checks). No synchronous
start-capture source resize or downscaled fallback is substituted. image-done
restores native1080p30. Preparation/capture failures unwind the native-photo
flag; pipeline errors stop rather than silently capture at a smaller size.
Default non-opt-in/non-MiPad2 behavior delegates to the existing photo path.

Only explicit private MIPAD2_ANDROID_VIDEO_PROFILE=1 + hardware H264 opt-in uses
this path. The private diagnostic picture action also requires existing
MIPAD2_DIAGNOSTIC_ACTIONS=1. No normal launcher/configuration changed.

Initial34 host/framework tests pass, including fixed front/rear JPEG geometry
and video/preview separation. This does NOT exercise the new asynchronous
hardware photo lifecycle, cancellation races, real JPEG save/decode, exposure
settling or responsiveness. Full build and real repeated front/rear shots,
native G_FMT versus JPEG dimensions (no upscale), restoration/recording/PM
checks remain required. 15fps is not a photo-mode preview downgrade: it exists
only between shutter request and completion. Actual shutter latency still
needs measurement and optimization versus Android; no parity claim.

Primary references:
- https://raw.githubusercontent.com/latte-dev/android_vendor_xiaomi_latte/aaf390134608e033e8c5b545568e68dab1ed2bdf/proprietary/etc/camera3_profiles.xml
- https://gstreamer.freedesktop.org/documentation/camerabin/camerabin.html
