# MiPad2 7.2: bounded VA frame-rate parameter and remaining recording gap

## Actual defect, not a sensor-clock workaround

GStreamer 1.28.7 `gst_va_base_enc_add_frame_rate_parameter()` masks each
negotiated fraction component to 16 bits. Front OV5693 native 1920x1080 NV12
reports 78125/2604 (about 30.002 fps), but masking submits 12589/2604
(about 4.834 fps) to VA bitrate control. These are different rates.

Primary sources: [pinned GStreamer implementation](https://raw.githubusercontent.com/GStreamer/gstreamer/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvabaseenc.c),
[libva frame-rate contract](https://github.com/intel/libva/blob/master/va/va.h).

`contrib/mipad2/va-frame-rate/0001-pack-bounded-rational-frame-rate.patch`
computes the nearest positive representable numerator/denominator using integer
continued fractions and a final semiconvergent comparison. The native fraction
becomes 62494/2083, not a bitwise truncation. Zero, negative/wrapped, and
unrepresentable rates fail before VA submission; VA failures propagate.
Caps, PTS, sensor programming, requested bitrate and image quality are unchanged.
No frame dropping, duplication, timestamp smoothing or power-policy adjustment.

## Reproduction / host oracle

Use the exact official gst-plugins-bad 1.28.7 `sys/va/gstvabaseenc.c`:

```sh
python3 contrib/mipad2/va-frame-rate/frame-rate-host-test.py \
  --upstream-file /absolute/path/to/gst-plugins-bad-1.28.7/sys/va/gstvabaseenc.c
python3 -m unittest discover -s contrib/mipad2/portable-plugins -v
python3 -O -m unittest discover -s contrib/mipad2/portable-plugins -v
```

The oracle verifies pinned input SHA-256, applies the actual patch, compiles the
actual parameter dispatcher and helper under ASan/UBSan, and tests 384 production
cases with VA success/failure plus 3969 exhaustive small-bound cases against
independent every-denominator nearest-fraction enumeration. Equal-distance ties
are accepted. The old implementation compiles and produces the wrong native
fraction. Only the final VA endpoint is stubbed: this is not a hardware model.
All 21 plugin builder tests and the combined 41 plugin/AAC/app builder tests pass
normally and under `-O`; no claim that the
assert-based standalone source oracle runs under `-O`.

The portable three-plugin recipe now pins and applies both VA patches, records
both resulting source hashes, and fails closed on invalid extra patches. A real
three-plugin build against a matching private 1.28.7 SDK completed; all exports
have build RPATH removed. The AAC builder helper hash is updated to the new pinned
recipe; full discovery caught the stale pin and passes after correcting it.
Historical AAC binary evidence is not relabeled as a new AAC rebuild.
This does not install a system package or new default.

## Actual hardware evidence on 7.2.0-mipad2-integrated-r3

Same Meson/SDK/build tree, only `gstvabaseenc.c` changed; baseline rebuilt from
the pinned original and fixed rebuilt from the patched file. Both use real
V4L2 -> `vah264enc` -> MP4, 1080p, requested 15 Mbps, no preview/audio.
Exact tested plugin and i965 driver independently observed mapped in process.

| Sample | Frames decoded | Actual bitrate | PTS rate | Maximum interval |
|---|---:|---:|---:|---:|
| Same-build original | 300 | 84.90 Mbps | 27.332 fps | 863.667 ms |
| Same-build fixed | 300 | 15.95 Mbps | 30.002 fps | 34.000 ms |
| Full-recipe fixed long sample | 900 | 15.23 Mbps | 29.737 fps | **301.333 ms** |

All decoded frame counts match packet counts, with no adjacent exact decoded
repeats. This does not prove conservation of physical sensor frames.

**The 900-frame whole-clip cadence gate FAILED.** Its only >50/100 ms interval
occurs at about 20.165 seconds (packet index 606), not startup. No startup frames
are excluded. Attribution to source delivery, downstream blocking, VA or system
scheduling remains unproven. Restored bitrate is not proof that all stalls are
fixed. Two short fixed trials cannot override this longer negative result.

The earlier normal foreground source Snapshot app (not background diagnostic
actions) recorded 909 frames with AAC: 29.936 fps, three >50 ms intervals,
maximum 56.667 ms, 14.89 Mbps video, 48 kHz stereo audio; decode passed.
That sample predates the new VA plugin and already had normal bitrate, so the
rational fix is NOT established as the cause or fix of its remaining cadence.
Normal app rear capture, matching full-plugin app run, optical/AE/AWB quality,
longer recording/preview/audio and post-resume/cold-boot qualification remain open.

Raw front V4L2 900-frame baseline was about 30.001 fps, max completion interval
33.889 ms, zero >50 ms intervals. Kernel timestamps are buffer completion, NOT
sensor SOF. This is descriptive isolation, not same-pipeline causal proof.

## Safety / integration boundary

New plugin artifacts live only in private diagnostic user-state directories.
Normal candidate settings and original sensor controls restored. Default camera,
installed system files, GRUB stable fallback and kernel unchanged. After tests,
both sensors and ISP were suspended with usage zero, no camera client remained,
no kernel fault marker, 686 protected files and 35 private aliases/bytes unchanged.
No raw recordings, per-frame timing, frame hashes, kernel traces or serial logs
are exported. Scalar evidence: [summary](tests/mipad2-va-frame-rate-7.2-summary.json).

Do not promote this as smooth normal-app recording, a full image, OEM/Android
parity, or completion of the broader MiPad2 kernel/system task.

## Subsequent diagnostic experiment

[Long-source metadata and bounded encoded writer queue experiment](MIPAD2_RECORDING_WRITE_ISOLATION_7_2.md). Includes negative controls and remaining live-delivery/application gates; not a default promotion.
