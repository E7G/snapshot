# Recording EOS reuse fixed privately; resume gates remain RED

## Causal evidence, not just a30fps average

A read-only private GstTracer records scalar pad/event/flow data without mapping
or retaining buffers. Host and tablet normal/early-EOS tests verify localization.
On real camerabin's second recording, the inactive passthrough branch sends EOS
to streamcombiner0, which immediately forwards EOS to h264parse **before** EOS
reaches the active encoder. The active encoder's delayed output then gets an EOS
return; its two final inputs are absent from the file. AAC shows the same early
combiner-EOS path. The causal window is retained before the tracer's2000-log cap;
the cap is reached later, so this is not a continuous/full-clip trace claim.
Push/send event hooks share a label in this diagnostic version; duplicate event
observations are not counted as distinct EOS events.

Exact upstream1.28.7 streamcombiner initializes each pad's is_eos only once and
never clears it when camerabin reuses encodebin through READY. The patch clears
all pad EOS flags and draining_encoder under the existing lock, after a successful
parent PAUSED_TO_READY transition. It does not force a failed flow to OK, ignore
EOS, alter codec quality, discard buffers, or touch DMA ownership.

## Real source regression and full plugin build

The test compiles the real original/patched upstream combiner, not a decision
model. Original first epoch succeeds; second epoch forwards inactive EOS early,
rejects the delayed active buffer with EOS, and loses that buffer. Patched second
epoch waits for active output and both EOS signals. A64-epoch repeat gives63
premature-EOS failures on original source and64 complete epochs on patched source.
Both runs have empty stderr. Host core libraries are1.28.4; full target tests below
use the tablet's1.28.7 environment.

The official gst-plugins-base1.28.7 archive (2,558,440bytes) matches its published
SHA256 and pinned GitHub source. Full original/fixed encoding plugins are built
with the same release configuration; only the combiner patch differs. Target
ldd-r/factory paths and actual process module maps are verified. Original signed
system encoding plugin is not replaced. See JSON for exact source/patch/binary
hashes and archive checksum. The VA depth2 plugin and R17b app binary are unchanged.

## Matched two-camera tests

All request native1920x1080/30, full1280x720/30 GPU-rotated preview,
H26415Mbps/AAC192kbps48kHzstereo. No lower bitrate, resolution, preview removal,
frequency override or frame-rate downgrade. Every recorded video/audio stream
decodes. Frame identity checks are full active NV12 XXH3-128 comparisons; private
media/core and individual pixel digests remain on the tablet.

| Same-toolchain encoding | Front input/packets | Rear input/packets | Rear drain warnings |
|---|---:|---:|---:|
| Original,10s full-pixel probes |312/312|**309/307**|**1**|
| Fixed,10s full-pixel probes |309/309|316/316|0|

All observed fixed encoder inputs uniquely match native frames with no ambiguity,
reuse, backwards/skip, missing, PTS mismatch or stale-owner matches. This closes
the measured input/output conservation failure for these recordings; it is not
an optical Android-motion or codec-ownership proof for arbitrary workloads.

Fixed full30s recordings without pixel probes: front29.997677fps (905 packets),
rear30.019112fps (912 packets); max gaps34.334/34.000ms. Both whole-clip cadence
gates pass with zero drain warnings. The10s instrumented fixed cases also measure
about30fps in this round, but earlier probe-induced slowdowns remain preserved.

## Resume is still NOT complete

The first post-camera10s RTC wake enters S0I3 for9,185,152us. After reopening
the same fixed candidate, another two30s recordings still have no drain warnings
and decode correctly, but front drops to **28.799364fps**, max gap **50.667ms**;
rear remains30.016694fps. The original strict cadence gate stays failed: no
startup/window exclusion, thermal excuse or quality reduction is substituted.
Recorded maximum front thermal value is73C, not evidence of a proved cause.
Container audio/video end differences are in JSON; physical AV sync is unproven.

Final10s RTC wake returns functionally without new kernel errors, but S0I3 delta
is **0**, another retained failure. A read-only check confirms ISP and both sensors
runtime-suspended/usage0. It does not identify the actual SoC low-power blocker.
Earlier intermittent zero-residency and Mesa stop-crash cases remain open.

All test processes and serial capture ended. Protected previous binaries, r6
default launcher, original VA/system encoding plugins, normal settings, sensor
controls/PM0 and GPU200/600MHz limits match. Stable GRUB default/empty next_entry
and anti-idle remain. No tones/playback, system package update or new kernel.

**Specific EOS fix passes; full objective remains active. No default promotion.**
Next investigate front post-resume video/preview throughput and the intermittent
S0I3 blocker under controlled serial/USB conditions, preserving these failures.
Kernel SP/CSS/failed-reset DMA fencing, full flash/default integration, foreground
UX and Android/Windows parity are still unfinished.

Primary references:
* [Upstream1.28.7 combiner](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst/encoding/gststreamcombiner.c)
* [Splitter EOS order](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst/encoding/gststreamsplitter.c)
* [Camerabin READY reuse](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/gst/camerabin2/gstcamerabin2.c)
* [Official source/checksum directory](https://gstreamer.freedesktop.org/src/gst-plugins-base/)
