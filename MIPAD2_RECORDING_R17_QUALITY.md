# R17a actual bitrate/codec quality candidate — 2026-10-15

Previous "Android profile" aligned recording/preview geometry, not full quality.
Measured legacy video was about2Mbps H264 with about51kbps44.1kHz mono MP3.
Vendor media_profiles.xml declares15Mbps/AAC192k48k2ch, but has a dummy MRD7/T100
header: this is a declared reference, not an active MIUI runtime benchmark.

## Implementation and genuine rank failure

MIPAD2_RECORDING_QUALITY=1 is capability-gated to exact MiPad2/H264/hardware/
Android-geometry opt-in and AAC availability (or disabled audio). It requests
VA H26415000kbps and explicit AAC-LC192000bps/48000Hz/2ch, retaining VFR and native
1080p, not faking timestamps or lowering resolution. Ineligible requests log an
explicit warning and retain legacy settings; they are not quality-validated.

AAC was absent on the device. A pinned official Arch gst-libav1.28.7 package
and detached signature were verified with the existing Arch keyring. Only its
plugin was extracted into a private directory; existing FFmpeg9 dependencies
resolve. No pacman installation, system update, or global plugin-path change.
Both control and quality arms use the same private plugin path and binary.

Initial R17 passed60 framework tests, but the actual isolated encodebin request
failed: avenc_aac rank0 is excluded from its candidate list even with factory
name pinned. This is a genuine runtime integration RED, not relabeled a green
host result. R17 was never packaged/installed. R17a temporarily raises only this
process's AAC factory rank and its source restores the saved rank when quality
is disabled. The policy test covers restoration; an isolated on-device silence
encode validates raise→AAC EOS/finalize/decode→restore. Actual UI-toggle rank
restoration has not been separately exercised. All61 framework tests/full build/
private package pass. The same-binary flag0 control still records MP3.

## Actual UI recording comparison,30s each

| Arm | Sensor | Measured video bps | Audio | fps | Max PTS gap |
|---|---|---:|---|---:|---:|
| Legacy control | Front |1,984,206|MP3,44.1kHz,1ch|30.0023|34.333ms|
| Legacy control | Rear |1,985,155|MP3,44.1kHz,1ch|30.0123|34.000ms|
| Quality | Front |14,956,104|AAC,48kHz,2ch,192059bps|30.0023|34.000ms|
| Quality | Rear |14,600,146|AAC,48kHz,2ch,192824bps|**28.75**|**50.334ms**|

Actual DOT confirms VA bitrate15000 and AAC bitrate192000, not merely requested
environment flags. All four clips video/audio decode and both apps quit cleanly.
Rear decoded adjacent identical frames change26→0, but codec/bitrate/cadence all
changed together: no unique causal conclusion or motion-loss fix is claimed.

**Joint high-quality rear throughput is RED.** Keep the target quality; do not
substitute the easier low-bitrate control as success. Next isolate video bitrate
and AAC/mux/preview cost in a same-binary factorial and measure queue/GPU/CPU
pressure. Four successful stop events do not resolve the earlier Mesa/GstGL crash.
The earlier76.333ms R16b startup observation also remains unresolved.

Encoded stereo alone does not establish native stereo microphone capture.
Scalar audio levels are near a quiet noise floor; no subjective microphone or
speaker listening validation occurred. No playback/tones or private media/core
export. S0I3+9,762,240us after camera use, same-boot resume, USB/Wi-Fi restored,
no new kernel errors. Original controls, PM0/suspended, normal settings and r6
launcher hashes restore. Stable GRUB default unchanged. No new kernel deployed.
This remains a private candidate, not an Android-quality/parity completion claim.

## Primary references

* [Pinned vendor declarations](https://github.com/latte-dev/android_vendor_xiaomi_latte/blob/aaf390134608e033e8c5b545568e68dab1ed2bdf/proprietary/etc/media_profiles.xml)
* [AAC factory/API](https://gstreamer.freedesktop.org/documentation/libav/avenc_aac.html)
* [GStreamer1.28.7 profile factory selection source](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst-libs/gst/pbutils/encoding-profile.c)
* [GStreamer1.28.7 encodebin source](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst/encoding/gstencodebasebin.c)
