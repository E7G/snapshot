# Mi Pad 2 recording r8 — process-opt-in candidate, not validated GUI repair

2026-10-13. Production device remains kernel6.14.0-mipad2-integrated-r18.
The r7 GUI clip uses real VA H264 and MP3, yet delivers275 packets over15.427s,
effective17.799fps, maximum PTS gap424.333ms and12 gaps above100ms. It is
1440x1920, whereas the camera input is1920x1440 I420: per-frame software
rotation/conversion is additional work absent from the earlier direct encode
benchmarks. The180s observation ended before this user recording and found
zero new clips; its idle CPU samples are NOT recording CPU proof.

Scoped rear300-frame real-camera comparison at1920x1440@30, single MMAP source:

| Path | Effective PTS fps | PLAYING s | CPU s | Sustained gate |
| --- | ---: | ---: | ---: | --- |
| I420 convert, identity | 29.814 | 10.267 | 8.277 | FAIL |
| I420 CPU90 rotation/convert | 25.052 | 12.187 | 11.217 | FAIL |
| NV12 CPU90 rotation | 28.948 | 10.556 | 9.148 | FAIL |
| NV12 GL90 rotation | 9.511 | 32.217 | 20.421 | FAIL |
| NV12 encode + MP4 matrix90 | 30.012 | 10.257 | 4.238 | PASS |

The explicit gate requires PTS within1% of30, maxgap<50ms and actual PLAYING
<10.6s. All cases decode, restore both sensor format/crop/full writable scalar
controls plus original video state exactly, and have no new dmesg output.
Failed paths are retained, not relabeled. Matrix clip remains1920x1440 coded
with display matrix rotation-90 (clockwise display1440x1920). It is not a
reduced-pixel-count/subsampled recording, nor a GUI+audio+preview proof.
Primary muxer support, including global/stream orientation tags and track matrix:
https://raw.githubusercontent.com/GStreamer/gstreamer/1.28/subprojects/gst-plugins-good/gst/isomp4/gstqtmux.c

Candidate uses exact Xiaomi Inc/Mipad2 DMI + hardware H264 + the process-only
`MIPAD2_MP4_ROTATION_MATRIX=1` opt-in. It prefers already-supported NV12 caps,
preserving all geometry/rate/fallback caps. The recorded video filter does not
physically rotate each frame; its MP4 orientation tag follows the user's selected
rotation. Preview and photos retain their physical rotation. Photos clear any
old video matrix tag to rotate-0, avoiding double rotation. Matrix capture keeps
orientation fixed until finalization; other platforms and non-opt-in/default
profiles retain their previous recording behavior. Unsupported players ignoring
MP4 display matrices remain a compatibility/UX validation item.

First source preparation wrongly expected two best_caps callers; actual source
has one. Two helper/API files had changed before the guard failure. Resume
applies only the not-yet-written viewfinder changes; no duplicate replay/reset.
Its second guard used a generic comment present in both photo/video methods.
Resume then targets the explicitly distinguished video tag call. These are
preparation-harness faults, not a hardware/application test result.

Pending: actual camerabin video caps/encoder/tag propagation, full portrait and
landscape recordings with simultaneous preview/audio, exact orientation pixel
semantics and supported player display, photos and switching/reopen regression,
measured GUI CPU/cadence/responsiveness/quality, and production opt-in promotion.
No default launcher/settings/package upgrades or reboot are part of this build.
Broader kernel/sensor/3A/boot/hardware/flash objective remains active.


## Built/private-installed checkpoint (same day, GUI recording still pending)

Built/package source: `41b931b65e8533d25c2e8f074321e2cfb07169fe`; offline full release/resources
build and 27 workspace framework tests pass. Binary SHA256:
`1c39f6d8fd8c984353a0b79039f18bded3b1e46981c35f5386802c623fa12832`. Installed only under `/usr/local/lib/mipad2-snapshot-recording-r8`; r6 default launcher,
r6/r7 device binaries, normal settings and packaged archives remain unchanged.
This documentation commit is not the build source. No global VA override,
package upgrade, kernel install or reboot is implied by candidate installation.

Full1920x1440 synthetic SMPTE was H264 encoded once, then losslessly remuxed
with rotate-0/90/180/270 MP4 tags. FFmpeg's normal autorotate output agrees
byte-for-byte with the expected pixel permutation of its non-autorotate RGB
decode for all four directions (MAE0/outliers0). Coded dimensions stay1920x1440;
90/270-degree display dimensions are1440x1920. This validates mux/FFmpeg
orientation semantics, not user GUI/gallery compatibility or live throughput.
The original pixel fixture needed missing numpy and failed at import before
creating its folder or touching a camera. Its log is retained; v2 uses only
stdlib byte operations and does not add/upgrade device packages.

The r8 private window is launched once with matrix opt-in and hardware H264
enabled in private keyfile settings only, and shutter sounds disabled there.
A separate bounded observer waits for the first finalized new recording,
collecting actual process CPU and file packet timestamps until it finalizes,
the app exits or6h expire. This avoids reusing the old180s idle sample window
which expired hours before the user's r7 recording. A live observer is NOT a
recording pass. User preview/recording responsiveness, actual NV12 caps/matrix
propagation, audio-bearing recording, playback direction and photo regressions
remain pending. No claim that the user's 17.8fps clip is repaired yet.
