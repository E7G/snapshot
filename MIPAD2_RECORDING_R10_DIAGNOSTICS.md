# Mi Pad 2 r10 timing diagnostics (not a performance fix)

Opt-in MIPAD2_TRACE_CAPTURE=1 adds scalar-only pad probes at native source,
shared source, VA encoder input/output. No buffer maps or sensor writes.
Report wall-clock throughput, PTS/offset gaps and upstream QoS; offset is not
universally a sensor sequence. Weak pad references avoid retaining pipelines.
Explicit camerabin DOT dump at recording start exposes actual negotiated chain.

MIPAD2_DIAGNOSTIC_ACTIONS=1 exports a private window GAction
mipad2-record-seconds(uint32), restricted to 1..30 seconds, using normal
recording callbacks. A unique-path ownership guard prevents the timer from
stopping a later user recording. No automatic playback.

MIPAD2_PREVIEW_QOS_OFF=1 is a separate exact-device diagnostic A/B switch.
It disables GTK preview sink QoS and sets its processing deadline to zero.
Baseline leaves this flag unset; this is not globally enabled or promoted.

Hardware validation pending. Normal launcher/settings, protected r6..r9
artifacts, kernel image and boot selection remain unchanged.
