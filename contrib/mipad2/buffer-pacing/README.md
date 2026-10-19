# Standard V4L2/VB2 buffer timing diagnostics

See [copy-only negative result and host-tested-only candidate](../../../MIPAD2_CAPTURE_DECOUPLE_20261019.md).
The original kernel timing module lives in ../kernel-pacing. Use both directories
on the import path; normal tests do not need root, a camera or a display:

```sh
PYTHONDONTWRITEBYTECODE=1 PYTHONPATH=/absolute/path/contrib/mipad2/kernel-pacing \
python3 -m unittest discover -s /absolute/path/contrib/mipad2/buffer-pacing \
  -p test_mp2_buffer_timing.py -v
```

Eleven synthetic parser tests cover spaced/comma fields, nested timecode keys,
timeval truncation, indexed owner uniqueness, ambiguous/missing/wrong-minor joins,
zero-driver-owned intervals/censoring and refusing nearest-stamp guessing.
Raw owner strings are local opaque tokens only; analyze() returns ordinal queue
IDs, not kernel owner tokens. Kernel candidates around stream switches or preview
must not be relabeled recording failures without an independent recording join.
VB2 driver-owned count is not the private CSS queue depth. Event counter/time
sampling has normal concurrency/printed precision limits; retain uncertainty.

capture-buffer-pacing.py is the exact date-scoped root controller used by this
task, not a generic installer. It requires a new own private base and nonexisting
report/stop/instance, verified R19 boot/idle/cooldown and a live configured systemd
PID before dispatch. The finite watchdog, exact PCI IRQ/function filters, instance-
only configuration and cleanup/global-state/loss checks follow ../kernel-pacing.
V4L2 events filter minor0; generic VB2 events filter capture type1 and are then
associated strictly to successful indexed minor0 dequeues. No global trace_options,
dynamic probes or payload memory reads. Keep raw traces private; do not reuse
stale names or restart pending jobs merely because an observation timed out.

Optional experimental source candidate flags are documented in the main report.
Do not place them in the normal launcher. Real device tests remain required;
synthetic byte conservation/early lease release cannot establish optical quality,
latency, A/V sync, lifecycle or kernel DMA safety.
