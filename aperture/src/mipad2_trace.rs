// SPDX-License-Identifier: GPL-3.0-or-later
//! Opt-in timing diagnostics; separate opt-in active-pixel fingerprints. No writes or retained buffers.
use crate::mipad2_trace_content::{self as content, Digest, History, Match};
use gst::prelude::*;
use gst_video::prelude::*;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::Instant;

static SEEN: LazyLock<Mutex<Vec<gtk::glib::WeakRef<gst::Pad>>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));
static HISTORY: LazyLock<Mutex<History>> = LazyLock::new(|| Mutex::new(History::new(256)));
static NEXT_OWNER: AtomicU64 = AtomicU64::new(1);

#[derive(Default)]
struct ContentState {
    epoch: u64,
    seen: u64,
    valid: u64,
    repeated: u64,
    last: Option<Digest>,
    matched: u64,
    ambiguous: u64,
    missing: u64,
    reused: u64,
    backwards: u64,
    skipped: u64,
    pts_mismatch: u64,
    stale_owner: u64,
    last_native: Option<u64>,
    native_epoch: u64,
    cost_us: u128,
    max_cost_us: u128,
}

fn fingerprint_buffer(buffer: &gst::BufferRef, info: &gst_video::VideoInfo) -> Option<Digest> {
    if info.format() != gst_video::VideoFormat::Nv12 {
        return None;
    }
    let frame = gst_video::VideoFrameRef::from_buffer_ref_readable(buffer, info).ok()?;
    content::fingerprint(
        info.width() as usize,
        info.height() as usize,
        frame.plane_data(0).ok()?,
        usize::try_from(frame.plane_stride()[0]).ok()?,
        frame.plane_data(1).ok()?,
        usize::try_from(frame.plane_stride()[1]).ok()?,
    )
}

impl ContentState {
    fn report(&self, label: &str) {
        log::info!(
            "Mi Pad 2 content {} epoch={} native-epoch={} seen={} valid={} repeated={} map-failed={} native-match={} ambiguous={} missing={} reused={} backwards={} skipped={} pts-mismatch={} stale-owner={} cost-us={} max-cost-us={}",
            label,
            self.epoch,
            self.native_epoch,
            self.seen,
            self.valid,
            self.repeated,
            self.seen - self.valid,
            self.matched,
            self.ambiguous,
            self.missing,
            self.reused,
            self.backwards,
            self.skipped,
            self.pts_mismatch,
            self.stale_owner,
            self.cost_us,
            self.max_cost_us
        );
    }
}

struct State {
    since: Instant,
    frames: u64,
    total: u64,
    qos: u64,
    pts: Option<u64>,
    offset: Option<u64>,
    max_pts_gap: u64,
    max_offset_gap: u64,
    content: ContentState,
    video_info: Option<gst_video::VideoInfo>,
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
    let content_enabled = content::enabled(
        true,
        std::env::var("MIPAD2_TRACE_CONTENT").as_deref() == Ok("1"),
        label,
    );
    let owner = if content_enabled && label == "native-source" {
        let owner = NEXT_OWNER.fetch_add(1, Ordering::Relaxed);
        HISTORY.lock().unwrap().begin(owner);
        owner
    } else {
        0
    };
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
        content: ContentState::default(),
        video_info: None,
    }));
    pad.add_probe(gst::PadProbeType::BUFFER | gst::PadProbeType::EVENT_UPSTREAM | gst::PadProbeType::EVENT_DOWNSTREAM, move |pad, info| {
        let mut state = state.lock().unwrap();
        if let Some(event) = info.event() {
            if content_enabled {
                match event.view() {
                    gst::EventView::Caps(caps) => {
                        state.video_info = gst_video::VideoInfo::from_caps(caps.caps()).ok();
                    }
                    gst::EventView::Segment(_) | gst::EventView::FlushStart(_) => {
                        if state.content.seen > 0 { state.content.report(&label); }
                        let epoch=state.content.epoch+1;
                        state.content=ContentState { epoch, ..ContentState::default() };
                        if owner!=0 { HISTORY.lock().unwrap().begin(owner); }
                    }
                    gst::EventView::Eos(_) => state.content.report(&label),
                    _ => {}
                }
            }
            if let gst::EventView::Qos(qos) = event.view() {
                state.qos += 1;
                if state.qos <= 3 || state.qos % 30 == 0 {
                    log::info!("Mi Pad 2 trace {} upstream-QOS count={} values={:?}", label, state.qos, qos.get());
                }
            }
        }
        if let Some(buffer) = info.buffer() {
            if content_enabled {
                let started=Instant::now();
                state.content.seen+=1;
                // Caps events normally supply this; handle probes installed after CAPS too.
                if state.video_info.is_none() {
                    state.video_info=pad.current_caps().and_then(|caps| gst_video::VideoInfo::from_caps(&caps).ok());
                }
                let digest=state.video_info.as_ref().and_then(|vi| fingerprint_buffer(buffer,vi));
                if let Some(digest)=digest {
                    let pts=buffer.pts().map(|p|p.nseconds());
                    let mut history=HISTORY.lock().unwrap();
                    let c=&mut state.content;
                    c.valid+=1;
                    if c.last==Some(digest) { c.repeated+=1; }
                    c.last=Some(digest);
                    if owner!=0 {
                        if !history.observe(owner,pts,digest) { c.stale_owner+=1; }
                    } else {
                        if c.native_epoch!=history.epoch { c.last_native=None; }
                        match history.resolve(pts,digest) {
                            Match::Unique(sample) => {
                                c.matched+=1;
                                if let Some(last)=c.last_native {
                                    if sample.id==last { c.reused+=1; }
                                    else if sample.id<last { c.backwards+=1; }
                                    else { c.skipped+=sample.id-last-1; }
                                }
                                c.last_native=Some(sample.id);
                                if pts!=sample.pts { c.pts_mismatch+=1; }
                            }
                            Match::Ambiguous => c.ambiguous+=1,
                            Match::Missing => c.missing+=1,
                        }
                    }
                    c.native_epoch=history.epoch;
                }
                let elapsed=started.elapsed().as_micros();
                state.content.cost_us+=elapsed;
                state.content.max_cost_us=state.content.max_cost_us.max(elapsed);
            }
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
            if content_enabled { state.content.report(&label); }
            state.frames = 0; state.since = Instant::now();
        }
        gst::PadProbeReturn::Ok
    });
}
