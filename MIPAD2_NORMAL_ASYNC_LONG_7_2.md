# 7.2 normal copied-async capture: front 60-second sample, rear incomplete

This is a follow-up to [mixed normal application cadence](MIPAD2_NORMAL_APPLICATION_7_2.md)
and the [VA rational-rate fix](MIPAD2_VA_FRAME_RATE_7_2.md). Runtime remains the
requested upstream 9928 / **7.2.0-mipad2-integrated-r3**, kernel build source
20f0304f. No new kernel C patch/build, reboot, package installation or default
launcher promotion for these tests. Source identities and scalar receipts are in
[the machine-readable summary](tests/mipad2-normal-async-long-7.2-summary.json).

## Actual normal foreground path

Exact published fe70 candidate launcher, explicitly private `raw_async=true`,
0a7ce87a app binary, source AAC and fixed full-recipe VA plugin. Native direct,
copy and allocation guards retained; bounded non-leaky four-buffer copied-memory
tail enabled. Existing encodebin encoded queue unchanged. No diagnostic GActions,
background capture, kernel ftrace, in-process tracer or LD overrides. Actual fixed
VA and i965 mapped, but encoder factory was not independently observed.

The normal `t` accelerator starts/stops recording only with PID-matched active
window, unlocked native session and camera ownership verified. Rear selected
using the real visible GTK Switch Camera button's normal AT-SPI Action, not a
diagnostic action or external live S_INPUT (which would desynchronize app state).
Temporary authorized keyboard destroyed after recording, normal app Quit exit0.

## Whole finalized files, no frames excluded

Both samples: 1920x1080 H264, AAC48kHz stereo, no rate/quality reduction, timestamp
smoothing/rewrite or packet discard. Video and audio fully decoded without error;
decoded count matches packet count; adjacent exact decoded repeats0.

| Run | Requested | Actual PTS span | Frames | PTS rate | Max interval | Nonpositive / >50ms |
|---|---:|---:|---:|---:|---:|---:|
| Front, completed | 60s | 64.7633s | 1944 | 30.00154fps | 38ms | 0 / 0 |
| Rear, **interrupted** | 60s | 14.6603s | 441 | 30.01296fps | 34.333ms | 0 / 0 |

Front video15.105Mbps/AAC192.259kbps; rear14.895Mbps/AAC191.782kbps.
Front maximum **observed** temperature79C, CPU ceilings1.33/1.60GHz. Rear retained
samples75/78/75C, same observed ceilings, then an unlabelled AssertionError ended
recording. **The triggering sample/condition was not retained** by that controller;
cannot conclusively label it an80C trip, ownership loss or session-query failure.
The app still quit gracefully, controls restored, truncated-duration MP4 retained.
No kernel fault deltas. Rear short stored-file cadence passes, but **60-second
completion fails**. A cadence-only flag must not turn a short sample into acceptance.

Follow-up controller labels guard failures and retains the triggering health
sample/private traceback before stopping. It never launched a camera: passive
cooling did not reach <=50C within120s. No temperature/free-space thresholds
relaxed and no repeat recording. This is not additional rear hardware evidence.

## Scope and state

This front whole-file pass adds evidence beyond the previous short traced B,
but does not erase the earlier normal556.667ms/duplicate-PTS failure, prove smooth
live delivery/preview, physical sensor frame conservation, optical AE/AWB/focus,
thermal stability or Android/OEM parity. Rear long recording remains unqualified.
Not a randomized temperature/frequency-matched A/B/A. No stable/default promotion.

Fresh postflight: same7.2 boot,686 protected installed files byte-identical,
kernel fault counts0, guards active, no failed units or camera/GStreamer clients,
both sensors/ISP runtime-suspended usage0. Original sensor crop/format/controls
(including vertical blanking) and global camera settings restored. Crop FIRST,
then format/binning, then controls, with readback. Existing rescue default kept.
Automatic lock/DPMS/suspend remain OFF per explicit local user preference;
manual locking and login unchanged. No audible playback.

Four precisely audited obsolete self-generated MP4s retired under authorization,
350331965bytes; all39 associated metadata files retained. Latest normal failure,
generic-VB2 A/B samples, photos, logs, kernels and new recordings retained.
Raw media, all per-frame timing/hashes and raw kernel/serial/pointer evidence stay
tablet-private. Current source defaults and package/rootfs ABI not newly qualified.

Previous3.5-hour hard freeze remains unexplained: user recovered and a guarded
7.2 reboot succeeded; surviving previous-boot journal has no attributed panic or
GPU/ISP failure, which does not exonerate a hard freeze. Failed-reset DMA fencing,
long normal rear/thermal/preview behavior, optical quality, actual7.2 acoustic/UCM,
resume/S0I3, S5 and complete-image qualification remain open.

## Reproducible bounded host gate

`tests/normal_recording_qualification_7_2.py` checks one whole-file >=60s normal
cadence sample; it deliberately does not claim product, optical or live-preview
acceptance. The interrupted rear is rejected even with a passing cadence flag.
Twelve tests also reject duplicate PTS,556ms gaps, incomplete decoding, missing or
nonfinite values, diagnostic capture, kernel faults and active runtime PM.
Both commands pass (no device access or raw data required):

```sh
python3 -m unittest discover -s tests -p test_normal_recording_qualification_7_2.py
python3 -O -m unittest discover -s tests -p test_normal_recording_qualification_7_2.py
```

These are scalar-schema/negative host oracles, not reconstruction of physical
frames or recovery of the missing rear controller stop sample. Existing flash
staging154tests also pass normally and with Python-O; runtime defaults unchanged.
