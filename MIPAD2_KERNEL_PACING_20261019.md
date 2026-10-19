# Driver-timestamp gaps: kernel IRQ evidence, not a completed fix

Follow-up to [source timestamp localization](MIPAD2_CAMERA_PACING_20261019.md).
Same R19 kernel8c57f2be2c2df6ffb811742478b0a0761bb80563 and R17b application,
same boot, native1080p30/full720p30 GPU preview/H26415Mbps/AAC192k48k2ch. No
quality/frequency reduction, new kernel, system plugin change or default promotion.

## Isolated kernel observation, with real negative controls

R19 offers function tracing in own tracefs instances. A graph prototype detects
that its instance funcgraph-abstime option affects shared graph settings, restores
the prior value and aborts before camera work. Its final global snapshot matches.
The actual capture uses **function entries**, no shared trace_options writes,
no dynamic probes, exact function filters, isolated buffers and mono clock.
An own-PID synthetic syscall/sleep test checks hook presence and monotonic markers;
the real global tracer/filter/options state stays unchanged after every capture.

The first capture dispatch guard assumed an `atomisp` IRQ label and fails before
launching any camera. Read-only PCI identity fixes discovery:0000:00:03.0 IRQ146,
thread528 `irq/146-isp_irq`. New namespaces, not a blind restart of pending work.
Actual captures verify a live systemd PID and bounded runtime watchdog, filter
IRQ events to146 and scheduler events to thread528, then stop/release the own
instance in cleanup. Ring per-CPU overrun/commit-overrun/dropped-events are0.
No global unrestricted graph tracing or kernel argument/memory reads are used.

## Actual recordings, including non-reproduction and thermal abort

First2x30s run: front29.999336fps/max34.333ms, rear30.011952fps/max42.667ms,
905 decoded frames each. No >50ms recording source gap reproduced. This is not
proof that the older intermittent bug is fixed. Its subsequent10s RTC reaches
S0I3+9.935744s, IRQ9; earlier zero-residency results remain valid failures.

Next plan is six30s repeated camera switches. It actually completes ONLY TWO:
front29.965243fps/max49.667ms (889 decoded frames), then rear29.930097fps/
max78.334ms (913 decoded frames, RED). At peak80C the existing thermal guard
aborts further recordings. Both actual clips/video/audio decode; app gracefully
quits, controls restore, sensor/ISP PM0/suspended. Parent remains failed/needs-review;
four unexecuted clips are not a pass and no post-abort RTC test was performed.
An earlier-capture parser analysis ran on the tablet during part of this cohort;
additional CPU/memory observer load may affect timing and heat. It is not an
unprofiled performance comparison or a unique proof of a thermal root cause.
That parser was subsequently optimized, retested while the camera was stopped,
and reproduces exactly the original anomaly/baseline results with0.264s baseline
analysis CPU time. No recording is repeated to relabel the thermal failure.

## Positive failure localization

Two rear source PTS gaps persist unchanged through encoder input. Driver stamp
steps are78.161ms and51.039ms. Each before/after driver timeval stamp uniquely
falls in a kernel `atomisp_buf_done` -> `vb2_buffer_done` function-entry bracket.
Printed kernel time and timeval truncate to microseconds; ambiguous ownership
matches are explicitly rejected, not assigned to the nearest convenient frame.

| Driver gap | IRQ-thread sleep intervals | Wake-to-run delays | Latest handler-entry to driver stamp |
| --- | --- | --- | ---: |
| 78.161ms | S70.821ms + S6.548ms | 82us,416us | 339us |
| 51.039ms | S44.178ms + S6.647ms | 24us,67us | 80us |

Thus the observed long windows mostly wait for later ISP event availability, not
a long mutex-blocked interval inside the active threaded handler. These samples
do not exclude all mutex/scheduling issues elsewhere. Typical entry-to-video-
completion spans in this cohort:p5017us,p9968us. Function tracing records entries,
not return durations; no return/CPU duration is invented. `atomisp_buf_done` also
handles statistics/nonvideo types, so entries without VB2 completion are not
automatically video drops or unbalanced lifecycle calls. IRQ exit's printable
`handled` value alone does not distinguish the WAKE_THREAD bit; actual scheduler
wake events are measured separately. IRQ delivery is not sensor SOF proof.

In the two gaps QBUF wrapper entries appear54.720ms/28.378ms after the previous
driver stamp; the next completion follows23.441ms/22.661ms later. This makes
buffer availability/backpressure a concrete next hypothesis, **not a proven
empty CSS queue or a reason to fabricate evenly spaced timestamps**. Existing
function-entry traces lack buffer indices, successful-QBUF results and live
owned-by-driver counts needed to establish queue starvation.

## Next discriminating measurement; broader work remains

Read-only target tracepoint format inventory confirms R19 V4L2 indexed QBUF/DQBUF
and VB2 queue/done metadata, including timestamp,queued_count,owned_by_drv_count,
index and type. These were NOT enabled in this cohort. Use an own filtered trace
when the tablet has cooled to distinguish buffer-return starvation from ISP/
sensor processing gaps. Preserve invalid/unknown/ownership and full-clip cadence
gates. CPU720p scaling/VA importer copies remain measured costs; any offload must
retain preview/native quality and proper DMA ownership.

Protected default apps/plugins/GRUB/GPU limits/anti-idle/USB remain unchanged.
Current controls restore FRONT OV5693 exposure1976/gain127, REAR T4KA3 exposure2486/
gain128. All jobs, serial readers, capture units and own instances are terminal.
Raw media/cores/pixel digests/kernel traces stay private; only source and scalar
evidence are published. SP failed-reset DMA fencing, Mesa stop crash, AF/WB,
physical AV sync, foreground UX, intermittent S0I3 and full kernel/flash Android/
Windows parity remain unfinished. No completion or default promotion is claimed.
