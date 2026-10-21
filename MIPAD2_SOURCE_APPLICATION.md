# Source Snapshot application and candidate package (2026-10-21)

This closes a **source build / app+plugin composition** gap, not the full MiPad2
Linux/Android/Windows parity objective. Kernel remains 6.14 integrated r26. No
6.18, stable launcher replacement, boot-default change or image flash.

## Exact build

- Source Snapshot commit: `0a7ce87affa902f33599819aed5c4123a548f1c6`.
- Builder: `contrib/mipad2/portable-plugins/build_app.py`.
- Build uses `git archive` of the explicit 40-character commit, not an untracked
  working tree or an old binary. Canonical ignored `src/config.rs` is preserved.
- Cargo offline, `--locked`, release profile and source LTO; two build jobs.
  Only registry/git source caches copied from Cargo home, never its credentials
  or config. Optional target cache is a performance input, not the app artifact.
- Meson 50.0 / Cargo package 46.0.0 are the source's existing version strings.
  Neither was silently rewritten to suggest a different upstream release.
- Direct GStreamer build ABI: core/base/video/bad 1.28.7, from the private SDK.
  The locked application has no gstreamer-gl Rust crate; its runtime GL factories
  still need native target qualification. No mixed-host GL SDK was bundled.
- Prefix: `/usr/local/lib/mipad2-camera-source-0a7ce87affa9`.
- Exported application SHA-256:
  `16bd4d3ea2d4347a7c049784760b0c305150c666241700671032e9e211bf13d9`.
- 65 app files, 5,835,755 bytes before candidate overrides: executable, private
  GSettings schema, GResource bundle, icons and translations. No app RPATH or
  RUNPATH. Original build artifact left unchanged.
- Builder receipt SHA-256:
  `10c8f6bb850570674d592f6afed7ecfe0e15af1ab4aec7e195488d0a2c240c9d`.

Example (explicit caller-provided source caches / matching private SDK):

```sh
python3 contrib/mipad2/portable-plugins/build_app.py \
  --source-root /path/to/snapshot \
  --source-commit 0a7ce87affa902f33599819aed5c4123a548f1c6 \
  --output /new/build/output \
  --prefix /usr/local/lib/mipad2-camera-source-0a7ce87affa9 \
  --cargo-cache /path/to/cached/cargo-home --jobs 2
```

This is a pinned repeatable build procedure, not a claim of bit-identical builds
across arbitrary Rust/compiler/host toolchains. Preserve the emitted receipt and
component logs. Failed outputs are not reused or erased.

## Full candidate composition

Flash tooling now has `scripts/stage_camera_app.py` and
`scripts/camera_candidate_launcher.py`. All three input receipts (app, functional
plugins, AAC) require independent explicit SHA-256 pins. Hash, size, exact file
membership, path, ABI, ELF architecture/dependencies and absent RPATH checks run
before creating a new output. Existing destinations are rejected.

The package contains the source app plus **four source plugins** (encoding,
videofilter, VA, AAC), private resources/settings and a separately named desktop
entry. It does not include any private SDK libraries or replace system packages.
The usual rootfs build still does not automatically install this candidate.

The candidate launcher:

- Selects only the versioned package plugin directories, plus stock system
  plugins (including stock V4L2). Does not use old `/var/tmp` plugin experiments.
- Uses a separate keyfile-backed GSettings config/cache namespace. Photos/videos
  retain the user's normal XDG directories; config files are copied as data,
  never sourced as shell scripts.
- Has explicit hardware H264 + AAC settings and muted shutter defaults.
- Uses CURRENT_CAPS1 / RAW_DIRECT1 / RAW_ASYNC0 / COPY1 / ALLOCATION_GUARD1,
  producer queues disabled, Android-profile native 1080p30 / requested 720p30
  preview, H264 15 Mbps / AAC 192 kbps, VA depth2 and GPU/matrix rotation.
- Removes inherited `MIPAD2_*`, `GST_*`, dynamic-library injection and renderer
  debug overrides before setting the production allowlist. No diagnostic actions,
  background capture, per-frame tracing, pacing/control files or retired INPUT_ONE.
- Uses `DBusActivatable=false`; activation must execute the wrapper rather than
  bypassing it. Snapshot retains its upstream portal/application ID; close any
  existing Snapshot instance first. The wrapper refuses an existing bus owner
  instead of silently activating the stable binary.

Schema/default and desktop activation choices follow upstream
[GIO Settings](https://docs.gtk.org/gio/class.Settings.html),
[GIO runtime environment](https://docs.gtk.org/gio/overview.html), and the
[Desktop Entry Specification](https://specifications.freedesktop.org/desktop-entry/latest-single/).

## Tests and limits

37 app/plugin builder contract tests and 35 flash staging tests pass in both
normal Python and `python -O` (9 new app-builder, 13 new composition/launcher
tests). The first test run's subprocess-mocking error was fixed in the test;
its failed log is retained. No build or device success inferred from a mock.

Actual candidate staging and desktop-file validation pass. 75 payload files were
installed into previously absent candidate-only paths; 292 protected software
files and the stable launcher are unchanged. Native `ldd -r` passes for the new
app and all four plugins. Private schema defaults and target GResource load /
six top-level children pass. Missing target development CLI `gresource` was
handled through the installed runtime libgio API, not by installing dev packages
or replaying the installation. No captured media or sound in those probes.

The normal production launcher then completed **six** new-app open/normal-quit
cycles (two separate groups of three), exact executable and private settings
checked each time, no diagnostic GActions/environment, no stale plugin paths,
all exit statuses zero. The new source videofilter was mapped each time.
**None of those six cycles owned `/dev/video0`.** This is application lifecycle
evidence only, not working-preview evidence. The active Wayland session initially
reported `LockedHint=yes`; GNOME/freedesktop ScreenSaver queries were unavailable.
One scoped logind unlock request did not establish camera acquisition; the
original lock request was restored. No optical/unlocked-foreground assumption is
made from a logind request. Both groups restored actual pretest controls and
preserved stable user settings. Foreground/portal/visibility diagnosis is open.

The earlier four-source-plugin full-quality dual-camera recordings used the
**old** retained R17h app, not this newly built app. Do not attribute those media
results to the new binary. Foreground UI/permission, both cameras, full native
photos, full-quality recording, independently observed preview/HW factory/native
frame conservation, camera-used suspend/resume, S5 boot, optical AE/AWB/focus and
Android/Windows comparison remain qualification work for the complete package.
No image boot/flash or broad hardware parity is established here.
