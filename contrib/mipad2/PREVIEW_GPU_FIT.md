# Experimental Mi Pad 2 GPU preview fit (default OFF)

## Internal viewfinder converter correction (CONT36, still default OFF)

After the output-pad fix, live GPU-ON preview has real native, shader and display
flow at approximately30 fps for both cameras, but still performs1315 software
NV12 1920x1080 ->1280x720 conversions. The retained trace identifies
`vfbin-videoscale`, not our optional scaler, with19.929768 s exclusive elapsed
time. That candidate is therefore NOT a demonstrated CPU/power improvement.

Only when the explicit GPU branch is selected, configure camerabin at NULL using
the registered `no-viewfinder-conversion` flag nickname. OR this into its current
typed flags; preserve audio, video-recording and still-image conversion flags.
GStreamer propagates it into the internal viewfinderbin before NULL->READY.
The default CPU branch does not call this configuration helper.

Six real-device Rust tests now pass. They include8 combinations of the other
conversion flags, idempotence, late-state and wrong-element rejection, and the
actual bin/rotation linkage plus missing-output-pad mutation. No camera is put
into READY/PLAYING by the flag tests. Live retesting of this correction, detailed
quality comparison and thermal/recording qualification are still required.

## Output-pad correction (CONT36, still default OFF)

The first same-binary GPU-OFF preview passes source/display trace coverage.
The GPU-ON launch aborts before streaming: the candidate bin omitted its output
ghost pad and could not link to glvideoflip. That run is a failure, not a zero-CPU
success. Its core and partial trace are retained privately; original signal6 and
controller failure1 were recorded before clearing only ended transient units.
The empty converter output is not interpreted as zero calls.

The bin now exposes both sink and src ghost pads. A fourth real-device Rust
test builds and links GL upload -> conversion -> actual fit bin -> rotation ->
fake display without drawing, then removes the output pad and confirms that
the exact old missing-source defect fails linking. All4 tests pass on the
tablet. This does not qualify live GPU-ON preview, picture quality, sustained
performance or power. The old e779569b app package must not be promoted.

Enable only in an explicit private process with `MIPAD2_PREVIEW_GPU_SCALE=1`.
The existing Mi Pad 2 and Android-video-profile gates must also be true, and
the `glshader` factory must exist. Without these conditions the previous CPU
preview branch remains unchanged. No launcher enables this option by default.

Measured normal preview:1139 converter calls are NV12 1920x1080 -> NV12
1280x720,16.859873 CPU seconds across the bounded lifecycle. This motivates
the candidate; it does not prove the candidate's performance or thermal gain.
The actual system video library is ORC-enabled; the SDK's ORC-disabled build
is not the library observed in the tablet's normal process.

Candidate branch: display-only videorate -> existing GL upload/color conversion
-> RGBA shader fit -> existing GPU orientation/display sink. Recording and JPEG
capture retain their original source caps; no bitrate, quality, clock or thermal
policy changes. Source DAR includes pixel aspect ratio and is updated on
serialized RGBA CAPS events. The shader uses output width/height uniforms plus
source DAR, preserves aspect and adds opaque black borders. It does not stretch
4:3 photos into16:9. Invalid input caps post a negotiation error. A GL context or
shader failure is not dynamically hidden by a CPU fallback.

Validation so far: compiled against pinned GStreamer1.28.7; exact Rust test
binary run on the tablet passes3 tests (32 opt-in combinations, DAR cases and
serialized CAPS changes). Initial host runtime lacks the GL plugin and failed
that test; the first untemplated test-pad fixture also failed on-device. Both
failures are retained; a templated sink fixture passes without skipping tests.
Synthetic real-GPU quadrant/border tests pass for1920x1080,3264x2448,
2560x1920,720x1280 and non-square pixels at1280x720 output. No physical camera
or audio device is used by those shader tests.

Still unqualified: live-app preview/recording, sustained CPU/GPU/power benefit,
fine-detail/chroma resampling quality, photo/video transitions, rotation,
resume/reopen, and full application packaging/installation. Defaults remain OFF.
The output is not pixel-identical to the old CPU YUV resampling path; geometric
synthetic tests are not real-scene image-quality or Android-parity evidence.
