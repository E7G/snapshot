# Startup cadence localized to driver-delivered timestamps; still RED

No new kernel or application/default promotion. This diagnostic follows the
separate [identity-copy fix](MIPAD2_CAMERA_IDENTITY_COPY_20261019.md), not a claim
that all recording stutter or Android/Windows parity is solved.

## Existing clip and two real diagnostic cohorts

The previous normal GCC-release rear clip has one74ms MP4 PTS gap, between its
fourth/fifth packets (0.100 ->0.174s). No warm-up frames are excluded from gates.
New full-quality30s front/rear recordings use the unchanged R17b executable,
R19 kernel,1920x1080/30 native,1280x720/30 GPU-rotated preview,H26415Mbps and
AAC192k/48k/stereo. Real codecs/caps/bitrate, packets, full video/audio decoding,
control restoration and actual private-plugin mappings are checked.

The bounded in-memory tracer records PTS/offset, thread/pad metadata, events and
paired push/chain timestamps. It never maps/retains a camera buffer and performs
no per-frame filesystem writes. Native source and encoder-input anomalies match
exactly by native offset and a unique bounded arrival window. The first cohort
has69.216413/77.564988ms native/encoder gaps; full-clip front gate remains RED.

A separately compiled opt-in V4L2 diagnostic reads driver stamp, monotonic time,
pipeline clock/base-time and output PTS without changing any of them. In the
second cohort the three anomalies are:

| Side | Driver delta ns | Output PTS delta ns | Clock-minus-monotonic delta ns |
| --- | ---: | ---: | ---: |
| Front | 52980000 | 52980425 | 413 |
| Front | 59925000 | 59924263 | -500 |
| Rear | 60688000 | 60688937 | 612 |

Base-time deltas are zero; the expected clock-conversion equation residual is
12/-237/325ns. Thus these observations originate in driver-delivered timestamps,
not downstream encoder retiming or a pipeline-clock/base-time discontinuity.
Front29.941650fps/max60ms and rear29.980711fps/max60.667ms remain cadence RED.
Both streams decode completely,899/918 frames respectively, no consecutive exact
decoded repeats. Sequential diagnostics add overhead, not a throughput A/B fix.
Offsets advance by1 but do NOT establish physical sensor/exposure continuity.

## Kernel/Android contract and what remains unknown

Deployed R19 source8c57f2be2c2df6ffb811742478b0a0761bb80563 assigns
`ktime_get_ns()` in `atomisp_buf_done()`: completion time, not a proved sensor SOF
timestamp. The V4L2 plugin transfers the dequeued driver's timeval into buffer
metadata, then converts it to the pipeline clock domain. We checked the pinned
[Android AtomISP reference](https://github.com/latte-dev/android_kernel_xiaomi_latte/blob/d2afb1de9a761ff01dafdf9efe4736e521ab71f6/drivers/external_drivers/camera/drivers/media/pci/atomisp2/atomisp_driver/atomisp_cmd.c)
and its SOF/sequence bookkeeping rather than substituting newer IPU hardware.
Android HAL timing, SOF/exposure-ID association and optical continuity still need
proof. Neither timestamp smoothing nor blindly replacing completion time with the
latest SOF is a validated fix; delayed IRQ/ISP completion versus actual capture
gaps remains unresolved. Read-only tracefs inventory confirms per-function tracing
is available for atomisp_isr/atomisp_isr_thread/atomisp_buf_done; nothing enabled.

## Preserve negative/lifecycle evidence

The first tracing parent deliberately exits1:34 pre-calls remain pending when
the application process exits, mostly thumbnail/playbin processing. Metadata
commits are complete (266788 records,overflow0,incomplete0), but **all calls are
not balanced**. Recording encoder pending calls are0. No duration is invented for
censored calls. Cleanup-only inspection never repeats/relabels those recordings.
The next cohort also retains34 pending calls while recording ring integrity and
all-calls-balanced as distinct fields (268164 committed records,no overflow).
Application thumbnail preroll is not an operator-initiated playback test.

After the first cohort10s RTC reachesS0I3+9.074976s. After the second cohort RTC
wakes functionally with IRQ9 but **S0I3 delta0**. Preserve both; no serial-state,
repeat-test or awake-device snapshot explanation is established. New filtered
fatal kernel errors0, same boot, sensor/ISP PM0/suspended and controls restored.
Default launcher/system plugins/GRUB/GPU limits/anti-idle remain unchanged.
No media, core dumps or individual camera pixel digests leave the tablet.

Next: correlate kernel completion/IRQ scheduling with source gaps and trustworthy
sensor/exposure events; retain full-cadence and deep-residency gates. SP failed-
reset DMA ownership, Mesa shutdown crash, AF/WB, physical AV sync, visible UX and
full kernel/flash Android/Windows parity remain open. No DMA zero-copy promotion.
