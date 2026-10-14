# R15c extended recording evidence, 2026-10-14

Actual existing R15c private binary on the R19 kernel. No kernel reboot,
default launcher promotion, normal settings change, playback, or media
transfer. Only sanitized scalar metadata is published. Four requested 30s
recordings use the real Snapshot/camerabin/UI/VA encoder pipeline, physical
input and mid-recording native 1920x1080 NV12 format checked separately.

| Sensor | Packets | fps from PTS | Max PTS gap | Identical adjacent decoded frames |
|---|---:|---:|---:|---:|
| front | 923 | 30.000000 | 34.334 ms | 0 |
| rear | 902 | 30.010992 | 34.000 ms | 21 |
| front | 905 | 30.000663 | 34.000 ms | 1 |
| rear | 902 | 30.011658 | 34.000 ms | 23 |

All four have no PTS gaps above 100ms and pass complete video frame-MD5 and
audio NULL decode. Maximum observed thermal-zone value was 70C. The process
exited gracefully with status0; both sensors returned to PMusage0/suspended;
both sensors' original exposure/gain were restored exactly while idle.

## Open finding: packet cadence is not unique motion throughput

Rear clips have 21/23 identical adjacent decoded frames out of 902 each.
Front clips have 0/1. These counts are NOT silently hidden by a 30fps label.
An identical decoded image alone cannot distinguish a held raw frame,
software duplication, or lossy compression of a stationary scene. Therefore
this is an open diagnostic finding, not proof of sensor loss or its cause.

A separate 960-frame rear V4L2->VA H264 capture, with full active NV12-plane
MD5 on-device and no videorate, produced960 distinct raw frames and960
distinct decoded frames. After the first60 frames, maximum raw PTS gap was
33.858ms, sequence steps always1. This different no-preview/no-AE pipeline
did NOT reproduce UI decoded repeats. It does not prove a fix in Snapshot.
Raw per-frame digests and media remain on-device; only counts are exported.

Existing H264 encoding profile already declares variable_framerate(true).
DOT topology shows videorate only on the drop-only display branch, not the
encoder branch. Do not remove preview throttling based on an unverified
assumption that encodebin inserted constant-rate padding. GStreamer's
videorate documentation explicitly distinguishes drops and duplicates:
https://gstreamer.freedesktop.org/documentation/videorate/

Next: opt-in same-UI-pipeline source/encoder-input frame fingerprints with
recording/segment epochs, not just lifetime max gaps. Keep diagnostics out
of the default low-overhead path. Visible motion, playback smoothness,
automatic3A/color/AF and same-scene Android parity remain unproven.

See MIPAD2_RECORDING_R15C_SOAK.json. Candidate remains private/not promoted;
the full kernel/port objective is still active.
