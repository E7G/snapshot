# Mi Pad 2 r11 MMAP lease-isolation candidate

r10 automatic actual GUI recording reproduces stutter without user steps.
Baseline NV12 1920x1440 requested30fps:152 packets,16.0167 effective fps,
maxgap388ms,7 gaps>100ms. Native/shared source averages20fps, VA encoder
input/output15.3fps. Underflow preview QoS propagates into shared transforms.
Same binary with preview QoS disabled and processing deadline zero:176 packets,
17.0289fps,maxgap97ms,no gaps>100ms. Encoder/native all16.4fps. This removes
long gaps but is NOT30fps and is NOT a complete fix. Both retain normal settings.

Previous isolated native-camera VA H264360-frame tests achieve30fps on each
sensor. Concurrent GUI exposes additional source-buffer retention/backpressure.
R11 exact-device opt-in MIPAD2_CAPTURE_COPY=1 deep-copies native v4l2src
buffers before decodebin/wrapper branches, releasing the scarce MMAP lease
without waiting for preview/encoder references. This is an experiment, not
a proven cause until matched hardware A/B. Geometry/rate/pixels unchanged,
no recording branch leak, no zero-copy performance claim. Allocation failure
retains original frame and logs a bounded counter; never silently drops.

Actual full NV124149248-byte unit test validates identical bytes, independent
memory, PTS/DTS/duration/offset/flags and VideoMeta plane offsets/strides.
Dropping original releases tracked memory owner while copy remains readable.
Private diagnostic switch-camera GAction reuses existing UI camera-switch code
and rejects switching while recording; no exposed normal-session actions.

Hardware validation pending. Kernel/default launcher/packages remain unchanged.
https://gstreamer.freedesktop.org/documentation/gstreamer/gstbuffer.html

## Unattended diagnostic startup
Desktop locked before r11 first foreground activation. Both guarded trials
correctly stopped at no_recent_native_source_frames; neither recorded anything.
Do not interpret DBus action success as actual stream/camera switching success.
Loginctl unlock signal did not change LockedHint; no repeated unlock attempt.
R11b process opt-ins MIPAD2_DIAGNOSTIC_ACTIONS=1 together with
MIPAD2_DIAGNOSTIC_BACKGROUND_CAPTURE=1 start the NORMAL camera/portal
initialization without waiting for first foreground activation. The is-active
handler is disconnected to prevent duplicate starts. Existing permission
requests and shutdown hooks remain intact. No unlock or permission bypass,
no default background capture. Locked-desktop pipeline tests are not visible
GTK responsiveness/subjective quality evidence.

## Hardware results: not a complete recording fix
R11b same-binary background capture comparison: copy181 packets/18.4156fps
(maxgap74ms), no-copy180 packets/18.0839fps(maxgap70.334ms); no gaps>100ms.
Copy raises average app CPU69.91% versus61.83%. This does NOT establish
MMAP starvation as primary cause or justify default copying. Copy remains opt-in.
Audio-disabled diagnostic still16.265fps; GST_CAPS logging also enabled in
that case, so it is not a clean timing A/B, but removing audio alone does not
produce30fps. Audio-enabled settings are retained for normal candidates.
R11 locked-desktop foreground-only attempts recorded nothing; source readiness
gates blocked correctly. R11b explicitly authorized normal-portal initialization
successfully starts native NV12 and automated recordings without user action.

Fresh R18 raw capture360 frames each: front30.008 and rear30.012 wall fps,
max wall gap34.019/34.473ms, no duplicate/reverse PTS. Separate VA front25fps,
rear30fps. Previous all-headless30fps evidence is not substituted for these
new results. MP4 diagnostic warm-front FFmpeg null sink complained about
rounded duplicate DTS; this is retained, not labelled a successful decode gate.
