// SPDX-License-Identifier: GPL-3.0-or-later
use std::collections::HashMap;

use gst::prelude::*;
use gst::subclass::prelude::*;
use gtk::glib;

mod imp {
    use std::sync::{Mutex, OnceLock};

    use super::*;

    #[derive(Debug, Default)]
    pub struct PipelineTee {
        pub hashmap: Mutex<HashMap<gst::Element, gst::Element>>,
        pub tee: OnceLock<gst::Element>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for PipelineTee {
        const NAME: &'static str = "AperturePipelineTee";
        type Type = super::PipelineTee;
        type ParentType = gst::Bin;
    }

    impl ObjectImpl for PipelineTee {
        fn constructed(&self) {
            self.parent_constructed();

            let obj = self.obj();

            let tee = gst::ElementFactory::make("tee").build().unwrap();
            obj.add(&tee).unwrap();

            let pad = tee.static_pad("sink").unwrap();
            let ghost_pad = gst::GhostPad::with_target(&pad).unwrap();
            ghost_pad.set_active(true).unwrap();

            obj.add_pad(&ghost_pad).unwrap();

            self.tee.set(tee).unwrap();
        }
    }

    impl GstObjectImpl for PipelineTee {}
    impl ElementImpl for PipelineTee {}
    impl BinImpl for PipelineTee {}
}

glib::wrapper! {
    pub struct PipelineTee(ObjectSubclass<imp::PipelineTee>)
        @extends gst::Bin, gst::Element, gst::Object;
}

impl Default for PipelineTee {
    fn default() -> Self {
        glib::Object::new()
    }
}

impl PipelineTee {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_branch(&self, branch: &gst::Element) {
        let imp = self.imp();

        let queue = gst::ElementFactory::make("queue").build().unwrap();
        // This tee belongs only to the viewfinder/optional QR branches, never
        // the recording branch. A late display must not retain capture buffers
        // indefinitely and backpressure the sensor or encoder.
        queue.set_property_from_str("leaky", "downstream");
        queue.set_property("max-size-buffers", 2u32);
        queue.set_property("max-size-bytes", 0u32);
        queue.set_property("max-size-time", 0u64);

        imp.hashmap
            .lock()
            .unwrap()
            .insert(branch.clone(), queue.clone());

        self.add_many([&queue, branch]).unwrap();
        queue.link(branch).unwrap();

        let tee_pad = imp.tee.get().unwrap().request_pad_simple("src_%u").unwrap();
        let queue_pad = queue.static_pad("sink").unwrap();

        tee_pad.link(&queue_pad).unwrap();

        queue.sync_state_with_parent().unwrap();
        branch.sync_state_with_parent().unwrap();
    }

