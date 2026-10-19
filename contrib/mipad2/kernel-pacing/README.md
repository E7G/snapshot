# Mi Pad2 R19 kernel pacing diagnostics

See [actual evidence, failed probes, thermal abort and remaining uncertainty](../../../MIPAD2_KERNEL_PACING_20261019.md).
These are date-scoped audited diagnostic sources, not a generic installer or a
kernel fix. Linux/amd64 R19 only. No camera payload access, dynamic probe, global
tracer/filter/options changes in the actual capture. A separately rejected graph
prototype's brief shared option change was restored before any camera capture.

`capture-kernel-pacing.py` is the exact source used by the bounded root transient
systemd unit. Its absolute private directory must be newly owned/prepared and its
report/stop/trace instance paths must not already exist. The dispatch wrapper
verifies the expected boot/kernel, no existing camera and a live configured unit
PID BEFORE starting a test. Do not reuse a stale namespace, overwrite reports,
copy these flags into global tracefs files, or restart a live/pending operation.

It discovers the PCI IRQ and its matching IRQ thread, restricts function/event
filters, allocates16MiB per CPU, sets only its instance clock, samples for at most
600s and stops on its private stopfile or TERM/HUP/INT. RuntimeMaxSec660 and
TimeoutStopSec25 provide an independent service bound. Cleanup freezes/stores the
own ring, checks every CPU's overrun/commit-overrun/dropped-event counts, disables
only own events/tracer, removes the own instance and checks the original global
snapshot. Failed/lost traces are not complete evidence. Inspect systemd actual
MainPID/ActiveState/ExecMainStatus as well as the scalar report for terminal state.

`function-selftest.py` is the actual own-PID, two-syscall filter/mono marker probe;
its own instance is always removed. Shared `trace_options` are intentionally not
written. The script expects the task-owned private base directory, fresh output
and normal privileged tracefs access; it does not weaken system policy.

`mp2_kernel_timing.py` parses only committed textual scalar traces after capture
is stopped. It rejects unknown/lost lines, handles exact decimal timestamps,
microsecond-truncated unique completion brackets, IRQ events, scheduler sleeping/
runnable intervals and explicit censored edges. Function entries are not returns;
statistics callbacks are not all video frames. `baseline()` uses indexed interval
search rather than repeated full-array scans to limit analysis CPU overhead.
Do not run heavy previous-capture analyses during a performance comparison.

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest discover \
  -s /absolute/path/contrib/mipad2/kernel-pacing -p test_mp2_kernel_timing.py -v
```

Thirteen synthetic tests cover timestamp precision, corruption/loss/unknown lines,
unique/ambiguous ownership, censored entries, scheduler overlap and baseline
statistics. Current published parser source was also rerun against the same
actual thermally aborted capture while no camera was active; anomaly and baseline
results match exactly. No simulated model is substituted for hardware evidence.
