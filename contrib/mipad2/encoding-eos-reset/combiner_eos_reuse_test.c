/* SPDX-License-Identifier: LGPL-2.1-or-later */
/* Real upstream combiner, two recording epochs, delayed active-branch output. */
#include <gst/gst.h>
#include "gststreamcombiner.h"

static guint eos_count, buffer_count;
static GstPadProbeReturn sink_probe (GstPad *pad, GstPadProbeInfo *info, gpointer data) {
  (void) pad; (void) data;
  if (GST_PAD_PROBE_INFO_TYPE (info) & GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM) {
    GstEvent *event = GST_PAD_PROBE_INFO_EVENT (info);
    if (GST_EVENT_TYPE (event) == GST_EVENT_EOS) eos_count++;
  }
  if (GST_PAD_PROBE_INFO_TYPE (info) & GST_PAD_PROBE_TYPE_BUFFER) buffer_count++;
  return GST_PAD_PROBE_OK;
}

static void start_stream (GstPad *pad, const char *id) {
  GstSegment segment;
  GstCaps *caps = gst_caps_new_empty_simple ("application/octet-stream");
  gst_pad_push_event (pad, gst_event_new_stream_start (id));
  gst_pad_push_event (pad, gst_event_new_caps (caps));
  gst_caps_unref (caps);
  gst_segment_init (&segment, GST_FORMAT_TIME);
  gst_pad_push_event (pad, gst_event_new_segment (&segment));
}

int main (int argc, char **argv) {
  GstElement *pipeline, *combiner, *sink;
  GstPad *pass, *enc, *pass_sink, *enc_sink, *sinkpad;
  gboolean success = TRUE;
  gst_init (&argc, &argv);
  pipeline = gst_pipeline_new (NULL);
  combiner = g_object_new (gst_stream_combiner_get_type (), NULL);
  sink = gst_element_factory_make ("fakesink", NULL);
  if (!sink) return 2;
  g_object_set (sink, "sync", FALSE, "async", FALSE, NULL);
  gst_bin_add_many (GST_BIN (pipeline), combiner, sink, NULL);
  if (!gst_element_link (combiner, sink)) return 3;
  pass_sink = gst_element_request_pad_simple (combiner, "sink_0");
  enc_sink = gst_element_request_pad_simple (combiner, "sink_1");
  pass = gst_pad_new ("pass", GST_PAD_SRC); enc = gst_pad_new ("enc", GST_PAD_SRC);
  if (!pass_sink || !enc_sink || gst_pad_link (pass, pass_sink) != GST_PAD_LINK_OK ||
      gst_pad_link (enc, enc_sink) != GST_PAD_LINK_OK) return 4;
  sinkpad = gst_element_get_static_pad (sink, "sink");
  gst_pad_add_probe (sinkpad, GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM | GST_PAD_PROBE_TYPE_BUFFER,
      sink_probe, NULL, NULL);
  for (guint round = 0; round < 64; round++) {
    guint before = eos_count, after_inactive, before_active, after_active;
    GstFlowReturn flow;
    GstBuffer *buffer;
    gst_pad_set_active (pass, FALSE); gst_pad_set_active (enc, FALSE);
    gst_element_set_state (pipeline, GST_STATE_READY);
    gst_element_set_state (pipeline, GST_STATE_PLAYING);
    gst_pad_set_active (pass, TRUE); gst_pad_set_active (enc, TRUE);
    start_stream (pass, "pass"); start_stream (enc, "enc");
    gst_pad_push_event (pass, gst_event_new_eos ());
    after_inactive = eos_count;
    buffer = gst_buffer_new_allocate (NULL, 1, NULL);
    GST_BUFFER_PTS (buffer) = 0; GST_BUFFER_DURATION (buffer) = GST_SECOND / 30;
    flow = gst_pad_push (enc, buffer);
    before_active = eos_count;
    gst_pad_push_event (enc, gst_event_new_eos ());
    after_active = eos_count;
    g_print ("round=%u before=%u after-inactive=%u before-active=%u after-active=%u flow=%s buffers=%u\n",
        round, before, after_inactive, before_active, after_active, gst_flow_get_name (flow), buffer_count);
    if (after_inactive != before || before_active != before || after_active != before + 1 ||
        flow != GST_FLOW_OK || buffer_count != round + 1) success = FALSE;
  }
  gst_pad_set_active (pass, FALSE); gst_pad_set_active (enc, FALSE);
  gst_element_set_state (pipeline, GST_STATE_NULL);
  gst_pad_unlink (pass, pass_sink); gst_pad_unlink (enc, enc_sink);
  gst_element_release_request_pad (combiner, pass_sink); gst_element_release_request_pad (combiner, enc_sink);
  gst_object_unref (pass_sink); gst_object_unref (enc_sink);
  gst_object_unref (pass); gst_object_unref (enc); gst_object_unref (sinkpad); gst_object_unref (pipeline);
  return success ? 0 : 1;
}
