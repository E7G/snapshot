# R16b measured full-content diagnostics — 2026-10-15

R16a's full-frame MD5 probes reduced actual UI recording to23–24fps. R16b
replaces only this opt-in fingerprint with pinned xxhash-rust0.8.19 XXH3-128,
using baseline x86_64 SSE2, not target-cpu=native/AVX. All active NV12 Y/UV bytes
and geometry still contribute; allocator padding does not. No row subsampling,
retained camera buffers, exported images or individual digests. This is
probabilistic diagnostic content identity, not cryptographic authentication.

## Host and actual hardware gates

* All57 framework tests and full release/resources/private packaging pass,
  preserving the prior56 tests. The new serialization/reference test additionally
  toggles every active byte in both planes. Cargo.lock adds only the pinned
  xxhash-rust package and the aperture dependency; no unrelated package upgrades.
* Same R19 boot, same UI pipeline, private R16b prefix: tracing-off front/rear
  30s controls passed at29.998/30.017fps. Two tracing-on front/rear pairs averaged
  about30fps, versus MD5's23–24fps. Encoder probe mean is about2.4ms/frame instead
  of13ms, **not zero overhead**. See JSON for actual counters/costs.
* First tracing-on rear retains a **76.333ms startup PTS gap**, index4 at
  PTS0.176333s. Its strict <50ms cadence gate is RED. After the first2s, maximum
  gap is34.333ms; the independent repeat's whole clips max34.333ms. Repeat
  success does not erase the startup failure or establish a timing-neutral probe.
* Instrumented encoder inputs916/918 and918/923 all uniquely match native
  content, with zero repeated/reused/missing/ambiguous/backward/stale-owner
  hashes. Rear decoded identical frames remain20/24, versus23 in the control.
  Distinct noisy inputs can quantize identically in lossy H264: decoded equality
  alone is not proof of missing physical motion. No synthetic CFR timestamps or
  frame duplication were added to obtain these rates.
* All six clips video/audio decode, all three app runs quit cleanly, original
  cached controls and PM0/suspended state restore. The earlier Mesa/GstGL stop
  SIGSEGV remains unresolved; this round's successful stops are not a fix.
* Post-camera10s RTC suspend resumes in the same boot, S0I3+9,312,608us.
  USB/Wi-Fi restore, no new kernel errors; bounded serial capture ends cleanly.

No playback/tones or private media/core transfer. r6 launcher and prior binaries
are unchanged; diagnostics remain off by default. No new kernel deployment or
Android-quality/motion parity claim. Harness precondition/observer failures,
including the repeat's unexecuted cooldown wrapper, remain documented in JSON;
the repeat's sampled thermal maxima were69/73C, not a claimed cooldown-gate pass.

## Quality scope is still incomplete

Actual encoder DOT shows vah264enc bitrate2048kbps; MP4 audio is MP3. A newly
pinned vendor media_profiles.xml declares15Mbps1080p H264 and192kbps48kHz stereo
AAC, but its header explicitly says dummy/MRD7/T100. Treat this as declared
reference metadata, **not proof of an active MIUI runtime profile**. Do not
redefine parity as geometry/fps or merely matching an unverified declaration.
Next work must cover actual bitrate/codec quality and controlled-motion behavior,
as well as the retained GL-stop and startup-cadence failures.

## Primary references

* [xxhash-rust upstream](https://github.com/DoumanAsh/xxhash-rust): SSE2 baseline
  acceleration on x86_64 and streaming/one-shot API.
* [Pinned vendor media profiles](https://github.com/latte-dev/android_vendor_xiaomi_latte/blob/aaf390134608e033e8c5b545568e68dab1ed2bdf/proprietary/etc/media_profiles.xml).
* [Prior R16a failures](MIPAD2_R16A_HARDWARE_20261015.md).
