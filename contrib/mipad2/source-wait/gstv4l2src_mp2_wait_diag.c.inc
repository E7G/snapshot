/* Opt-in slow-operation scalar timing. No payload reads, caps/PTS/flow changes. */
typedef struct {
  GstObject *object; /* borrowed for the synchronous lexical scope */
  const gchar *operation;
  GstClockTime begin;
} Mp2WaitSpan;

static gboolean
mp2_wait_enabled (void)
{
  static gsize enabled_once;
  if (g_once_init_enter (&enabled_once)) {
    gboolean enabled = g_strcmp0 (g_getenv ("MIPAD2_V4L2_WAIT_DIAG"), "1") == 0;
    g_once_init_leave (&enabled_once, enabled ? 2 : 1);
  }
  return enabled_once == 2;
}

static Mp2WaitSpan
mp2_wait_begin (GstObject *object, const gchar *operation)
{
  Mp2WaitSpan span = { object, operation, GST_CLOCK_TIME_NONE };
  if (mp2_wait_enabled ())
    span.begin = gst_util_get_timestamp ();
  return span;
}

static void
mp2_wait_end (Mp2WaitSpan *span)
{
  static gint emitted;
  GstClockTime end, elapsed;
  if (!GST_CLOCK_TIME_IS_VALID (span->begin))
    return;
  end = gst_util_get_timestamp ();
  elapsed = end - span->begin;
  if (elapsed < 40 * GST_MSECOND)
    return;
  /* A bounded process-wide slow-only log. No normal-frame logging or writes. */
  if (g_atomic_int_add (&emitted, 1) < 128)
    g_printerr ("MP2_V4L2_WAIT source=%s op=%s begin=%" G_GUINT64_FORMAT
        " end=%" G_GUINT64_FORMAT " elapsed=%" G_GUINT64_FORMAT "\n",
        GST_OBJECT_NAME (span->object), span->operation, span->begin, end, elapsed);
}

G_DEFINE_AUTO_CLEANUP_CLEAR_FUNC (Mp2WaitSpan, mp2_wait_end)
