# Mi Pad 2 native raw source: scoped playback-layer bypass candidate (R17f)

Status: actual source compiled,8 focused tests and78 full release/allfeatures/
alltargets workspace tests PASS. Not hardware-tested, installed or defaulted at
this source commit. Full Linux6.14/flash/Android-Windows parity goal remains open.

## Why this change, not another producer queue

The R17e controlled hardware/DOT evidence shows physical system-memory NV12 still
traverses decodebin3 and its playback multiqueue. The added producer queue was
REJECTED: front6.679523s zero-driver-owned time and433ms median source->encoder
arrival; delays accumulate in MQ and downstream asynchronous record-path stages.
See MIPAD2_R17E_HARDWARE_20261020.md. Do not re-enable those source queues or
lower resolution, FPS, preview quality, H26415Mbps or AAC192k to conceal delay.

The physical native camera already produces the required raw NV12; no decoder,
demuxer or multi-stream selector is needed on this specific single raw stream.
Official v4l2src documentation distinguishes a direct raw pipeline from JPEG
camera output requiring decoding:
https://gstreamer.freedesktop.org/documentation/video4linux2/v4l2src.html
decodebin3 is an autoplugging decoder/demux/stream-selection bin, not a required
raw-camera transform:
https://gstreamer.freedesktop.org/documentation/playback/decodebin3.html
This design rationale is not a claim of proven device performance.

## Actual source wiring and strict scope

New opt-in MIPAD2_CAPTURE_RAW_DIRECT=1 is accepted only for MiPad2 DMI, the existing
verified AtomISP native camera descriptor, actual v4l2src /dev/video0, native NV12
profile, COPY1, ALLOCATION_GUARD1, and both source queues OFF. Invalid requests on
the target fail explicitly rather than silently switching to an unqualified path.
Other devices/default-off retain decodebin3 and its original dynamic video-pad /
oneshot error handler. The actual Viewfinder source builder uses the same tested
CameraTail helper; the descriptor is supplied from Camera's existing native-driver
selector, not inferred solely from the node name or source factory.

Direct path: native source -> native copy -> allocation guard -> existing mode
capsfilter -> NV12/system-memory output capsfilter -> wrappercamerabinsrc. There
is no decoder/multiqueue/producer queue in that source tail, no conversion/scaling,
clock rewriting, synthetic frames, leaky recording path or new worker thread.
The downstream camerabin/recording encoder/mux path is unchanged and still needs
acceptance; bypassing one layer cannot guarantee all downstream work meets30fps.
Existing shared-source/AE/preview probes and the source mode capsfilter remain in
place. Video1080p30, front still2560x1920/15 and rear3264x2448/15 modes are not
weakened. Unsupported/compressed/other raw or DMABuf caps are rejected only by the
explicit native direct tail; normal compressed camera decoding remains available.

## Actual host proofs and limits

Eight real-GStreamer tests exercise the production helper and policy:
- DMI/request/factory/path/profile/copy/guard/source-queue/AtomISP gates.
- No decoder or queue in direct tail; default retains decodebin3.
- Unsupported JPEG/I420/RGBA and non-system NV12 rejected by direct caps.
-12 padded NV12 frames preserve all bytes, stride/offset VideoMeta, PTS/DTS,
  duration, flags, offsets and stream-start/caps/segment/EOS; source and consumer
  are same task and synthetic native lease is released before downstream use.
- Actual1080p video and both full-photo caps/size pass repeated NULL/reopen.
- Allocation guard still returns FALSE without querying consumer; normal upstream
  LATENCY query still traverses. Initial test incorrectly counted both pre/post
  callbacks; failed receipt retained, final test uses pre-query PUSH explicitly.
- Responsive FLUSH_START cancels blocked consumer/producer in<1s, FLUSH_STOP starts
  fresh stream, only already-delivered old frame0 can precede all12 new frames;
  no stale queued tail. The5s mock timeout is not accepted as a liveness proof.
- Legacy compressed-camera path actually encodes/decodes3 synthetic JPEG frames,
  links its dynamic VIDEO pad, reaches raw output and real EOS with geometry intact.

These are synthetic host tests, not camera sensor/MMAP hardware acceptance or
foreground UI/latency/physical AV/Android quality proof. No new device recording,
kernel/boot/default change or media export was performed for these tests. Tablet
recording reserve is currently below2GiB; no new recording pair may start until
legitimate space and all original preflight gates pass. Preserve all current
samples/logs. R17e rejection remains valid; R17f must pass its own real dual-camera,
mode/switch/reopen/photo/record/flush/PM, conservation/availability/latency gates
before any default promotion. This candidate alone does not complete the goal.
