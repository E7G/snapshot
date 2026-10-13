# Mi Pad 2 r13 Android-profile alignment candidate

Pinned MIUI-derived vendor camera3 profiles aaf390134608e033e8c5b545568e68dab1ed2bdf
list maximum processed video1920x1080, approximately30fps; separate photo JPEG
capabilities reach rear3264x2448/front2560x1920. This is declared metadata,
not a live Android throughput/quality benchmark.
https://raw.githubusercontent.com/latte-dev/android_vendor_xiaomi_latte/aaf390134608e033e8c5b545568e68dab1ed2bdf/proprietary/etc/camera3_profiles.xml

Previously tested1920x1440 is a larger/non-stock video workload. R12 at that
size remains failed: front with GL flip21.157fps/maxgap85.667ms; without flip
21.530fps/maxgap74ms; rear23.489fps/maxgap62.667ms. No gaps>100ms in those
clips, but NOT30fps. Extra GL flip removal is not established as the cause.
All failures are retained. Mid-record DOT confirms actual NV12 recording path,
ref-frames1/bitrate2048; no hidden I420 conversion claimed.

Process opt-in MIPAD2_ANDROID_VIDEO_PROFILE=1 on exact device/H264/hardware
requests recording NV121920x1080/30 and independently scaled preview
NV121280x720/30. Recording is not reduced to preview resolution. Display-rate
limiter changes from15 to30 for this opt-in. No change to image-capture-caps,
though actual photo geometry after recording still needs regression testing.
This explicitly changes recording geometry/aspect ratio to match the declared
Android video profile; it does not repair or pass the previous1440p tests.
Default launcher/global settings remain unchanged; candidate-only until actual
file/frame timing, preview/scaling/pixel correctness and quality gates complete.
Bitrate still2048kbps, audio enabled; source-copyOFF and ref1/QoS switches
remain separately opt-in. No automatic playback or acoustic test.

## r13 initial negotiation failure, retained
Initial source-ready gate fails: decodebin3 video pad cannot link to shared
post-decode capsfilter (no common format). Zero new recording, no success claim.
Fixed1280x720 camerabin viewfinder-caps conflict with shared source/video
negotiation. R13b keeps viewfinder-capsANY and inserts videoscale+NV12
1280x720/30 caps only in the optional display bin, after rate selection and
leaky queue, before GL upload. add-borders=true preserves differing aspect
ratios rather than stretching pre-record/photo preview. Record caps remain
explicit1080p30. Actual native input and encoder/file geometry must be audited
so a720p source upscaled to1080p cannot be accepted as native1080p capture.
Default non-opt-in preview adds no scaler/caps elements.
