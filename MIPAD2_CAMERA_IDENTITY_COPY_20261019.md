# Default identity-frame copies removed; cadence remains RED

This is a measured companion GStreamer fix on the unchanged Mi Pad2 R19 kernel,
not a claim of a new kernel, default promotion or Android/Windows parity.

## Cause and real negative control

A bounded per-process CPU-clock sampler records user instruction-pointer and
frame-pointer callchain metadata; it does not use ptrace/signals or map target
memory/camera buffers. Its real target busy-function self-test resolves96 of97
samples to the expected function, with no lost/parse errors. Camera recordings
identify CPU720p scaling, VA import copies, and two otherwise unnecessary full
frame copies through videoflip in viewfinder and recording branches.

Exact upstream1.28.7 initializes proposed_method to IDENTITY. Its constructed
callback invokes set_method(default), but that setter only updates BaseTransform
passthrough when the effective method differs from proposed_method. Thus a new
identity instance does not enter passthrough, even if explicitly set to none.
Actual default NV12 input/output buffers and pixel memories differ. A separate
public-API passthrough control, or toggling away and back, shares the memory.

The small patch initializes proposed_method to an AUTO sentinel, forcing the
existing constructed/default setter to perform its normal IDENTITY setup. Later
nonidentity/automatic transitions retain the existing setter/negotiation path.
No frame is dropped, failed flow forced to OK, quality reduced, or DMA lifetime
fence bypassed. Upstream main observed during this investigation has the same
initialization; source hashes and observation time are in the JSON.

## Actual source, builds and geometry/content checks

The official gst-plugins-good1.28.7 archive matches its published SHA256, and
videoflip source matches the pinned GitHub tag. Real git apply --check/application
and exact patched-byte verification pass. A first Windows apply check encountered
autocrlf conversion; the retained retry disables autocrlf and checks exact bytes.

Both complete four-factory plugins (videoflip, videobalance, gamma, videomedian)
are built with normal GCC/Meson release settings, same source/config except the
patch. Host core libraries are1.28.4. Target1.28.7 dependency relocation checks,
factory filenames and process module maps are verified; stock plugin unchanged.

The standalone ctypes regression processes real synthetic buffers through the
actual plugin, not a decision model. It checks every active RGB and NV12 component
under all8 orientations, output geometry, and shared pixel memory for identity.
Original exit1 is the expected RED (identity copies); patched exit0 preserves all
16 mappings and shares identity memory. Both GCC host and actual tablet binaries
pass this original/patched contrast. Public-API control is separate and is never
reported as validation of a patched binary. It does not open a camera or display.
Broader in-stream tag/seek/concurrency coverage is not claimed by this test.

## Real cameras, full quality, retained failures

All clips request native1920x1080/30, full1280x720/30 GPU-rotated preview,
H26415Mbps and AAC192kbps48kHzstereo; same R17b app and VA-depth2/EOS companions.
No bitrate/resolution/rate downgrade, preview removal or frequency override.

The first paired camera build uses signed, privately extracted Arch TCC because
WSL was temporarily unavailable. Both arms use the same compiler/config/packaging
and full four-factory source. TCC omits GNU_STACK; signed private patchelf adds an
explicit non-executable stack declaration to BOTH arms. Failed direct-build and
packaging attempts are retained locally, not treated as working candidates.
Regular GCC release builds above do not require this workaround.

| Same-toolchain24s CPU sampling | Front | Rear |
|---|---:|---:|
| Original videofilter frame-copy samples |198|206|
| Fixed videofilter frame-copy samples |0|0|

Total user CPU samples change1196/1261 to974/974, but sample counts are NOT exact
CPU-time/energy savings. Remaining CPU scaling and VA importer copies are still
observed. Frame-pointer chains can be incomplete and profiling perturbs timing.
The profiled fixed clips average29.940919/29.950739fps but have63.333/84ms max gaps;
both strict cadence gates stay RED. This is a copy-path fix, not a unique-cause
or smooth-recording claim.

After WSL recovery, the normal GCC release fixed candidate is tested without
CPU/pixel profiling for two full30s recordings:

| GCC fixed, unprofiled | Packets/decoded | Average fps | Maximum gap | Strict gate |
|---|---:|---:|---:|---|
| Front |899/899|30.001003|41.666ms|PASS|
| Rear |896/896|29.977558|**74ms**|**RED**|

All video/audio streams decode; no drain warnings or consecutive exact decoded
repeats. No startup-window exclusion is used. Audio container ends are about
0.314/0.288s earlier than video; physical AV sync is unproven. The following10s RTC
wakes via IRQ9 with **9,049,888us S0I3** and no new filtered fatal kernel errors.
This does not erase earlier zero-residency or Mesa stop-crash cases.

The first GCC camera attempt stopped at its2GiB storage guard BEFORE app launch.
After explicit human approval, only old pre-Oct19 diagnostic videos were pruned;
logs/photos/current samples/installed kernels were retained. No private media,
core or individual pixel digests were exported. Recordings were not blindly
restarted after timeouts. All dispatched hardware jobs and serial captures ended.

Defaults/previous binaries/system plugins, normal settings, current sensor
controls/PM0, GPU200..600MHz limits, stable GRUB/empty next_entry and anti-idle
remain unchanged. Current rear gain127 was observed before these experiments;
it is not relabeled as the earlier8 baseline.

**Narrow identity-copy fix verified. Rear cadence, full lifecycle/foreground UX,
physical AV sync, earlier intermittent S0I3/Mesa failures, kernel SP/CSS failed-
reset DMA ownership, AF/WB and flash/default integration remain unfinished.**

Next address measured CPU720p scaling and VA import costs without reducing
quality; DMA zero-copy work must first establish ownership and physical fencing.

Primary references:
* [Pinned videoflip source](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-good/gst/videofilter/gstvideoflip.c)
* [Official good source/checksum directory](https://gstreamer.freedesktop.org/src/gst-plugins-good/)
* [Kernel perf ABI](https://man7.org/linux/man-pages/man2/perf_event_open.2.html)
