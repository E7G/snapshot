// SPDX-License-Identifier: GPL-3.0-or-later
use std::fs;
use std::os::unix::io::OwnedFd;
use std::process::Command;
use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;

use adw::prelude::*;
use adw::subclass::prelude::*;
use anyhow::Context;
use ashpd::desktop::camera;
use gettextrs::gettext;
use gtk::CompositeTemplate;
use gtk::{gio, glib};

use super::CameraControls;
use crate::enums::ControlsLayout;
use crate::{config, utils};

mod imp {
    use std::cell::{Cell, OnceCell, RefCell};

    use gtk::{CallbackAction, Shortcut, ShortcutController, ShortcutTrigger};

    use crate::CaptureMode;

    use super::*;

    #[derive(Debug, Default, CompositeTemplate, glib::Properties)]
    #[template(resource = "/org/gnome/Snapshot/ui/camera.ui")]
    #[properties(wrapper_type = super::Camera)]
    pub struct Camera {
        pub selection: gtk::SingleSelection,
        pub provider: OnceCell<aperture::DeviceProvider>,
        pub players: RefCell<Option<gtk::MediaFile>>,
        settings: OnceCell<gio::Settings>,
        pub permission_denied: Cell<bool>,
        pub mipad2_input: Cell<u32>,
        pub mipad2_vcm: RefCell<Option<fs::File>>,
        pub mipad2_af_generation: Arc<AtomicU32>,

        pub recording_duration: Cell<u32>,
        pub recording_source: RefCell<Option<glib::source::SourceId>>,

        #[property(get, set = Self::set_capture_mode, explicit_notify, default)]
        capture_mode: Cell<crate::CaptureMode>,

        #[template_child]
        pub single_landscape_bp: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub dual_landscape_bp: TemplateChild<adw::Breakpoint>,
        #[template_child]
        pub dual_portrait_bp: TemplateChild<adw::Breakpoint>,

        #[template_child]
        pub recording_revealer: TemplateChild<gtk::Revealer>,
        #[template_child]
        pub recording_label: TemplateChild<gtk::Label>,

        #[template_child]
        pub viewfinder: TemplateChild<aperture::Viewfinder>,
        #[template_child]
        pub flash_bin: TemplateChild<crate::FlashBin>,
        #[template_child]
        pub qr_screen_bin: TemplateChild<crate::QrScreenBin>,
        #[template_child]
        pub stack: TemplateChild<gtk::Stack>,

        #[template_child]
        pub guidelines: TemplateChild<crate::GuidelinesBin>,

        #[template_child]
        pub camera_controls: TemplateChild<crate::CameraControls>,

        #[template_child]
        pub bottom_sheet: TemplateChild<adw::BottomSheet>,
        #[template_child]
        pub qr_bottom_sheet: TemplateChild<crate::QrBottomSheet>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for Camera {
        const NAME: &'static str = "Camera";
        type Type = super::Camera;
        type ParentType = adw::BreakpointBin;

        fn class_init(klass: &mut Self::Class) {
            klass.bind_template();
            klass.bind_template_callbacks();
            klass.set_css_name("camera");
        }

        fn instance_init(obj: &glib::subclass::InitializingObject<Self>) {
            obj.init_template();
        }
    }

    #[gtk::template_callbacks]
    impl Camera {
        fn set_capture_mode(&self, capture_mode: crate::CaptureMode) {
            if capture_mode != self.capture_mode.replace(capture_mode) {
                match capture_mode {
                    CaptureMode::Picture => {
                        self.obj().set_shutter_mode(crate::ShutterMode::Picture);
                    }
                    CaptureMode::Video => {
                        self.obj().set_shutter_mode(crate::ShutterMode::Video);
                    }
                    CaptureMode::QrDetection => (),
                };
                self.obj()
                    .set_detect_codes(matches!(capture_mode, CaptureMode::QrDetection));

                self.obj().notify_capture_mode();
            }
        }

        pub fn settings(&self) -> &gio::Settings {
            self.settings
                .get_or_init(|| gio::Settings::new(config::APP_ID))
        }

        #[template_callback]
        fn change_breakpoint(&self, breakpoint: adw::Breakpoint) {
            let obj = self.obj();

            if breakpoint.eq(&self.dual_landscape_bp.get())
                || breakpoint.eq(&self.dual_portrait_bp.get())
            {
                obj.add_css_class("mobile");
            } else {
                obj.remove_css_class("mobile");
            }
        }
    }

    #[glib::derived_properties]
    impl ObjectImpl for Camera {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let provider = aperture::DeviceProvider::instance();
            self.provider.set(provider.clone()).unwrap();

            let create_shortcut = |shortcut, value: CaptureMode| {
                Shortcut::new(
                    ShortcutTrigger::parse_string(shortcut),
                    Some(CallbackAction::new(glib::clone!(
                        #[weak]
                        obj,
                        #[upgrade_or]
                        glib::Propagation::Proceed,
                        move |_, _| {
                            obj.set_capture_mode(value);
                            glib::Propagation::Proceed
                        }
                    ))),
                )
            };

