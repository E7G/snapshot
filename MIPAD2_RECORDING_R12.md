# Mi Pad 2 r12 reference-frame latency candidate

Current raw sensor path supplies real30fps. Isolated VA H264 default3 reference
frames yields front25.50 wall outputfps /25.88 PTSfps; ref1-front output30.07
wallfps /30.002 PTSfps. Both retain360 frames. Mid-run scalar Y mean/stddev
about27.3..27.9/10.0..10.5, consistent within this stationary low-light scene;
not universal lighting/quality proof. Rear nearly flat scene encodes30fps.
Strict original stage gates remain FALSE due startup wall gaps139/102ms and
output drain timing exceeding narrow bounds. Do not rewrite them as passes.

MIPAD2_H264_LOW_LATENCY=1 exact-device H264+hardware opt-in changes only
VA H264 ref-frames to1. Default/rank/fallback, bitrate2048kbps, geometry,
recording rate, audio remain unchanged. No quality-equivalence or universal
performance claim; fewer references may affect compression efficiency.
MMAP copy remains OFF for validation. Preview QoS-off comparison remains
a separate flag. Normal launcher/global environment remain unchanged.

An owned-path guarded one-second diagnostic DOT dump exposes negotiated
recording graph, not pre-start unnegotiated caps. Scalar recording probes
continue. Full GUI recording/quality and sustained repeats pending.
Primary: https://gstreamer.freedesktop.org/documentation/va/vah264enc.html
