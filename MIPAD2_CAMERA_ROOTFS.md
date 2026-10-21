# Source camera rootfs integration and foreground-gate evidence

2026-10-21. Linux 6.14 integrated work, **not full Android/Windows parity**.
No new kernel, reboot, RTC, photo, video, tone or default promotion in this step.

## Normal-camera evidence: do not patch the driver around an inactive window

The tablet uses Denial `0.4.4.r4.gfee449df-1`, not an active GNOME Shell. The
installed package identifies upstream `denialwm/denial`; the matching source
revision resolves to `fee449dfcbfce7b994ba1bd8930f57751494a9e5`.

Snapshot waits for `GtkWindow:is-active` before calling `Camera.start()`. Its
explicit background diagnostic branch is separate and remains disabled in the
production wrapper. The previous six normal application lifecycle cycles never
owned `/dev/video0`; they remain **lifecycle-only**, not preview/recording passes.

Independent native `denialctl` replies succeed. The single connected/enabled
panel reports `powered=false`, native1536x2048@60Hz, scale2, transform270. The
optimized shell is idle with no reported UI error/diagnostics. GNOME ScreenSaver
introspection exists, but `GetActive` returns D-Bus `NoReply`; do not convert that
failure to `active=false`. Logind's `LockedHint` is not proof of visible lock or
unlock state, and a successful unlock *request* is not authentication evidence.

A single native **timed, nonpersistent DPMS-only transaction** was tested using
the matching upstream [control-v1 protocol](https://github.com/denialwm/denial/blob/fee449dfcbfce7b994ba1bd8930f57751494a9e5/docs/protocol/control-v1.md).
Every connector and current mode/position/scale/transform was retained; only
power was requested on. The native reply reports on; the matching transaction
was rolled back and exact original output fields re-read. No pending confirmation
remains. No unlock, application launch or persistent setting write was requested.
ScreenSaver still failed while the output was on: DPMS-off is an observed gate,
**not a proven sole cause**, and visible foreground capture remains unqualified.
No compositor restart or security/foreground bypass was used as a camera fix.

## Opt-in image-build merge

Flash tooling now provides `scripts/install_camera_candidate.py` and an explicit
optional `build_rootfs.sh` hook. Default builds retain their stock camera path.

```sh
# Run from the flash-tool repository on the Linux image builder, after composing
# an independently pinned source app/four-plugin candidate package.
export CAMERA_CANDIDATE_PACKAGE=/path/to/composed-camera-package
export CAMERA_CANDIDATE_MANIFEST_SHA256=183721cbc3adda8e018a1f870fe986536784c604cd4f66867077c0c38a7c2d7a
sudo -E ./build_rootfs.sh
```

That SHA belongs to the exact previously built 75-file package, not arbitrary
future builds. Supply a new trusted external manifest pin for a different build.
Both options must be supplied together. Hash/membership/ELF/path/resource/profile
validation runs **before NBD, mounts or image operations**. This does not grant
permission to replace an unrelated existing image or flash a tablet.

The offline installer:

- Refuses live `/`, nested roots, symlink paths, extra/missing/changed payload
  files, any pre-existing destination, global library payloads, incomplete private
  resources, a desktop entry bypassing the wrapper, or diagnostic/background
  capture in the private profile. Source bytes are rechecked against the original
  pinned digest immediately before each copy, not against a later changed source.
- Requires the target pacman DB's tested ABI versions: GStreamer/core/base/bad/good
  1.28.7 and FFmpeg9.0.1 (ignoring package epoch/release only). These match the real
  tablet's queried packages. It does not downgrade packages or ignore mismatches.
- Copies only the versioned app/plugins/resources, separately named candidate
  desktop and provenance docs. No SDK libraries or stable launcher replacement.
- Executes five `ldd -r` checks inside a plain offline chroot with C locale and
  clean loader environment. This requires no bind mounts, display server, camera
  startup or target compiler. Missing libraries / versions / unresolved symbols
  fail the build before GRUB/signing/conversion.
- Retains a failed receipt and files on a post-copy failure; no implicit overwrite
  retry. Receipt and loader logs live under `/var/lib/mipad2-camera-source/`.
  A successful receipt still has `default_promotion=false` and
  `image_boot_qualified=false`.

## Actual checks, not only mocks

19 new installer/shell contract cases plus35 existing tests: **54 pass normally
and under Python `-O`**. Shell syntax and the optional-pair failure before NBD/image
side effects are tested. Actual 75-file source payload validation also passes.

Then the final installer was executed for real in a **minimal ABI-only chroot**:
215 root-owned runtime software files (248,264,680 bytes), exported read-only from
the tablet; no user config, account data, media or raw logs. The compressed software
fixture SHA is `506b2b684b0c4df069994ec4a15cca35d78a88318b2ba6500f7041dc78ffa461`.
Matching pacman name/version metadata was reproduced solely for this fixture.
The original external 75-file payload pin was used, not a newly trusted self-pin.

Dry-run left no install receipt. Actual copy + **all five native relocation checks
passed**, all payload hashes match the original export, and all215 input runtime
software hashes remain unchanged. This is not a mocked loader acceptance. Earlier
intermediate fixture/test receipts remain retained; the final r2 fixture and
`...abi-chroot-v2...` evidence correspond to the final resource/profile validator.

This minimal fixture has no bootloader/kernel/complete desktop. It is **not a
full rootfs image**, is never shipped, and proves no boot, optical preview, camera
recording, suspend or hardware parity. No new complete image has been built or
flashed. The explicit real image-build path now exists but still needs execution
and boot qualification with the current source app and full hardware stack.

The read-only tablet postflight passes292 protected software +75 candidate hashes,
actual pretest sensor controls (front1976/gain4, rear2486/gain250), both sensors and
ISP0/suspended, no remaining app/profiler/traces, and zero selected whole-boot
BUG/WARNING/Oops/GPU-hang/sensor-NACK/stream-stop-timeout counts. Original stable
boot/launcher/thermal/no-idle policy is retained. No control writes in checkpoint.

## Still required

Resolve the real foreground/lock/ScreenSaver bridge using observed desktop state;
do not enable background diagnostics as a production fix. A new DPMS-on normal
activation observation is a different hypothesis from the old six power-off
lifecycle cycles. Normal permission/preview, dual native photos/full-quality
recording, observed preview/HW factory/native-frame conservation, AE/AWB/focus,
camera-used RTC/S5, full image boot/flash and broad battery/WiFi/touch/rotation/
audio/PM/OEM comparison remain open. The full requested objective stays active.