            let controller = ShortcutController::new();
            controller.set_scope(gtk::ShortcutScope::Managed);
            controller.add_shortcut(create_shortcut("p", CaptureMode::Picture));
            controller.add_shortcut(create_shortcut("r", CaptureMode::Video));

            obj.add_controller(controller);

            provider.connect_camera_added(glib::clone!(
                #[weak]
                obj,
                move |provider, _| {
                    obj.update_cameras_button(provider);
                }
            ));
            provider.connect_camera_removed(glib::clone!(
                #[weak]
                obj,
                move |provider, _| {
                    obj.update_cameras_button(provider);
                }
            ));
            obj.update_cameras_button(provider);

            self.viewfinder.connect_state_notify(glib::clone!(
                #[weak]
                obj,
                move |_| {
                    obj.update_state();
                }
            ));

            self.viewfinder.connect_code_detected(glib::clone!(
                #[weak]
                obj,
                move |_, code| {
                    match std::str::from_utf8(&code) {
                        Ok(code) => {
                            log::debug!("Detected QR code: {code}");
                            obj.imp().bottom_sheet.set_open(true);
                            obj.imp().qr_bottom_sheet.set_contents(code);
                        }
                        Err(err) => {
                            log::error!("Could not decode QR code into utf8: {err}");
                        }
                    }
                }
            ));

            self.qr_screen_bin.set_viewfinder(self.viewfinder.clone());

            obj.update_state();

            self.viewfinder.connect_is_recording_notify(glib::clone!(
                #[weak]
                obj,
                move |viewfinder| {
                    let window = viewfinder.root().and_downcast::<crate::Window>().unwrap();

                    if viewfinder.is_recording() {
                        obj.set_shutter_mode(crate::ShutterMode::Recording);
                        window.inhibit("Recording Video");
                        obj.show_recording_label();
                    } else {
                        obj.hide_recording_label();
                        window.uninhibit();
                        if matches!(obj.shutter_mode(), crate::ShutterMode::Recording) {
                            obj.set_shutter_mode(crate::ShutterMode::Video);
                        }
                    }
                }
            ));

            self.selection.set_model(Some(provider));
            self.selection.connect_selected_item_notify(glib::clone!(
                #[weak]
                obj,
                move |selection| {
                    if let Some(selected_item) = selection.selected_item() {
                        let camera = selected_item.downcast::<aperture::Camera>().ok();

                        if matches!(
                            obj.imp().viewfinder.state(),
                            aperture::ViewfinderState::Ready | aperture::ViewfinderState::Error
                        ) {
                            obj.set_camera_inner(camera);
                        }
                    }
                }
            ));

            self.camera_controls.set_selection(self.selection.clone());
            self.camera_controls.connect_camera_switched(glib::clone!(
                #[weak]
                obj,
                move |_: &CameraControls| {
                    obj.camera_switched();
                }
            ));
            self.camera_controls.connect_camera_rotated(glib::clone!(
                #[weak]
                obj,
                move |_: &CameraControls| {
                    obj.imp().viewfinder.rotate_clockwise();
                }
            ));

            self.settings()
                .bind(
                    "show-composition-guidelines",
                    &*self.guidelines,
                    "draw-guidelines",
                )
                .build();

            self.settings()
                .bind(
                    "enable-audio-recording",
                    &*self.viewfinder,
                    "disable-audio-recording",
                )
                .invert_boolean()
                .build();

            self.settings()
                .bind("capture-mode", &*obj, "capture-mode")
                .build();

            let format = aperture::preferred_video_format(
                self.settings().boolean("enable-hardware-encoding"),
            );
            log::debug!("Selected available recording profile: {:?}", format);
            self.viewfinder.set_video_format(format);

            self.settings()
                .bind(
                    "enable-hardware-encoding",
                    &*self.viewfinder,
                    "enable-hw-encoding",
                )
                .get_only()
                .build();

            obj.connect_current_breakpoint_notify(glib::clone!(
                #[weak(rename_to = obj)]
                self,
                move |imp| {
                    if imp.current_breakpoint().is_none()
                        || imp
                            .current_breakpoint()
                            .is_some_and(|breakpoint| breakpoint.eq(&obj.dual_portrait_bp.get()))
                    {
                        imp.add_css_class("portrait");
                    } else {
                        imp.remove_css_class("portrait");
                    }
                }
            ));
        }
    }

    impl WidgetImpl for Camera {}
    impl BreakpointBinImpl for Camera {}
}

fn is_mipad2() -> bool {
    let vendor = fs::read_to_string("/sys/class/dmi/id/sys_vendor").unwrap_or_default();
    let product = fs::read_to_string("/sys/class/dmi/id/product_name").unwrap_or_default();
    vendor.trim() == "Xiaomi Inc" && product.trim() == "Mipad2"
}

