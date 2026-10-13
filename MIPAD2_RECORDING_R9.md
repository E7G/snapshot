# Mi Pad 2 r9 preview/recording isolation candidate — hardware validation pending

2026-10-13. R8 actually captures NV12 at1920x1440 and writes rotate-90 MP4
display matrix through VA H264+MP3, but the user's clip is still396 packets over
18.166s, effective21.784fps, maxgap380ms,15 gaps>100ms. Every two-second window
is about19.5..25.5fps, not only startup. User still reports recording/drop stutter.
Observed app CPU from18 actual recording samples averages130.57%, peak154.97%;
the preceding preview samples are about116..124%. This is NOT fixed by r8.

Actual Rust PipelineTee + live90-frame1920x1440@30 GStreamer test uses a slow
10fps display branch and separate fast fake recording sink. Pre-fix source
retains all90 recording handoffs but takes9.046s, failing the unchanged4.5s
elapsed gate. Corrected source takes3.236s with all90 recording handoffs. No
timestamps alone are used to claim real time. This is simulated slow display,
not real GTK/encoder/sensor proof. Original runtime-red log is retained.

Only this viewfinder/optional QR tee becomes bounded/nonblocking: downstream
leaky queue, max2 buffers, unlimited byte/time thresholds. Older preview frames
may be discarded, but the recording branch is outside this tee and is NOT made
leaky. A slow display must not hold sensor buffers/backpressure encoding.
The camera's recording resolution/rate remain1920x1440@30. The pre-existing
display branch's20fps rate remains unchanged for this isolated comparison.

On exact Mi Pad2, process opt-inMIPAD2_PREVIEW_GPU_ROTATION=1 with glvideoflip
available moves preview rotation after display-rate selection and GL upload/
color conversion. Already RGBA GLMemory is rotated without gldownload. The
camerabin CPU viewfinder filter stays identity. Other platforms/non-opt-in or
missing factory retain old rotation. Photos retain their physical rotation,
video keeps r8's separately opt-in MP4 matrix path. No global environment or
default-launcher promotion is part of the candidate.
Primary GPU filter format/property contract:
https://gstreamer.freedesktop.org/documentation/opengl/glvideoflip.html

R8 observer remained waiting despite a finalized file because gallery holds
read-only MP4 descriptors (verified access_mode0, two FDs). This is an observer
gate defect, not proof recording was still writing. After the failed candidate
closed gracefully (status0, camera idle), the SAME worker finalized its report
and exited normally. No restart caused by an observation timeout. Future observer
must distinguish writing descriptors from read-only gallery playback.

Pending actual GPU preview correctness, tag/caps propagation, concurrent live
preview/audio/recording CPU/cadence/user responsiveness, photos, camera switching
and reopen regression. No full camera/kernel/boot/3A/hardware/flash completion.
