// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in scalar-only timing diagnostics. No buffer maps or camera writes.
use gst::prelude::*;
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

static SEEN: LazyLock<Mutex<Vec<gtk::glib::WeakRef<gst::Pad>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
struct State {
    since: Instant,
    frames: u64,
    total: u64,
    qos: u64,
    pts: Option<u64>,
    offset: Option<u64>,
    max_pts_gap: u64,
    max_offset_gap: u64,
}
pub(crate) fn pad(pad: &gst::Pad, label: &str) {
    if std::env::var("MIPAD2_TRACE_CAPTURE").as_deref() != Ok("1") {
        return;
    }
    let mut seen = SEEN.lock().unwrap();
    seen.retain(|p| p.upgrade().is_some());
    if seen.iter().any(|p| p.upgrade().as_ref() == Some(pad)) {
        return;
    }
    seen.push(pad.downgrade());
    drop(seen);
    let label = label.to_string();
    let state = Arc::new(Mutex::new(State {
        since: Instant::now(),
        frames: 0,
        total: 0,
        qos: 0,
        pts: None,
        offset: None,
        max_pts_gap: 0,
        max_offset_gap: 0,
    }));
    pad.add_probe(gst::PadProbeType::BUFFER | gst::PadProbeType::EVENT_UPSTREAM, move |pad, info| {
        let mut state = state.lock().unwrap();
        if let Some(event) = info.event() {
            if let gst::EventView::Qos(qos) = event.view() {
                state.qos += 1;
                if state.qos <= 3 || state.qos % 30 == 0 {
                    log::info!("Mi Pad 2 trace {} upstream-QOS count={} values={:?}", label, state.qos, qos.get());
                }
            }
        }
        if let Some(buffer) = info.buffer() {
            state.frames += 1; state.total += 1;
            if let Some(pts) = buffer.pts().map(|p| p.nseconds()) {
                if let Some(last) = state.pts {if pts > last {state.max_pts_gap = state.max_pts_gap.max(pts-last);}}
                state.pts = Some(pts);
            }
            let offset = buffer.offset();
            if offset != u64::MAX {
                if let Some(last) = state.offset {if offset > last {state.max_offset_gap = state.max_offset_gap.max(offset-last);}}
                state.offset = Some(offset);
            }
        }
        let elapsed = state.since.elapsed().as_secs_f64();
        if elapsed >= 1.0 && state.frames > 0 {
            log::info!("Mi Pad 2 trace {} frames={} total={} wall={:.3} fps={:.3} max-pts-gap-ns={} max-offset-gap={} qos={} caps={}",
                label, state.frames, state.total, elapsed, state.frames as f64 / elapsed,
                state.max_pts_gap, state.max_offset_gap, state.qos,
                pad.current_caps().map(|c| c.to_string()).unwrap_or_default());
            state.frames = 0; state.since = Instant::now();
        }
        gst::PadProbeReturn::Ok
    });
}
