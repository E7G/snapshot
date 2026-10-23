#include <gst/gst.h>
#include <stdio.h>
#include <stdint.h>
#include <time.h>

#define CAPACITY 4096
typedef struct { guint64 arrival, pts, dts, duration, offset, end; guint flags; } Row;
typedef struct { Row rows[CAPACITY]; gint count; } Trace;
static Trace traces[3];

static GstPadProbeReturn record_buffer(GstPad *pad, GstPadProbeInfo *info, gpointer data)
{
  Trace *trace = data;
  GstBuffer *buffer = GST_PAD_PROBE_INFO_BUFFER(info);
  struct timespec now;
  gint i;
  (void)pad;
  if (!buffer) return GST_PAD_PROBE_OK;
  i = g_atomic_int_add(&trace->count, 1);
  if (i >= CAPACITY) return GST_PAD_PROBE_OK;
  clock_gettime(CLOCK_MONOTONIC, &now);
  trace->rows[i] = (Row){ .arrival = (guint64)now.tv_sec * 1000000000 + now.tv_nsec,
    .pts = GST_BUFFER_PTS(buffer), .dts = GST_BUFFER_DTS(buffer),
    .duration = GST_BUFFER_DURATION(buffer), .offset = GST_BUFFER_OFFSET(buffer),
    .end = GST_BUFFER_OFFSET_END(buffer), .flags = GST_BUFFER_FLAGS(buffer) };
  return GST_PAD_PROBE_OK;
}

int main(int argc, char **argv)
{
  GstElement *pipeline, *elements[3];
  GstBus *bus;
  GstMessage *message;
  GError *error = NULL;
  int result = 0;
  FILE *output;
  const char *names[] = {"capture", "encoder", "parser"};
  if (argc != 3) return 2;
  gst_init(&argc, &argv);
  pipeline = gst_parse_launch(argv[1], &error);
  if (error || !pipeline) { if (error) g_printerr("parse: %s\n", error->message); return 3; }
  for (int i = 0; i < 3; i++) {
    GstPad *pad;
    elements[i] = gst_bin_get_by_name(GST_BIN(pipeline), names[i]);
    if (!elements[i]) return 4;
    pad = gst_element_get_static_pad(elements[i], "src");
    if (!pad || !gst_pad_add_probe(pad, GST_PAD_PROBE_TYPE_BUFFER, record_buffer, &traces[i], NULL)) return 5;
    gst_object_unref(pad);
  }
  bus = gst_element_get_bus(pipeline);
  if (gst_element_set_state(pipeline, GST_STATE_PLAYING) == GST_STATE_CHANGE_FAILURE) result = 6;
  message = result ? NULL : gst_bus_timed_pop_filtered(bus, 85 * GST_SECOND, GST_MESSAGE_ERROR | GST_MESSAGE_EOS);
  if (!message) result = 7;
  else if (GST_MESSAGE_TYPE(message) == GST_MESSAGE_ERROR) {
    gchar *debug = NULL;
    gst_message_parse_error(message, &error, &debug);
    g_printerr("pipeline: %s\n", error->message);
    g_free(debug); g_clear_error(&error); result = 8;
  }
  gst_element_set_state(pipeline, GST_STATE_NULL);
  gst_element_get_state(pipeline, NULL, NULL, 10 * GST_SECOND);
  output = fopen(argv[2], "wx");
  if (!output) return 9;
  fprintf(output, "site,index,arrival_ns,pts_ns,dts_ns,duration_ns,offset,offset_end,flags\n");
  for (int site = 0; site < 3; site++) {
    int count = g_atomic_int_get(&traces[site].count);
    if (count > CAPACITY) result = 10;
    for (int i = 0; i < MIN(count, CAPACITY); i++) {
      Row *r = &traces[site].rows[i];
      if (fprintf(output, "%d,%d,%" G_GUINT64_FORMAT ",%" G_GUINT64_FORMAT ",%" G_GUINT64_FORMAT ",%" G_GUINT64_FORMAT ",%" G_GUINT64_FORMAT ",%" G_GUINT64_FORMAT ",%u\n",
          site,i,r->arrival,r->pts,r->dts,r->duration,r->offset,r->end,r->flags) < 0) result = 11;
    }
  }
  if (fclose(output)) result = 12;
  for (int i = 0; i < 3; i++) gst_object_unref(elements[i]);
  if (message) gst_message_unref(message);
  gst_object_unref(bus); gst_object_unref(pipeline);
  return result;
}
