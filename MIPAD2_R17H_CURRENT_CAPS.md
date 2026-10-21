# R17h: opt-in unchanged-native CAPS recommendation (not device validated)

## Observed R26 evidence, not a completed performance fix

Kernel6.14 integratedR26/built6074419168fbb70aa5685e976874da6cf86d1fae,
R17g app e19702936b2dda4ce0912f073e0e6564c2adc83b, actual2026-10-21 device.
The earlier same-binary full1080p/30 +720p/30 GL/15MbpsH264/192kbpsAAC crossover
remains RED. Four-copy async mode adds persistent334..338ms stage-arrival
latency and is not promoted. See the integrated kernel branch's
fix_file/MIPAD2_R26_RECORDING_CADENCE.md for its full protocol and limits.

A subsequent SINGLE FRONT30s original-quality diagnostic, using the original
thermal governor (observed2240/1840MHz maxima; no forced cap), recorded920frames
at29.903683fps, maximum MP4PTS gap123ms, one>100ms. Thermal peak67C. Its gate
remains RED and it is explicitly instrumented, not unprofiled/foreground or
optical acceptance. Native/encoder-input/encoder-output/parser/MP4/decode counts
all920, exact native PTS+offset joins, no adjacent exact decoded repeats; AAC
decode clean. Mux relative PTS residual<=one declared333333.333ns tick.

A private2.863MB perf6.14 build was staged outside system executable/package
paths: exact signed-index libdw0.195 development/runtime packages extracted
privately on the build host, no system package installs. Device hardware
cycles:u and DWARF post-unwinding passed a positive control:201 actual stacks
contain all three known non-inlined functions in a frame-pointer-omitted test.

The bounded15s sampler produced1,664 parsable user-cycle samples spanning
14.076645s, with no parser-unknown/lost records. Actual sample span relative
first recorded native frame:-1.369154251s..12.707490484s, NOT the whole30s.
153 samples belong to the actual native source TID;106 source leaf symbols
are unresolved. Exact integer CLOCK_MONOTONIC/TID joins find five samples in
one45.645598ms source-guard CAPS query during recording. All five stacks
include gst_caps_intersect_full and gst_structure_intersect; leaf symbols are
gst_value_intersect(2), g_utf8_validate(1), unresolved(2). Inclusive presence
counts are not additive CPU time, a unique-lock identity, exhaustive attribution
or proof this one query explains the full123ms gap. They justify testing a
repeated-negotiation optimization; do not invent an all-known CPU profile.

Original sampler/controller failure receipts are retained. Our worker expected
return0 after sending SIGINT, but perf6.14 intentionally restores the signal's
default disposition and re-raises it in record__sig_exit (builtin-record.c).
The actual return is-2; an independent read-only audit verifies the finalized
PERFILE2 header, bounded nonzero data section, successful complete perf-script
decoding and14s actual sampled span. No camera rerun repairs that bookkeeping.
An initial EOS count audit also rejected one empty shutdown EOS; corrected
analysis retains that empty EOS and selects the one real nonempty encoder EOS
epoch, with no unfinished encoded tail.17 parser/scope/EOS tests pass normal
and-O on host and device. Raw addresses, stacks, media and pixel hashes stay
tablet-private. No reboot, audio tones, thermal disabling or default promotion.

## The candidate and its boundaries

MIPAD2_CAPTURE_CURRENT_CAPS=1 is DEFAULT OFF. It requires Mi Pad2 and the
already validated RAW_DIRECT path (physical AtomISP NV12 +copy/allocation guard,
other source queues disabled and the existing native-profile checks). Other
devices and unvalidated source routes are unchanged.

Only a CAPS query entering the physical source capsfilter may be answered at
this boundary. Its explicitly configured native profile AND negotiated sink
caps must both be fixed positive-size/rate system-memory NV12, and the active
caps must remain a subset of the configured request. The response is the
actual negotiated caps intersected with the optional query filter; colorimetry
and all negotiated fields are preserved. Incompatible filters fall through
normal downstream negotiation, including a changed colorimetry requirement.

No current caps, ANY/ranged/incomplete config, a new photo/video mode, another
raw format or another memory domain also falls through. No capability cache or
permanent mode freeze. The element reference is weak, not a reference cycle.
ACCEPT_CAPS, ALLOCATION, real serialized CAPS events, state/error flow, buffers,
PTS/duration/offsets, recording/audio quality and EOS remain on their old paths.
This is a constrained active-format recommendation, not fabricated hardware
capability or forced downstream acceptance. It is not an argument for generally
short-circuiting CAPS queries in unpinned pipelines.

Eight focused actual-GStreamer tests and the full95-test release workspace pass:
- scope truth table, pre-negotiation and incomplete/ranged fallbacks;
- video1080p30 ->front/full-photo2560x1920/15 ->rear3264x2448/15 ->video30;
- real before/after downstream-query negative control, unchanged acceptance/
  allocation dispatch, incompatible colors/format/memory filter fallbacks;
- every synthetic full-size NV12 byte, size, offset, PTS, duration and EOS;
- no element reference cycle and no support claim for arbitrary elements.

These host tests do NOT establish real camera/GL interoperability, all dynamic
renegotiation races, hardware JPEG/photo behavior, improved recorder cadence,
lower optical latency or Android/Windows parity. Next: private SAME-R17h-binary
flagOFF/ON physical comparison without lowering quality, followed by both-input
photo/record/reopen/resume checks if it helps. Keep R17g and stable launchers
intact; do not make this feature default on host tests alone.
