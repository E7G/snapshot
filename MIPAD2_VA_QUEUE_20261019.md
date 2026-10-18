# VA queue experiment: throughput improves, EOS gate remains RED

## Source and scope

R19 Linux6.14, unchanged R17b app binary. Official gst-plugins-bad1.28.7
archive is complete (8,347,976bytes), matches its published SHA256, and both
VA encoder source files match the pinned GitHub tag byte for byte. The earlier
proposal only passed a syntax/model test: its real `git apply --check` failed
because its context omitted an upstream blank line. The corrected full-context
patch now applies and the full VA plugin builds (91 Ninja steps). Earlier
failures remain in history/evidence; they are not rewritten as passes.

The private plugin opts into live H264 output delay2 only for exact environment
value `MIPAD2_VA_ASYNC_DEPTH=2`. Unset/invalid/0 retains live delay0; non-live
retains4. Actual logs show live delay0/min-buffers1/latency0 versus live
delay2/min-buffers3/latency66,666,666ns. Existing upstream pool, blocking-output
and drain logic is reused, not bypassed. No bitrate, profile, geometry, preview
or requested frame-rate downgrade. No system plugin/library/package replaced.

Private dependency resolution and actual factory filename/hash were checked.
Four120-frame synthetic cases (live unset, live2, live invalid, non-live2)
finalize and decode all120 packets. This was necessary but not sufficient:
the real camerabin lifecycle exposes a failure the synthetic test misses.

## Actual same-binary camera recordings

All request native1920x1080/30, full1280x720/30 GPU-rotated preview, H264
15Mbps plus AAC192kbps/48kHz/stereo. All finalized video/audio streams decode.
These are sequential diagnostic runs, not randomized/optically controlled
Android comparisons. A decode pass is not a frame-conservation pass.

| Arm | Front fps | Rear fps |
|---|---:|---:|
| Prior GTK Vulkan/default |26.033|27.402|
| Prior explicit GTK GL |26.119|27.254|
| New private VA plugin, depth0 |28.411|26.576|
| Same private plugin, depth2 |30.001|30.012|
| Depth2 process reopen after functional RTC wake |29.992|29.995|
| Depth2 with full active-pixel fingerprint probes |**27.062**|29.874|

GTK renderer change did not remove the high-rate slowdown in its matched
earlier round. Depth2 improves throughput in this round, including reopen;
it is **not** sufficient evidence of a completed recording fix. Content probes
add measurable overhead, particularly on the front camera. Their slow result
is retained; it is not extrapolated into uninstrumented optical-motion proof.

## New blocking lifecycle finding

With full active NV12 XXH3-128 matching, all observed encoder inputs uniquely
match native inputs (no missing, reused, backwards, ambiguous or stale-owner
matches). Front:807 inputs,807 packets. Rear:**888 inputs,886 packets**.
The original strict analysis remains failed. Successful pixel identity checks
must not hide the two-frame input/output conservation mismatch.

All four depth2 rear trials (including a fresh10-second diagnostic) produce
`Still 1 frame in the output list after drain`; depth0 trials do not.
The shorter diagnostic reproduces input/output difference2 and identifies
`_push_out_one_buffer` returning **eos** for system frame579, then purging the
remaining queued frame. The camerabin video-branch EOS/reset message follows
that failure, rather than preceding it. The JSON preserves the ordered trace.

Next gate: locate the receiving pad/segment/EOS boundary returning this flow,
and distinguish true recording-tail loss from post-capture boundary accounting.
Do not turn `GST_FLOW_EOS` into success, suppress the warning, discard buffers
silently, lower the quality target, or promote solely on average30fps. A real
fix needs per-stream input/output/EOS conservation and repeat/reopen tests.

## Power, restoration and limits

First10-second RTC suspend wakes functionally but S0I3 delta is0: **retained
failure**. Later20-second repeat enters S0I3 for19,693,952us; subsequent10-second
tests measure9,552,992us and9,925,728us without new kernel errors. First failure's
cause is not established. Serial captures are bounded separate windows, not
continuous; the20-second repeat was observed through SSH/on-device logs.

Both sensors return to their original controls/runtime-usage0/suspended.
Protected previous app binaries, r6 default launcher and original signed VA
plugin hashes match; GPU limits remain200/600MHz. Stable saved GRUB default is
unchanged with empty next_entry. Test processes/captures ended. No tones,
playback, private media/core or individual pixel-digest transfer/publication.

The candidate remains private/opt-in and **not default-promoted**. Earlier
Mesa stop crash, initial zero-residency case, SP/CSS/failed-reset DMA ownership,
full flash/default integration and Android/Windows parity remain unfinished.

Primary sources:
* [Official1.28.7 source](https://gstreamer.freedesktop.org/src/gst-plugins-bad/gst-plugins-bad-1.28.7.tar.xz)
* [Official checksum](https://gstreamer.freedesktop.org/src/gst-plugins-bad/gst-plugins-bad-1.28.7.tar.xz.sha256sum)
* [H264 reconfigure/pool/latency](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvah264enc.c)
* [VA output/drain](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/sys/va/gstvabaseenc.c)
* [Camerabin EOS/reset](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-bad/gst/camerabin2/gstcamerabin2.c)
* [VideoEncoder base class](https://github.com/GStreamer/gstreamer/blob/1.28.7/subprojects/gst-plugins-base/gst-libs/gst/video/gstvideoencoder.c)
