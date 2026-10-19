/* SPDX-License-Identifier: LGPL-2.1-or-later */
/* Small host-only EOS/error runner; no multimedia command-line tools required. */
#include <gst/gst.h>
int main (int argc, char **argv) {
  GError *error = NULL;
  GstElement *pipeline;
  GstBus *bus;
  GstMessage *message;
  gboolean success;
  gst_init (&argc, &argv);
  if (argc != 2) return 2;
  pipeline = gst_parse_launch (argv[1], &error);
  if (!pipeline || error) {
    g_printerr ("parse failed: %s\n", error ? error->message : "null pipeline");
    if (error) g_error_free (error);
    if (pipeline) gst_object_unref (pipeline);
    return 3;
  }
  bus = gst_element_get_bus (pipeline);
  if (gst_element_set_state (pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE)
    message = NULL;
  else
    message = gst_bus_timed_pop_filtered (bus, 20 * GST_SECOND,
        GST_MESSAGE_EOS | GST_MESSAGE_ERROR);
  success = message && GST_MESSAGE_TYPE (message) == GST_MESSAGE_EOS;
  if (message && GST_MESSAGE_TYPE (message) == GST_MESSAGE_ERROR) {
    gst_message_parse_error (message, &error, NULL);
    g_printerr ("pipeline error: %s\n", error->message);
    g_error_free (error);
  }
  gst_element_set_state (pipeline, GST_STATE_NULL);
  if (message) gst_message_unref (message);
  gst_object_unref (bus);
  gst_object_unref (pipeline);
  return success ? 0 : 4;
}
