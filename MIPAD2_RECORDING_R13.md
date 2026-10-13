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

## r13b start-capture stall and r13c isolated diagnostic repair
Preview starts at native1920x1440/30; synchronous start-capture geometry change
stalls before diagnostic timer can be installed. Zero-byte open output, no
encoder frames/no timed-action-start log. Exact candidate service stopped;
camera fd idle and zero-byte failure preserved. No reboot/default change.
R13c fixes native source caps toNV121920x1080/30 before initial PLAYING,
keeps camerabin video-capture-caps raw-format-only and viewfinder-capsANY.
Recording no longer asks to resize the active source. Separate720p30 display
scaler remains. THIS FLAG IS RECORDING-DIAGNOSTIC ONLY: fixed source also
limits photos in that session to1080p until a mode-specific native switch is
implemented/tested. Not promoted as general camera or Android photo parity.
Default non-opt-in source/recorder/photo behavior remains unchanged.
Actual native/encoder/file dimensions and recording throughput still pending.

## Actual r13c automated throughput gates, 2026-10-14

| Actual camera trial | Video packets | Effective fps | Maximum PTS gap | >100ms gaps |
| --- | ---: | ---: | ---: | ---: |
| Front10s |301|30.001|34ms|0|
| Front30s |918|30.002|34ms|0|
| Rear30s |908|30.012|34ms|0|

Native source, encoder input and file are all1920x1080, no720p upscale.
Mid-record VIDIOC_G_FMT readbacks confirm native1920x1080; H264+MP3 audio.
GPIO camera switching goes front to rear without source geometry regression.
All three software video/audio decode gates pass after camera/GUI idle;
video null validation preserves demux timebase to avoid unrelated integer
frame-rate rounding. No playback or media transfer. 33 framework tests pass,
full release build/private artifact installed; prior artifacts/default launcher
unchanged. Source21198c731e009601034b1ccb637ff012c6a98154 and private binary
454edaeea64a1f0e6f78d8dd75803c36ac21b680ab9a1d89c4c407a362fcc75b.

This meets only the measured1080p30 VIDEO THROUGHPUT gates in this scene.
Not a same-scene Android benchmark, not visible GUI/preview/player/rotation
proof, not AE/AWB/NR/AF/image-quality parity, not general photo-mode release.
Fixed-native diagnostic source limits this opted-in session's photos to1080p;
normal launcher remains r6. Need mode-specific native resolution switching,
pixel/scaling/rotation, repeated opens/suspend and broader hardware/boot/flash
regressions before promotion. Original1440p failures remain failed, and the
full objective is not complete. No global VA unblocklist/package upgrade.
