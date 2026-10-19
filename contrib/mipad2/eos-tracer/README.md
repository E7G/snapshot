# Bounded scalar EOS tracer (diagnostic v1)

Build gstmp2eos.c as a private shared GStreamer plugin using pkg-config gstreamer-1.0.
Enable only in a private test process with GST_TRACERS=mipad2-eos and a fresh
GST_PLUGIN_PATH/registry. It logs paths, timestamps, events, counters and non-OK
flows, never maps/retains buffers or exports pixels/digests. Log cap2000; records
after the cap are missing, not evidence of no events. Push/send hooks share an
event label, so duplicate observations are not distinct EOS. This diagnostic
perturbs timing and must not replace independent frame-conservation tests.
pipeline-test.c provides bounded host EOS/error testing without gst-launch.
