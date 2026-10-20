/* SPDX-License-Identifier: LGPL-2.1-or-later */
#include <gst/gst.h>
static gboolean slow;
static GstPadProbeReturn probe(GstPad *pad,GstPadProbeInfo *info,gpointer data) {
  (void)pad;(void)data;
  if (!slow) return GST_PAD_PROBE_OK;
  if ((GST_PAD_PROBE_INFO_TYPE(info)&GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM) &&
      GST_EVENT_TYPE(GST_PAD_PROBE_INFO_EVENT(info))==GST_EVENT_SEGMENT) g_usleep(60000);
  if ((GST_PAD_PROBE_INFO_TYPE(info)&GST_PAD_PROBE_TYPE_QUERY_DOWNSTREAM) &&
      GST_QUERY_TYPE(GST_PAD_PROBE_INFO_QUERY(info))==GST_QUERY_ALLOCATION) g_usleep(60000);
  return GST_PAD_PROBE_OK;
}
int main(int argc,char **argv) {
  gst_init(&argc,&argv);slow=g_strcmp0(g_getenv("MP2_CONTROL_TEST_SLOW"),"1")==0;
  GError *error=NULL;
  GstElement *pipeline=gst_parse_launch("videotestsrc is-live=true num-buffers=12 ! capsfilter name=unrelated caps=video/x-raw,format=NV12,width=8,height=6,framerate=30/1 ! capsfilter name=mipad2-native-raw-output ! identity name=measured ! fakesink sync=false",&error);
  g_assert_no_error(error);g_assert_nonnull(pipeline);
  GstElement *identity=gst_bin_get_by_name(GST_BIN(pipeline),"measured");GstPad *sink=gst_element_get_static_pad(identity,"sink");
  gst_pad_add_probe(sink,GST_PAD_PROBE_TYPE_EVENT_DOWNSTREAM|GST_PAD_PROBE_TYPE_QUERY_DOWNSTREAM,probe,NULL,NULL);
  g_assert_cmpint(gst_element_set_state(pipeline,GST_STATE_PLAYING),!=,GST_STATE_CHANGE_FAILURE);
  g_assert_cmpint(gst_element_get_state(pipeline,NULL,NULL,3*GST_SECOND),!=,GST_STATE_CHANGE_FAILURE);
  GstQuery *query=gst_query_new_allocation(NULL,FALSE);gst_pad_query(sink,query);gst_query_unref(query);
  GstBus *bus=gst_element_get_bus(pipeline);GstMessage *message=gst_bus_timed_pop_filtered(bus,5*GST_SECOND,GST_MESSAGE_EOS|GST_MESSAGE_ERROR);
  g_assert_nonnull(message);g_assert_cmpint(GST_MESSAGE_TYPE(message),==,GST_MESSAGE_EOS);gst_message_unref(message);
  gst_element_set_state(pipeline,GST_STATE_NULL);gst_element_get_state(pipeline,NULL,NULL,3*GST_SECOND);
  gst_object_unref(sink);gst_object_unref(identity);gst_object_unref(bus);gst_object_unref(pipeline);return 0;
}
