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

## R14 initial failure and R14b paired ISP lease

R14 private one-shot phototest FAILED: not-negotiated before full native frames,
no JPEG. NativeNV12 sizes limited to1920x1440 by PREVIEW run mode. Cached G_FMT
1920x1080 after failure is NOT evidence of actual restarted preview. Same
candidate then quit cleanly; camera idle, normal settings/launcher unchanged.

Idle R19 mode-only probe: STILL2 changes front enumeration to2608x1944;
originalMode3/video/size enumeration restored. Native full2560x1920 S_FMT
succeeds in STILL2, but the first direct12-frame test at defaultVFPP0 yields
only one frame then select timeout. Its failed result remains failed/restored.
Changing only VFPP to2 (disable to lowlatency/single main output) while retaining
STILL2/full native geometry/15fps request returns12frames; front raw source
2576x1936 for2560x1920 output, rear3280x2464 for3264x2448 output. Both tests
restore originalMode3/VFPP0, both sensor shapes/full mutablecontrols and video.
Rear CLI sample reports8.08fps despite15fps request: do NOT claim15fps physical
throughput or erase this performance gap. Rear report's copied scope label
incorrectly says front2560; its input1/T4/raw/output fields are authoritative.

R14b acquires a qualified AtomISP control-only node after ownerpipelineNULL,
saves and sets both mode2/VFPP2 before caps probing. Restoration attempts both
original controls and verifies readbacks, returns failures, no hidden success.
Managed capture lease restores only afterNULL, including image-done, cancel,
close and pipeline-error paths; failed shutdown is fail-closed, not an unsafe
control write against a running pipe. Partial entry rolls back with explicit
transport errors. Generic transport mocks cover18 original-mode/VFPP/exit
combinations, failed entry/readback/restoration cases; actual GTK cancellation
races are NOT covered by these mocks.

Full photo readiness now counts only exactNV12 full-resolution buffers with
sufficient bytes, noCORRUPTED/GAP flags. Thirty-nine host/framework tests pass;
the initial malformed ANY-caps negative case emitted GStreamer criticals, so
fixed-caps/positive-size checks were added before parsing. Final full-build
tests must reverify this corrected version. Hardware mode/stream probes are
not proof of successful R14b GUI JPEG capture, decoder/rotation, latency,
repeated shots/restore/recording/PM or Android optical parity. Candidate remains
private and unpromoted until those actual gates pass. No reduced-size fallback.

## R14b real failure / R14c display-rate ceiling

R14b front photo FAILED with no JPEG. Actual ISP mode2/VFPP2 and native
2560x1920 NV12 source plus2576x1936 sensor confirmed, but decodebin3 dynamic
video pad cannot link downstream: no common format, not-linked stream error.
Both ISP controls restored3/0 afterNULL; candidate does not claim preview
recovery from cached G_FMT. Failed trial closed gracefully0, no video owner,
normal settings untouched. Preserve failed report and immutable R14b package.

Source inspection identifies two display-only capsfilters requiring30fps
(720p CPU scale and GLMemory RGBA output) despite drop-only videorate and
15fps transient still source. Official videorate contract: drop-only produces
no duplicates; max-rate is a ceiling, not a command to synthesize30fps:
https://gstreamer.freedesktop.org/documentation/videorate/

R14c uses1..maximum framerate ranges in BOTH display-only capsfilters, keeping
native full still15 request, recorder1080p30 and GL-memory-only rotation.
Real host buffer regression: old fixed30 branch fails; corrected front/rear
full-native15fps sources and normal1080p30 source reachEOS, negotiated720p
display remains15/30 respectively, four buffers, zero duplicates. Also tests
GL output rate-caps containment. This is not actual hardwareGL/JPEG proof.
Private candidate build and actual front/rear JPEG/repeated shot/30fps restore
gates remain required; Android live optical parity is not established.
