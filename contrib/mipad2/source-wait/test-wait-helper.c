
#include <gst/gst.h>
#include "wait-helper.inc"
static gint early (GstObject *object) {
  g_auto(Mp2WaitSpan) span = mp2_wait_begin (object, "early-return");
  g_usleep (45000);
  return 17;
}
static gint error_before_metadata (GstObject *object) {
  g_auto(Mp2WaitSpan) span = { object, "must-not-log-uninitialized", GST_CLOCK_TIME_NONE };
  goto failed;
  span = mp2_wait_begin (object, "unreached");
failed:
  return 23;
}
int main (int argc, char **argv) {
  gst_init (&argc, &argv);
  GstElement *object = gst_element_factory_make ("fakesrc", "actual-test-object");
  g_assert_nonnull (object);
  g_assert_cmpint (error_before_metadata (GST_OBJECT (object)), ==, 23);
  if (g_strcmp0 (g_getenv ("MP2_HELPER_CASE"), "cap") == 0) {
    for (int i = 0; i < 140; i++) {
      g_auto(Mp2WaitSpan) span = mp2_wait_begin (GST_OBJECT (object), "cap");
      if (GST_CLOCK_TIME_IS_VALID (span.begin))
        span.begin -= 45 * GST_MSECOND;
    }
    gst_object_unref (object);
    return 0;
  }

  { g_auto(Mp2WaitSpan) span = mp2_wait_begin (GST_OBJECT (object), "fast"); }
  g_assert_cmpint (early (GST_OBJECT (object)), ==, 17);
  { g_auto(Mp2WaitSpan) span = mp2_wait_begin (GST_OBJECT (object), "slow"); g_usleep (45000); }
  gst_object_unref (object);
  return 0;
}
