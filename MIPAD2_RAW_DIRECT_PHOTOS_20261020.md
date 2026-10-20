# R17f: private real native full-resolution stills,2026-10-20

Same opt-in RAW_DIRECT1 executable as the prior preview/resume evidence:
`c84c24315f40ef625ba962e1919bc87a9ee478fbd4ca4c70477b53dc268d3c48`.
Unchanged R19 kernel, stable default R6 launcher/system plugins, ordinary app
settings and original thermal guard. COPY1/ALLOCATION_GUARD1; both producer
queues OFF; no profile/FPS/bitrate/power reduction and no playback/tones.

Actual front OV5693:2560x1920 JPEG(1366602bytes); rear T4KA3:3264x2448
JPEG(1134041bytes). Both local JPEGs fully decode one frame with FFmpeg exit0
and empty error output. The test observed actual native capture geometry
and matching physical sensor selection, source-ready and preview-restoration
logs. Actual front pad2576x1936 crop24,10,2576,1936; rear3280x2464
crop0,0,3280,2464. No scaling a1080p screenshot into a claimed full-res photo.

Both camera photos return to actual1920x1080 NV12 native/shared-source output;
four bounded preview observation phases cover launch, each photo restoration
and sensor switching. Closing exits0, copy failures/critical logs0; original
exposure/gain restored and sensor/ISP runtime usage0/suspended. Fresh checkpoint
verifies protected application/system/thermal hashes, anti-idle, GPU200..600,
stable GRUB and unchanged boot. Explicit selected new kernel error patterns0.

## Limits retained

This proves JPEG creation/decodability and mode restoration, not focus, color,
automatic exposure/WB, foreground smoothness, shutter latency or Android parity.
Per-phase cumulative source gaps/offset gaps remain in adjacent scalar JSON;
they include launch/switch/still-mode transitions and are not reset or trimmed
into a zero-drop claim. No full recording pair or new RTC-after-photo trial was
run. The previous preview/RTC result is a separate test, not this result.

All real photos stay on tablet in this trial's private picture namespace; no
images/encoded bytes/pixel hashes/EXIF are exported. Only dimensions, file
sizes, sensor geometry, counters, decode exit and state are published. The
2GiB new-recording reserve remains unmet. No old media were deleted this turn.
Private opt-in only: default launcher and deployed kernel have not changed.
Full Linux6.14/kernel+flash objective remains unfinished.
