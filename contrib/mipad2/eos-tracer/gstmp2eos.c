/* SPDX-License-Identifier: LGPL-2.1-or-later */
/* Read-only scalar flow/event diagnostics. Never maps or retains a buffer. */
#include <gst/gst.h>
#include <gst/gsttracer.h>
#ifndef PACKAGE
#define PACKAGE "mipad2-eos-tracer"
#endif

typedef struct { GstTracer parent; } Mp2EosTracer;
typedef struct { GstTracerClass parent; } Mp2EosTracerClass;
typedef struct {
  GMutex lock;
  guint64 epoch, seen, ok;
  GstClockTime first, last;
} PadState;

G_DEFINE_TYPE (Mp2EosTracer, mp2_eos_tracer, GST_TYPE_TRACER)
static GQuark state_key;
G_LOCK_DEFINE_STATIC (state_creation);
static gint sequence, log_count;

static void
free_state (gpointer data)
{
  PadState *s = data;
  g_mutex_clear (&s->lock);
  g_free (s);
}

static PadState *
get_state (GstPad *pad)
{
  PadState *s;
  G_LOCK (state_creation);
  s = g_object_get_qdata (G_OBJECT (pad), state_key);
  if (!s) {
    s = g_new0 (PadState, 1);
    g_mutex_init (&s->lock);
    s->first = s->last = GST_CLOCK_TIME_NONE;
    g_object_set_qdata_full (G_OBJECT (pad), state_key, s, free_state);
  }
  G_UNLOCK (state_creation);
  return s;
}

static void
log_pad (GstClockTime ts, GstPad *pad, const gchar *op,
    const gchar *detail, guint event_seq)
{
  PadState *s = get_state (pad);
  guint64 epoch, seen, ok;
  GstClockTime first, last;
  GstPad *peer;
  gchar *path, *peer_path;
  gint seq;
  if (g_atomic_int_add (&log_count, 1) >= 2000)
    return;
  g_mutex_lock (&s->lock);
  epoch = s->epoch; seen = s->seen; ok = s->ok;
  first = s->first; last = s->last;
  g_mutex_unlock (&s->lock);
  path = gst_object_get_path_string (GST_OBJECT (pad));
  peer = gst_pad_get_peer (pad);
  peer_path = peer ? gst_object_get_path_string (GST_OBJECT (peer)) : g_strdup ("none");
  seq = g_atomic_int_add (&sequence, 1);
  g_printerr ("MIPAD2_EOS seq=%d ts=%" G_GUINT64_FORMAT
      " op=%s detail=%s event-seq=%u pad=%s peer=%s epoch=%" G_GUINT64_FORMAT
      " seen=%" G_GUINT64_FORMAT " ok=%" G_GUINT64_FORMAT
      " first=%" G_GUINT64_FORMAT " last=%" G_GUINT64_FORMAT "\n",
      seq, ts, op, detail, event_seq, path, peer_path, epoch, seen, ok, first, last);
  if (peer) gst_object_unref (peer);
  g_free (peer_path); g_free (path);
}

static void
buffer_pre (GObject *self, GstClockTime ts, GstPad *pad, GstBuffer *buffer)
{
  PadState *s = get_state (pad);
  (void) self; (void) ts;
  g_mutex_lock (&s->lock);
  if (!s->seen) s->first = GST_BUFFER_PTS (buffer);
  s->seen++;
  s->last = GST_BUFFER_PTS (buffer);
  g_mutex_unlock (&s->lock);
}

static void
push_post (GObject *self, GstClockTime ts, GstPad *pad, GstFlowReturn result)
{
  PadState *s = get_state (pad);
  (void) self;
  if (result == GST_FLOW_OK) {
    g_mutex_lock (&s->lock); s->ok++; g_mutex_unlock (&s->lock);
  } else {
    log_pad (ts, pad, "push-return", gst_flow_get_name (result), 0);
  }
}

static void
chain_post (GObject *self, GstClockTime ts, GstPad *pad, GstFlowReturn result)
{
  (void) self;
  if (result != GST_FLOW_OK)
    log_pad (ts, pad, "chain-return", gst_flow_get_name (result), 0);
}

static void
event_pre (GObject *self, GstClockTime ts, GstPad *pad, GstEvent *event)
{
  GstEventType type = GST_EVENT_TYPE (event);
  (void) self;
  if (type == GST_EVENT_SEGMENT) {
    const GstSegment *seg;
    gchar *detail;
    PadState *s = get_state (pad);
    gst_event_parse_segment (event, &seg);
    g_mutex_lock (&s->lock);
    s->epoch++; s->seen = s->ok = 0;
    s->first = s->last = GST_CLOCK_TIME_NONE;
    g_mutex_unlock (&s->lock);
    detail = g_strdup_printf ("segment:format=%s,start=%" G_GUINT64_FORMAT
        ",stop=%" G_GUINT64_FORMAT ",base=%" G_GUINT64_FORMAT,
        gst_format_get_name (seg->format), seg->start, seg->stop, seg->base);
    log_pad (ts, pad, "event", detail, gst_event_get_seqnum (event));
    g_free (detail);
  } else if (type == GST_EVENT_EOS || type == GST_EVENT_FLUSH_START ||
      type == GST_EVENT_FLUSH_STOP || type == GST_EVENT_STREAM_START) {
    log_pad (ts, pad, "event", GST_EVENT_TYPE_NAME (event), gst_event_get_seqnum (event));
  }
}

static void
mp2_eos_tracer_init (Mp2EosTracer *self)
{
  if (!state_key) state_key = g_quark_from_static_string ("mipad2-eos:pad-state");
  gst_tracing_register_hook (GST_TRACER (self), "pad-push-pre", G_CALLBACK (buffer_pre));
  gst_tracing_register_hook (GST_TRACER (self), "pad-push-post", G_CALLBACK (push_post));
  gst_tracing_register_hook (GST_TRACER (self), "pad-chain-post", G_CALLBACK (chain_post));
  gst_tracing_register_hook (GST_TRACER (self), "pad-push-event-pre", G_CALLBACK (event_pre));
  gst_tracing_register_hook (GST_TRACER (self), "pad-send-event-pre", G_CALLBACK (event_pre));
  g_printerr ("MIPAD2_EOS enabled version=1 scalar-only=1 log-limit=2000\n");
}

static void mp2_eos_tracer_class_init (Mp2EosTracerClass *klass) { (void) klass; }
static gboolean plugin_init (GstPlugin *plugin) {
  return gst_tracer_register (plugin, "mipad2-eos", mp2_eos_tracer_get_type ());
}
GST_PLUGIN_DEFINE (GST_VERSION_MAJOR, GST_VERSION_MINOR, mp2eos,
    "Mi Pad 2 bounded scalar EOS flow tracing", plugin_init, "0.1.0", "LGPL",
    "mipad2-eos-tracer", "https://github.com/E7G/snapshot")