    pub fn remove_branch(&self, branch: &gst::Element) {
        let imp = self.imp();
        if let Some(queue) = imp.hashmap.lock().unwrap().remove(branch) {
            let queue_pad = queue.static_pad("sink").unwrap();
            let tee_pad = queue_pad.peer().unwrap();

            tee_pad.add_probe(
                gst::PadProbeType::BLOCK_DOWNSTREAM,
                glib::clone!(
                    #[weak(rename_to = obj)]
                    self,
                    #[weak]
                    branch,
                    #[weak]
                    queue,
                    #[upgrade_or]
                    gst::PadProbeReturn::Remove,
                    move |tee_pad, _| {
                        let tee = obj.imp().tee.get().unwrap();
                        tee.call_async(glib::clone!(
                            #[weak]
                            obj,
                            #[weak]
                            tee_pad,
                            #[weak]
                            branch,
                            #[weak]
                            queue,
                            move |tee| {
                                tee.release_request_pad(&tee_pad);

                                branch.set_state(gst::State::Null).unwrap();
                                queue.set_state(gst::State::Null).unwrap();

                                obj.remove(&queue).unwrap();
                                obj.remove(&branch).unwrap();
                            }
                        ));

                        gst::PadProbeReturn::Remove
                    }
                ),
            );
        } else {
            log::error!("Branch {branch:?} not in the pipeline");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };
    use std::time::Instant;

    #[test]
    fn optional_preview_queue_is_bounded_and_nonblocking() {
        gst::init().unwrap();
        let preview = PipelineTee::new();
        let sink = gst::ElementFactory::make("fakesink").build().unwrap();
        preview.add_branch(&sink);
        let queue = preview
            .imp()
            .hashmap
            .lock()
            .unwrap()
            .get(&sink)
            .unwrap()
            .clone();
        assert_eq!(queue.property::<u32>("max-size-buffers"), 2);
        assert_eq!(queue.property::<u32>("max-size-bytes"), 0);
        assert_eq!(queue.property::<u64>("max-size-time"), 0);
        assert_eq!(
            queue
                .property_value("leaky")
                .transform::<i32>()
                .unwrap()
                .get::<i32>()
                .unwrap(),
            2
        );
    }

    #[test]
    fn slow_preview_does_not_block_recording() {
        gst::init().unwrap();
        let pipeline = gst::Pipeline::new();
        let source = gst::ElementFactory::make("videotestsrc")
            .property("is-live", true)
            .property("num-buffers", 90i32)
            .build()
            .unwrap();
        source.set_property_from_str("pattern", "black");
        let capsfilter = gst::ElementFactory::make("capsfilter")
            .property(
                "caps",
                gst::Caps::builder("video/x-raw")
                    .field("format", "NV12")
                    .field("width", 1920i32)
                    .field("height", 1440i32)
                    .field("framerate", gst::Fraction::new(30, 1))
                    .build(),
            )
            .build()
            .unwrap();
        let tee = gst::ElementFactory::make("tee").build().unwrap();
        let recording_queue = gst::ElementFactory::make("queue").build().unwrap();
        let recording = gst::ElementFactory::make("fakesink")
            .property("sync", false)
            .property("async", false)
            .property("signal-handoffs", true)
            .build()
            .unwrap();
        let count = Arc::new(AtomicUsize::new(0));
        let copy = count.clone();
        recording.connect("handoff", false, move |_| {
            copy.fetch_add(1, Ordering::SeqCst);
            None
        });
        let slow = gst::parse::bin_from_description(
            "identity sleep-time=100000 ! fakesink sync=false async=false",
            true,
        )
        .unwrap();
        let preview = PipelineTee::new();
        preview.add_branch(slow.upcast_ref());
        pipeline
            .add_many([
                &source,
                &capsfilter,
                &tee,
                &recording_queue,
                &recording,
                preview.upcast_ref(),
            ])
            .unwrap();
        gst::Element::link_many([&source, &capsfilter, &tee]).unwrap();
        gst::Element::link_many([&tee, &recording_queue, &recording]).unwrap();
        tee.link(&preview).unwrap();
        let start = Instant::now();
        pipeline.set_state(gst::State::Playing).unwrap();
        let message = pipeline.bus().unwrap().timed_pop_filtered(
            gst::ClockTime::from_seconds(20),
            &[gst::MessageType::Eos, gst::MessageType::Error],
        );
        let seconds = start.elapsed().as_secs_f64();
        pipeline.set_state(gst::State::Null).unwrap();
        assert!(
            matches!(
                message.as_ref().map(|m| m.type_()),
                Some(gst::MessageType::Eos)
            ),
            "{message:?}"
        );
        assert_eq!(
            count.load(Ordering::SeqCst),
            90,
            "Encoding branch must retain every source frame"
        );
        eprintln!(
            "actual PipelineTee slow-display test: 90 fullres source frames, all90 recording handoffs, elapsed={seconds:.3}s"
        );
        assert!(
            seconds < 4.5,
            "Slow preview must not backpressure recording; elapsed={seconds:.3}s"
        );
    }
}