fn v4l2_output(args: &[&str]) -> Option<String> {
    let output = Command::new("v4l2-ctl").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn mipad2_set_v4l2_input(input: u32) -> anyhow::Result<()> {
    let arg = format!("--set-input={input}");
    let status = Command::new("v4l2-ctl")
        .args(["-d", "/dev/video0", &arg])
        .status()
        .context("Failed to execute v4l2-ctl")?;
    anyhow::ensure!(status.success(), "v4l2-ctl failed to switch input");
    Ok(())
}

const MIPAD2_FRONT_RED_BALANCE: u32 = 1440;
const MIPAD2_FRONT_BLUE_BALANCE: u32 = 1581;
const MIPAD2_REAR_RED_BALANCE: u32 = 1441;
const MIPAD2_REAR_BLUE_BALANCE: u32 = 1561;

fn mipad2_find_subdev(name_prefix: &str) -> anyhow::Result<String> {
    for index in 0..16 {
        let name_path = format!("/sys/class/video4linux/v4l-subdev{index}/name");
        let Ok(name) = fs::read_to_string(&name_path) else {
            continue;
        };
        if name.trim().starts_with(name_prefix) {
            return Ok(format!("/dev/v4l-subdev{index}"));
        }
    }

    anyhow::bail!("Could not find V4L2 subdev matching {name_prefix}")
}

fn mipad2_set_color_balance(input: u32) -> anyhow::Result<()> {
    let (sensor_name, red, blue) = if input == 0 {
        (
            "ov5693",
            MIPAD2_FRONT_RED_BALANCE,
            MIPAD2_FRONT_BLUE_BALANCE,
        )
    } else {
        ("t4ka3", MIPAD2_REAR_RED_BALANCE, MIPAD2_REAR_BLUE_BALANCE)
    };
    let device = mipad2_find_subdev(sensor_name)?;
    let ctrl = format!("red_balance={red},blue_balance={blue}");
    let status = Command::new("v4l2-ctl")
        .args(["-d", &device, "--set-ctrl", &ctrl])
        .status()
        .context("Failed to execute v4l2-ctl for Mi Pad 2 white balance")?;
    anyhow::ensure!(
        status.success(),
        "v4l2-ctl failed to set Mi Pad 2 color balance"
    );
    Ok(())
}

fn mipad2_set_focus(position: u32) -> anyhow::Result<()> {
    let device = mipad2_find_subdev("dw9719")?;
    let ctrl = format!("focus_absolute={position}");
    let status = Command::new("v4l2-ctl")
        .args(["-d", &device, "--set-ctrl", &ctrl])
        .status()
        .context("Failed to execute v4l2-ctl for Mi Pad 2 VCM")?;
    anyhow::ensure!(status.success(), "v4l2-ctl failed to set Mi Pad 2 focus");
    Ok(())
}

const MIPAD2_REAR_OTP_PATH: &str = "/sys/bus/nvmem/devices/mipad2-t4ka3-otp/nvmem";
const MIPAD2_AF_FALLBACK_INFINITY: u32 = 237;
const MIPAD2_AF_FALLBACK_MACRO: u32 = 366;

#[derive(Clone, Copy, Debug)]
struct Mipad2AfRange {
    infinity: u32,
    macro_pos: u32,
}

impl Mipad2AfRange {
    fn default_focus(self) -> u32 {
        self.infinity
    }
}

fn mipad2_checksum255(data: &[u8]) -> u8 {
    (data.iter().map(|value| u32::from(*value)).sum::<u32>() % 255) as u8
}

fn mipad2_af_range() -> Mipad2AfRange {
    let fallback = Mipad2AfRange {
        infinity: MIPAD2_AF_FALLBACK_INFINITY,
        macro_pos: MIPAD2_AF_FALLBACK_MACRO,
    };

    let Ok(data) = fs::read(MIPAD2_REAR_OTP_PATH) else {
        return fallback;
    };

    let valid = data.len() == 578
        && data[0] == 1
        && mipad2_checksum255(&data[1..15]) == data[15]
        && data[16] == 1
        && mipad2_checksum255(&data[17..31]) == data[31]
        && data[32] == 1
        && mipad2_checksum255(&data[33..303]) == data[303]
        && data[304] == 1
        && mipad2_checksum255(&data[305..577]) == data[577];
    if !valid {
        log::warn!("Mi Pad 2 rear OTP failed validation; using fallback autofocus range");
        return fallback;
    }

    let infinity = u16::from_be_bytes([data[19], data[20]]) as u32;
    let macro_pos = u16::from_be_bytes([data[21], data[22]]) as u32;
    if infinity >= macro_pos || macro_pos > 1023 {
        log::warn!(
            "Mi Pad 2 rear OTP has invalid autofocus range {infinity}-{macro_pos}; using fallback"
        );
        return fallback;
    }

    Mipad2AfRange {
        infinity,
        macro_pos,
    }
}

// Xiaomi's Android T4KA3/DW9761 stack consumes the factory infinity/macro
// endpoints instead of treating the whole 0..1023 actuator range as useful.
// Read those endpoints from the kernel's validated read-only NVMEM provider
// when available; older kernels fall back to the values from the test tablet.
fn mipad2_frame_focus_score(frame: &[u8]) -> anyhow::Result<f64> {
    const WIDTH: usize = 320;
    const HEIGHT: usize = 180;
    const FRAME_SIZE: usize = WIDTH * HEIGHT;
    const KERNEL: [f64; 5] = [1.0, 4.0, 6.0, 4.0, 1.0];

    anyhow::ensure!(
        frame.len() == FRAME_SIZE,
        "Mi Pad 2 autofocus frame has unexpected size: {}",
        frame.len()
    );

    let mut blurred = vec![0.0f64; FRAME_SIZE];
    for y in 2..(HEIGHT - 2) {
        for x in 2..(WIDTH - 2) {
            let mut sum = 0.0;
            for (ky, wy) in KERNEL.iter().enumerate() {
                for (kx, wx) in KERNEL.iter().enumerate() {
                    let yy = y + ky - 2;
                    let xx = x + kx - 2;
                    sum += frame[yy * WIDTH + xx] as f64 * wy * wx;
                }
            }
            blurred[y * WIDTH + x] = sum / 256.0;
        }
    }

    let x0 = WIDTH / 3;
    let x1 = WIDTH * 2 / 3;
    let y0 = HEIGHT / 3;
    let y1 = HEIGHT * 2 / 3;
    let mut magnitudes = Vec::with_capacity((x1 - x0) * (y1 - y0));

    for y in (y0 + 1)..(y1 - 1) {
        for x in (x0 + 1)..(x1 - 1) {
            let tl = blurred[(y - 1) * WIDTH + x - 1];
            let tc = blurred[(y - 1) * WIDTH + x];
            let tr = blurred[(y - 1) * WIDTH + x + 1];
            let ml = blurred[y * WIDTH + x - 1];
            let mr = blurred[y * WIDTH + x + 1];
            let bl = blurred[(y + 1) * WIDTH + x - 1];
            let bc = blurred[(y + 1) * WIDTH + x];
            let br = blurred[(y + 1) * WIDTH + x + 1];

            let gx = -tl + tr - 2.0 * ml + 2.0 * mr - bl + br;
            let gy = -tl - 2.0 * tc - tr + bl + 2.0 * bc + br;
            magnitudes.push((gx * gx + gy * gy).sqrt());
        }
    }

    anyhow::ensure!(!magnitudes.is_empty(), "Mi Pad 2 autofocus ROI is empty");
    magnitudes.sort_by(|a, b| a.total_cmp(b));
    let p95_index = (magnitudes.len() * 95 / 100).min(magnitudes.len() - 1);
    let cap = magnitudes[p95_index];
    Ok(magnitudes
        .iter()
        .map(|value| (*value).min(cap))
        .sum::<f64>()
        / magnitudes.len() as f64)
}

fn mipad2_focus_score(sampler: &aperture::PreviewFrameSampler) -> anyhow::Result<f64> {
    // Skip two post-settle frames, score the last three; preview stays running.
    let frames = sampler
        .capture(5, Duration::from_secs(3))
        .context("Failed to sample existing Mi Pad 2 preview for autofocus")?;
    let mut scores = frames
        .iter()
        .skip(2)
        .map(|frame| mipad2_frame_focus_score(frame))
        .collect::<anyhow::Result<Vec<_>>>()?;
    scores.sort_by(|a, b| a.total_cmp(b));
    Ok(scores[scores.len() / 2])
}

fn mipad2_evaluate_focus(
    sampler: &aperture::PreviewFrameSampler,
    cancel: &AtomicU32,
    generation: u32,
    range: Mipad2AfRange,
    focus: u32,
) -> anyhow::Result<Option<f64>> {
    if cancel.load(Ordering::SeqCst) != generation {
        return Ok(None);
    }

    let focus = focus.clamp(range.infinity, range.macro_pos);
    mipad2_set_focus(focus)?;
    std::thread::sleep(Duration::from_millis(120));

    if cancel.load(Ordering::SeqCst) != generation {
        return Ok(None);
    }

    let score = mipad2_focus_score(sampler)?;
    log::debug!("Mi Pad 2 autofocus focus={focus} score={score:.4}");
    Ok(Some(score))
}

fn mipad2_autofocus(
    sampler: aperture::PreviewFrameSampler,
    cancel: Arc<AtomicU32>,
    generation: u32,
) {
    const COARSE_ACCEPT_RATIO: f64 = 1.08;
    const REFINE_ACCEPT_RATIO: f64 = 1.025;

    let range = mipad2_af_range();
    let default_focus = range.default_focus();
    let span = range.macro_pos - range.infinity;
    let coarse_step = (span / 8).max(8);
    let mut coarse = Vec::new();
    let mut focus = range.infinity;
    loop {
        coarse.push(focus);
        if focus >= range.macro_pos {
            break;
        }
        focus = (focus + coarse_step).min(range.macro_pos);
    }

    let baseline_score = match mipad2_evaluate_focus(
        &sampler,
        &cancel,
        generation,
        range,
        default_focus,
    ) {
        Ok(Some(score)) => score,
        Ok(None) => return,
        Err(err) => {
            log::warn!(
                "Mi Pad 2 autofocus baseline capture failed, keeping OTP-safe focus at {default_focus}: {err}"
            );
            if cancel.load(Ordering::SeqCst) == generation {
                let _ = mipad2_set_focus(default_focus);
            }
            return;
        }
    };

    let mut best_focus = default_focus;
    let mut best_score = baseline_score;

    for focus in coarse {
        if focus == default_focus {
            continue;
        }
        match mipad2_evaluate_focus(&sampler, &cancel, generation, range, focus) {
            Ok(Some(score)) if score > best_score => {
                best_focus = focus;
                best_score = score;
            }
            Ok(Some(_)) => {}
            Ok(None) => return,
            Err(err) => log::debug!("Mi Pad 2 autofocus coarse sample skipped at {focus}: {err}"),
        }
    }

    if best_focus != default_focus && best_score < baseline_score * COARSE_ACCEPT_RATIO {
        log::info!(
            "Mi Pad 2 autofocus peak at {best_focus} was marginal ({best_score:.4} vs baseline {baseline_score:.4}); keeping {default_focus}"
        );
        best_focus = default_focus;
        best_score = baseline_score;
    } else if best_focus != default_focus {
        for step in [8u32, 4u32] {
            let center = best_focus;
            let candidates = [
                center.saturating_sub(step).max(range.infinity),
                center.saturating_add(step).min(range.macro_pos),
            ];

            for focus in candidates {
                if focus == best_focus {
                    continue;
                }
                match mipad2_evaluate_focus(&sampler, &cancel, generation, range, focus) {
                    Ok(Some(score)) if score > best_score * REFINE_ACCEPT_RATIO => {
                        best_focus = focus;
                        best_score = score;
                    }
                    Ok(Some(_)) => {}
                    Ok(None) => return,
                    Err(err) => {
                        log::debug!("Mi Pad 2 autofocus refinement skipped at {focus}: {err}")
                    }
                }
            }
        }
    }

    if cancel.load(Ordering::SeqCst) != generation {
        return;
    }

    if let Err(err) = mipad2_set_focus(best_focus) {
        log::warn!("Could not apply Mi Pad 2 autofocus result: {err}");
    } else {
        log::info!(
            "Mi Pad 2 rear autofocus selected focus {best_focus} within OTP range {}-{} (score {best_score:.4})",
            range.infinity,
            range.macro_pos
        );
    }
}

glib::wrapper! {
    pub struct Camera(ObjectSubclass<imp::Camera>)
        @extends gtk::Widget, adw::BreakpointBin,
        @implements gtk::ConstraintTarget, gtk::Buildable, gtk::Accessible;
}

impl Default for Camera {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl Camera {
    pub fn new() -> Self {
        Self::default()
    }

    fn on_portal_not_allowed(&self) {
        // We don't start the device provider if we are not
        // allowed to use cameras.
        self.imp().permission_denied.set(true);
        self.update_state();
    }

    pub async fn start(&self) {
        let provider = self.imp().provider.get().unwrap();

        if is_mipad2() {
            match mipad2_set_v4l2_input(0) {
                Ok(()) => {
                    if let Err(err) = mipad2_set_color_balance(0) {
                        log::warn!("Could not set Mi Pad 2 front OTP white balance: {err}");
                    } else {
                        log::info!(
                            "Applied Mi Pad 2 front OTP white balance R={MIPAD2_FRONT_RED_BALANCE} B={MIPAD2_FRONT_BLUE_BALANCE}"
                        );
                    }
                    self.imp().mipad2_input.set(0);
                    self.imp().viewfinder.set_front_camera(true);
                    log::info!("Initialized Mi Pad 2 V4L2 camera input to 0");
                }
                Err(err) => {
                    log::warn!("Could not initialize Mi Pad 2 camera input: {err}");
                }
            }
        }

        glib::spawn_future_local(glib::clone!(
            #[weak(rename_to = obj)]
            self,
            #[strong]
            provider,
            async move {
                if let Err(err) = ashpd::register_host_app(config::APP_ID.try_into().unwrap()).await
                {
                    log::error!(
                        "Failed to run org.freedesktop.host.portal.Registry.Register: {err}"
                    );
                }
                match stream().await {
                    Ok(fd) => {
                        if let Err(err) = provider.set_fd(fd) {
                            log::error!("Could not use the camera portal: {err}");
                        };
                    }
                    Err(err) => match err.downcast_ref::<ashpd::Error>() {
                        Some(ashpd::Error::Portal(ashpd::PortalError::NotAllowed(err))) => {
                            log::warn!("Permission to use the camera portal denied: {err:#?}");
                            obj.on_portal_not_allowed();
                            return;
                        }
                        Some(ashpd::Error::Zbus(ashpd::zbus::Error::MethodError(
                            name,
                            _,
                            message,
                        ))) if *name == "org.freedesktop.portal.Error.NotAllowed" => {
                            log::warn!("Permission to use the camera portal denied: {message}");
                            obj.on_portal_not_allowed();
                            return;
                        }
                        _ => (),
                    },
                }

                if let Err(err) = provider.start_with_default(glib::clone!(
                    #[weak]
                    obj,
                    #[upgrade_or]
                    false,
                    move |camera| {
                        let stored_id = obj.imp().settings().string("last-camera-id");
                        !stored_id.is_empty() && id_from_pw(camera) == stored_id
                    }
                )) {
                    log::error!("Could not start the device provider: {err}");
                } else {
                    log::debug!("Device provider started");
                    obj.update_cameras_button(&provider);
                };
            }
        ));
    }

    pub async fn start_recording(&self) -> anyhow::Result<()> {
        let imp = self.imp();
        let format = imp.viewfinder.video_format();
        let filename = utils::video_file_name(format);
        let path = utils::videos_dir()?.join(filename);

        imp.viewfinder.start_recording(path)?;

        Ok(())
    }

    /// Private opt-in diagnostic; uses normal recording/UI callbacks and a
    /// unique-path ownership guard so a timer cannot stop a later user clip.
    pub async fn diagnostic_record(&self, seconds: u32) -> anyhow::Result<()> {
        if std::env::var("MIPAD2_DIAGNOSTIC_ACTIONS").as_deref() != Ok("1")
            || !(1..=30).contains(&seconds)
        {
            anyhow::bail!("Diagnostic action unavailable or invalid duration");
        }
        if self.is_recording_active() {
            anyhow::bail!("Recording already active");
        }
        self.start_recording().await?;
        let owned_path = self.imp().viewfinder.recording_location();
        let camera = self.downgrade();
        glib::timeout_add_seconds_local_once(seconds, move || {
            if let Some(camera) = camera.upgrade() {
                if owned_path.is_some()
                    && camera.imp().viewfinder.recording_location() == owned_path
                {
                    camera.stop_recording();
                }
            }
        });
        log::info!(
            "Mi Pad 2 diagnostic timed recording started: {} seconds",
            seconds
        );
        Ok(())
    }

    pub fn stop_recording(&self) {
        let imp = self.imp();
        if matches!(imp.viewfinder.state(), aperture::ViewfinderState::Ready)
            && imp.viewfinder.is_recording()
            && let Err(err) = imp.viewfinder.stop_recording()
        {
            log::error!("Could not stop camera: {err}");
        }
    }

    pub async fn take_picture(&self, format: crate::PictureFormat) -> anyhow::Result<()> {
        let imp = self.imp();
        let window = self.root().and_downcast::<crate::Window>().unwrap();

        // We enable the shutter whenever picture-stored is emitted.
        window.set_shutter_enabled(false);

        let filename = utils::picture_file_name(format);
        let path = utils::pictures_dir()?.join(filename);

        imp.viewfinder.take_picture(path)?;
        imp.flash_bin.flash();

        let settings = imp.settings();
        if settings.boolean("play-shutter-sound") {
            self.play_shutter_sound();
        }

        Ok(())
    }

    pub fn diagnostic_switch_camera(&self) {
        if std::env::var("MIPAD2_DIAGNOSTIC_ACTIONS").as_deref() == Ok("1")
            && !self.is_recording_active()
        {
            self.camera_switched();
        }
    }

    fn camera_switched(&self) {
        let provider = self.imp().provider.get().unwrap();

        if provider.n_items() == 1 && is_mipad2() {
            let imp = self.imp();
            if imp.viewfinder.is_recording() {
                self.stop_recording();
            }

            // PipeWire owns /dev/video0 while the preview is active. Moving
            // camerabin to NULL releases that fd asynchronously, so retry the
            // V4L2 input switch for a short period instead of racing it.
            imp.viewfinder.stop_stream();

            let viewfinder = imp.viewfinder.clone();
            let obj = self.clone();
            let next_input = if imp.mipad2_input.get() == 0 { 1 } else { 0 };
            let af_generation = imp
                .mipad2_af_generation
                .fetch_add(1, Ordering::SeqCst)
                .wrapping_add(1);
            let mut attempts = 0u8;
            glib::timeout_add_local(std::time::Duration::from_millis(100), move || {
                attempts += 1;
                match mipad2_set_v4l2_input(next_input) {
                    Ok(()) => {
                        obj.imp().mipad2_input.set(next_input);

                        if next_input == 1 {
                            match mipad2_find_subdev("dw9719")
                                .and_then(|device| fs::File::open(device).map_err(Into::into))
                            {
                                Ok(vcm) => {
                                    // Keep the VCM subdev open for the whole rear-camera
                                    // session. dw9719 parks the lens back at 0 when its
                                    // last fd closes, which otherwise makes every focus
                                    // command effectively transient.
                                    obj.imp().mipad2_vcm.replace(Some(vcm));
                                    let af_range = mipad2_af_range();
                                    let default_focus = af_range.default_focus();
                                    if let Err(err) = mipad2_set_focus(default_focus) {
                                        log::warn!("Could not set Mi Pad 2 rear focus: {err}");
                                    } else {
                                        log::info!(
                                            "Set Mi Pad 2 rear focus to OTP-safe default {default_focus} before autofocus (range {}-{})",
                                            af_range.infinity,
                                            af_range.macro_pos
                                        );
                                    }
                                }
                                Err(err) => {
                                    log::warn!("Could not keep Mi Pad 2 VCM active: {err}");
                                }
                            }
                        } else {
                            obj.imp().mipad2_vcm.replace(None);
                        }

                        if let Err(err) = mipad2_set_color_balance(next_input) {
                            log::warn!(
                                "Could not set Mi Pad 2 camera {next_input} OTP white balance: {err}"
                            );
                        } else {
                            log::info!("Applied Mi Pad 2 camera {next_input} OTP white balance");
                        }

                        log::info!("Switched Mi Pad 2 V4L2 camera input to {next_input}");
                        viewfinder.set_front_camera(next_input == 0);
                        viewfinder.start_stream();

                        if next_input == 1 {
                            let cancel = obj.imp().mipad2_af_generation.clone();
                            let exposure_hold = viewfinder.hold_auto_exposure();
                            let sampler = viewfinder.preview_frame_sampler();
                            std::thread::spawn(move || {
                                let _exposure_hold = exposure_hold;
                                // Give the restarted PipeWire/AtomISP stream enough
                                // time to settle before sampling autofocus frames.
                                std::thread::sleep(Duration::from_millis(700));
                                if cancel.load(Ordering::SeqCst) == af_generation {
                                    mipad2_autofocus(sampler, cancel, af_generation);
                                }
                            });
                        }

                        glib::ControlFlow::Break
                    }
                    Err(err) if attempts < 10 => {
                        log::debug!(
                            "Mi Pad 2 camera input still busy (attempt {attempts}/10): {err}"
                        );
                        glib::ControlFlow::Continue
                    }
                    Err(err) => {
                        log::error!(
                            "Could not switch Mi Pad 2 camera input after {attempts} attempts: {err}"
                        );
                        viewfinder.start_stream();
                        glib::ControlFlow::Break
                    }
                }
            });
            return;
        }

        let current = self.imp().viewfinder.camera();
        let mut pos = 0;
        if current == provider.camera(0) {
            pos += 1;
        };
        if let Some(camera) = provider.camera(pos) {
            self.set_camera_inner(Some(camera));
        }
    }

    fn set_camera_inner(&self, camera: Option<aperture::Camera>) {
        let imp = self.imp();

        if let Some(ref camera) = camera {
            let id = id_from_pw(camera);
            imp.settings().set_string("last-camera-id", &id).unwrap();
        }

        if imp.viewfinder.is_recording() {
            self.stop_recording();
        }

        imp.viewfinder.set_camera(camera);
    }

    fn play_shutter_sound(&self) {
        // If we don't hold a reference to it there is a condition race which
        // will cause the sound to play only sometimes.
        let resource = "/org/gnome/Snapshot/sounds/camera-shutter.wav";
        let player = gtk::MediaFile::for_resource(resource);
        player.play();

        self.imp().players.replace(Some(player));
    }

    pub fn set_countdown(&self, countdown: u32) {
        self.imp().camera_controls.set_countdown(countdown);
    }

    pub fn start_countdown(&self) {
        self.imp().camera_controls.start_countdown();
    }

    pub fn stop_countdown(&self) {
        self.imp().camera_controls.stop_countdown();
    }

    pub fn shutter_mode(&self) -> crate::ShutterMode {
        self.imp().camera_controls.shutter_mode()
    }

    pub fn set_shutter_mode(&self, shutter_mode: crate::ShutterMode) {
        if matches!(shutter_mode, crate::ShutterMode::Picture) {
            self.stop_recording();
        }
        self.imp().camera_controls.set_shutter_mode(shutter_mode);
    }

    fn set_detect_codes(&self, detect_codes: bool) {
        let imp = self.imp();

        imp.viewfinder.set_detect_codes(detect_codes);
        imp.qr_screen_bin.set_enabled(detect_codes);

        let layout = if detect_codes {
            ControlsLayout::DetectingCodes
        } else {
            ControlsLayout::Default
        };
        imp.camera_controls.set_layout(layout);
    }

    pub fn set_gallery(&self, gallery: crate::Gallery) {
        let imp = self.imp();

        imp.viewfinder.connect_picture_done(glib::clone!(
            #[weak]
            gallery,
            #[weak(rename_to = obj)]
            self,
            move |_, file| {
                let window = obj.root().and_downcast::<crate::Window>().unwrap();
                window.set_shutter_enabled(true);
                // TODO Maybe report error via toast on None
                if let Some(file) = file {
                    gallery.add_image(file);
                }
            }
        ));
        imp.viewfinder.connect_recording_done(glib::clone!(
            #[weak]
            gallery,
            move |_, file| {
                if let Some(file) = file {
                    gallery.add_video(file);
                } else {
                    log::error!("Didn't find any file when recording finished!");
                }
            }
        ));
        imp.camera_controls.set_gallery(&gallery);
    }

    pub fn stop_stream(&self) {
        self.imp().viewfinder.stop_stream();
    }

    pub fn start_stream(&self) {
        self.imp().viewfinder.start_stream();
    }

    pub fn toggle_guidelines(&self) {
        let imp = self.imp();

        imp.guidelines
            .set_draw_guidelines(!imp.guidelines.draw_guidelines());
    }

    pub fn is_recording_active(&self) -> bool {
        self.imp().viewfinder.is_recording()
    }

    fn update_cameras_button(&self, provider: &aperture::DeviceProvider) {
        let imp = self.imp();

        let n_cameras = if provider.n_items() == 1 && is_mipad2() {
            2
        } else {
            provider.n_items()
        };
        imp.camera_controls.update_visible_camera_button(n_cameras);

        // We need to set the correct selected item at least when loading. The
        // default camera might not be the first one. A similar thing happens
        // when a camera is removed.
        let camera = imp.viewfinder.camera();
        if let Some(pos) = imp
            .selection
            // gtk::SingleSelection will Always returns glib::Object as its
            // gio::ListModel::item_type().
            .iter::<glib::Object>()
            .enumerate()
            .find(|(_pos, cam)| {
                cam.as_ref()
                    .is_ok_and(|c| c.downcast_ref::<aperture::Camera>() == camera.as_ref())
            })
            .map(|(pos, _cam)| pos)
        {
            imp.selection.set_selected(pos as u32);
        }
    }

    fn update_state(&self) {
        let imp = self.imp();

        if imp.permission_denied.get() {
            imp.stack.set_visible_child_name("permission-denied");
            return;
        }

        match imp.viewfinder.state() {
            aperture::ViewfinderState::Loading => {
                imp.stack.set_visible_child_name("loading");
            }
            aperture::ViewfinderState::Ready => {
                imp.stack.set_visible_child_name("camera");
            }
            aperture::ViewfinderState::NoCameras => imp.stack.set_visible_child_name("not-found"),
            aperture::ViewfinderState::Error => {
                imp.stack.set_visible_child_name("camera");

                let window = self.root().and_downcast::<crate::Window>().unwrap();
                window.send_toast(&gettext("Could not play camera stream"));
            }
        }
    }

    fn show_recording_label(&self) {
        let imp = self.imp();

        let source = glib::timeout_add_seconds_local(
            1,
            glib::clone!(
                #[weak(rename_to = obj)]
                self,
                #[upgrade_or]
                glib::ControlFlow::Break,
                move || {
                    let imp = obj.imp();

                    imp.recording_duration.update(|d| d + 1);
                    let duration = imp.recording_duration.get();

                    let minutes = duration.div_euclid(60);
                    let seconds = duration.rem_euclid(60);
                    imp.recording_label
                        .set_label(&format!("{minutes}∶{seconds:02}"));

                    glib::ControlFlow::Continue
                }
            ),
        );
        if let Some(old) = imp.recording_source.replace(Some(source)) {
            old.remove();
        }
        imp.recording_duration.set(0);
        imp.recording_revealer.set_reveal_child(true);
        imp.recording_label.set_label("0∶00");
    }

    fn hide_recording_label(&self) {
        let imp = self.imp();

        if let Some(source) = imp.recording_source.take() {
            source.remove();
            imp.recording_duration.set(0);
            imp.recording_label.set_label("0∶00");
            imp.recording_revealer.set_reveal_child(false);
        }
    }
}

async fn stream() -> anyhow::Result<OwnedFd> {
    let proxy = camera::Camera::new().await?;
    proxy
        .request_access(camera::CameraAccessOptions::default())
        .await
        .context("org.freedesktop.portal.Camera.AccessCamera failed")?;
    let is_present = proxy
        .is_present()
        .await
        .context("org.freedesktop.portal.Camera.IsCameraPresent failed")?;
    log::debug!("org.freedesktop.portal.Camera:IsCameraPresent: {is_present}");

    proxy
        .open_pipe_wire_remote(camera::OpenPipeWireRemoteOptions::default())
        .await
        .context("org.freedesktop.portal.Camera.OpenPipeWireRemote")
}

// Id used to identify the last-used camera.
fn id_from_pw(camera: &aperture::Camera) -> glib::GString {
    camera.display_name()
}
