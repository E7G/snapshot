# R17i: opt-in bound on the real VIDEO encodebin input queue

Hardware performance/compatibility NOT YET VALIDATED. Default OFF, stable
launcher unchanged. This is a different boundary from rejected native capture
producer queues. See MIPAD2_R17H_DEVICE_VALIDATION.md: real R17h cadence passes
but source-to-encoder median is166.976/166.346ms, of which actual named encode
input queue sink-arrival->worker-push accounts for162.369/161.885ms. Snapshot
queue depth4; neither interval alone is isolated residence or optical latency.

MIPAD2_ENCODE_INPUT_ONE=1 requires MiPad2, actually validated RAW_DIRECT,
CURRENT_CAPS=1 and RAW_ASYNC=0. Direct validation already requires physical
AtomISP NV12, requested Android H264/hardware profile, copy/allocation guard and
disabled source producer queues. It does not enable those conditions itself.

One per-viewfinder observer attaches to actual video-encodebin/encodebin.
Resolve each numeric video ghost SINK's actual queue target, optionally through
the existing single-segment identity. Validate direction, factory and ownership;
do not use auto-generated queue2 as a selector. Only that existing INPUT queue
gets max-size-buffers1/bytes0/time0. It must be pre-streaming/inactive, nonleaky,
not flush-on-eos and have no minimum threshold; otherwise reject/log, not flush
or force it to fit. Audio input, JPEG image encodebin, encoded/mux output and
preview queues are not changed. No buffer/PTS/duration/offset/quality/event/EOS
edits, added queue, worker or private cap-cache. Ordinary GStreamer backpressure
remains and may expose throughput problems; this is not an acceptance claim.

New video pads after profile rebuild resolve new targets, not stale queues.
Signal closures capture neither their owning bin nor child queues. An existing
encodebin is scanned too. Root install requires a pre-streaming real camerabin;
the MiPad2 validated camera-source path installs once per viewfinder lifetime.
Other devices/unrequested and invalid native paths remain unchanged.

Seven focused actual-GStreamer tests and102 release workspace tests pass:
32-condition scope truth table; direct/single-segment ghost targets with arbitrary
queue names; audio/image/wrong owner exclusions; live/leaky/flushing/threshold/
wrong identity rejection with no property change; real encodebin video profile
while audio and mux queues keep their original values; profile rebuild without
retained old queue/reference cycle; real JPEG encodebin with a slow consumer,
two real input dimensions, all16 buffers' exact input bytes/PTS/duration/offsets,
all encoded PTS/counts and EOS. Hardware NV12/H264/AAC and photo/resume tests
are still required. Build host has encodebin/jpeg/vorbis/vp8/matroska factories,
but no camerabin factory: root camerabin observer is compile-checked only until
actual tablet validation. No factory availability is replaced by a mock.

Initial host compile failed only on fixture naming API; retained logs and
receipts. Revised fixtures set names before parent attachment; tests were rerun.
No device launch or success is inferred from host tests. CURRENT_CAPS alone
still has known latency/ownership regressions; do not promote R17h or R17i yet.

Primary references:
- https://gstreamer.freedesktop.org/documentation/coreelements/queue.html
- https://gstreamer.freedesktop.org/documentation/encoding/encodebin.html
- Exact1.28.7 gstencodebasebin.c1573..1602 creates/configures input queue and
  ghost target;1801..1809 synchronizes children before exposing the ghost pad.
  A late dynamic active target is deliberately rejected, not silently resized.
  https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst/encoding/gstencodebasebin.c
