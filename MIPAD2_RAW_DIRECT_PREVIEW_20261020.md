# R17f private native-raw preview/switch/resume smoke,2026-10-20

The raw-direct source candidate is privately built/installed and passed two
bounded real dual-camera preview/switch/close runs, one before and one after a
10s RTC suspend. This supersedes only the historical not-installed/not-hardware-
tested status in MIPAD2_RAW_DIRECT_20261020.md: FULL RECORDING ACCEPTANCE REMAINS
UNPROVEN. Host focused8/workspace78 tests were for the actual committed source.

Source commit `bc31426214b39e973e3a4f4b70887188bd91928b`; private executable SHA
`c84c24315f40ef625ba962e1919bc87a9ee478fbd4ca4c70477b53dc268d3c48`. Defaults/launcher, kernel6.14 integrated R19,
boot, system packages, normal settings and old private/system plugins remain
unchanged. Source RAW_DIRECT1/COPY1/ALLOCATION_GUARD1, both producer queues0;
same full native NV12 1920x1080/30 and1280x720/30 display profile, default renderer,
content tracing0. No clock/power tuning, playback, photo or recording actions.
The harness only invokes switch-camera and quit; no new media was created.

Both physical inputs emitted native-source and shared-source frame samples at
actual NV12/1920x1080 caps. Switch changed video0 input0/front to1/rear; both
processes closed normally, copy-failure/critical logs0, exact exposure/gain restored
and both sensor/ISP runtime usage0/suspended. Actual raw-direct activation is
verified from the matching installed executable/environment and full unit log.
RTC resume succeeded, S0I3 increased9015008us, wake IRQ9, same boot, no new detected
kernel/GPU/stream-stop errors. Post-resume reopen and both-input switching passed.
Original active thermal guard/stop-timeout drop-in, GPU200..600, anti-idle and GRUB
saved/next-entry settings verified; no temporary cap overrides or trace instances.

## Important retained limitations

These are liveness/negotiation/teardown tests, NOT whole-startup or recorded cadence
acceptance. Observation windows begin after launch; the complete log stays local.
Lifetime counters are retained: initial run cumulative max PTS gap117457276ns and
offset gap2; resumed run59838150ns and offset gap2. The counters include periods
before/through observation windows and cannot be assigned to a cause/phase from
these summaries. They are not reset or dismissed as harmless warmup. No zero-drop,
startup smoothness, optical image quality or foreground animation claim is made.

No encoder/mux frame-conservation, recorded driver ownership, source->encoder delay,
full-resolution still/photo hardware, physical AV or Android parity acceptance was
performed. The previous R17e buffer-queue regression remains rejected. The tablet
root is still below the2GiB new-recording reserve: no new recording pair, reduced
reserve or additional media deletion was used to manufacture a result. Existing
traces/source remain available for offline work. R17f stays opt-in/private, with no
default promotion. Full Linux6.14/kernel+flash objective is unfinished.

All real imagery, raw frames and private logs remain tablet-local. Adjacent public
JSON is only an allowlist of scalar profile/trace-counter/PM/liveness observations.
