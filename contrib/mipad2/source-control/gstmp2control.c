/* SPDX-License-Identifier: LGPL-2.1-or-later */
/* Bounded, metadata-only Linux tracer. No buffer map/ref, no per-frame I/O.
 * The private mmap is process-lifetime; inspect only after producer exits. */
#define _GNU_SOURCE
#include <gst/gst.h>
#include <gst/gsttracer.h>
#include <stdatomic.h>
#include <stdint.h>
#include <string.h>
#include <fcntl.h>
#include <sys/mman.h>
#include <sys/stat.h>
#include <sys/syscall.h>
#include <unistd.h>
#ifndef PACKAGE
#define PACKAGE "mipad2-control-tracer"
#endif
#define PAD_CAP 512
#define PAD_SIZE 512
typedef struct {
  char magic[8]; uint32_t version, record_size;
  uint64_t capacity;
  _Atomic uint64_t reserved, pads, overflow;
  uint64_t started, pid;
} Header;
typedef struct {
  uint64_t ts, pts, offset, duration, tid;
  uint32_t pad, op; int32_t flow; uint32_t flags;
  _Atomic uint64_t committed;
} Record;
typedef struct { GstTracer parent; } Mp2Control;
typedef struct { GstTracerClass parent; } Mp2ControlClass;
typedef struct { uint32_t id; } PadState;
_Static_assert(sizeof(Header)==64,"header ABI");
_Static_assert(sizeof(Record)==64,"record ABI");
G_DEFINE_TYPE(Mp2Control, mp2_control, GST_TYPE_TRACER)
static Header *header;
static char *dictionary;
static Record *records;
static GQuark key;
G_LOCK_DEFINE_STATIC(pad_creation);
static _Thread_local uint64_t thread_id;

static gboolean selected(const char *type) {
  const char *names[]={"GstV4l2Src","GstQueue","GstMultiQueue","GstTee",
    "GstVaH264Enc","GstH264Parse","GstVideoScale","GstVideoConvert",
    "GstVideoFlip","GstVideoTestSrc","GstIdentity","GstCameraBin",
    "GstWrapperCameraBinSrc","GstBin","GstPipeline","GstPulseSrc",
    "GstAutoAudioSrc","GstGLSinkBin",NULL};
  for(unsigned i=0;names[i];i++) if(!strcmp(type,names[i])) return TRUE;
  return FALSE;
}
static uint32_t pad_id(GstObject *pad) {
  PadState *state=g_object_get_qdata(G_OBJECT(pad),key);
  if(state) return state->id;
  G_LOCK(pad_creation);
  state=g_object_get_qdata(G_OBJECT(pad),key);
  if(!state) {
    GstElement *element=GST_IS_PAD(pad)?gst_pad_get_parent_element(GST_PAD(pad)):
      GST_IS_ELEMENT(pad)?GST_ELEMENT(gst_object_ref(pad)):NULL;
    if(!element) { G_UNLOCK(pad_creation); return 0; }
    state=g_new0(PadState,1);
    const char *type=G_OBJECT_TYPE_NAME(element);
    if(selected(type)) {
      uint64_t n=atomic_load_explicit(&header->pads,memory_order_relaxed);
      if(n<PAD_CAP) {
        char *path=gst_object_get_path_string(GST_OBJECT(pad));
        int bytes=g_snprintf(dictionary+n*PAD_SIZE,PAD_SIZE,"%s\t%s",type,path);
        if(bytes>=PAD_SIZE) atomic_fetch_add(&header->overflow,1);
        g_free(path);
        state->id=(uint32_t)n+1;
        atomic_store_explicit(&header->pads,n+1,memory_order_release);
      } else atomic_fetch_add(&header->overflow,1);
    }
    gst_object_unref(element);
    g_object_set_qdata_full(G_OBJECT(pad),key,state,g_free);
  }
  uint32_t id=state->id;
  G_UNLOCK(pad_creation);
  return id;
}
static void record(GstClockTime ts,GstObject *pad,uint32_t op,GstBuffer *buf,
    gint32 flow,GstEvent *event) {
  uint32_t id=pad_id(pad);
  if(!id) return;
  uint64_t n=atomic_fetch_add_explicit(&header->reserved,1,memory_order_relaxed);
  if(n>=header->capacity) { atomic_fetch_add(&header->overflow,1); return; }
  if(!thread_id) thread_id=(uint64_t)syscall(SYS_gettid);
  Record *r=&records[n];
  (void)ts;
  r->ts=gst_util_get_timestamp(); r->pad=id; r->op=op; r->tid=thread_id; r->flow=flow;
  r->pts=buf?GST_BUFFER_PTS(buf):GST_CLOCK_TIME_NONE;
  r->offset=buf?GST_BUFFER_OFFSET(buf):GST_BUFFER_OFFSET_NONE;
  r->duration=buf?GST_BUFFER_DURATION(buf):GST_CLOCK_TIME_NONE;
  r->flags=buf?GST_BUFFER_FLAGS(buf):0;
  if(event) {
    r->flags=GST_EVENT_TYPE(event); r->offset=gst_event_get_seqnum(event);
    if(GST_EVENT_TYPE(event)==GST_EVENT_SEGMENT) {
      const GstSegment *s;gst_event_parse_segment(event,&s);
      r->pts=s->start;r->duration=s->base;r->flow=s->format;
    }
  }
  atomic_store_explicit(&r->committed,n+1,memory_order_release);
}
static void push_pre(GObject *s,GstClockTime t,GstPad *p,GstBuffer *b) {
  (void)s; record(t,GST_OBJECT(p),1,b,GST_FLOW_OK,NULL);
}
static void push_post(GObject *s,GstClockTime t,GstPad *p,GstFlowReturn f) {
  (void)s; record(t,GST_OBJECT(p),2,NULL,f,NULL);
}
static void chain_pre(GObject *s,GstClockTime t,GstPad *p,GstBuffer *b) {
  (void)s; record(t,GST_OBJECT(p),3,b,GST_FLOW_OK,NULL);
}
static void chain_post(GObject *s,GstClockTime t,GstPad *p,GstFlowReturn f) {
  (void)s; record(t,GST_OBJECT(p),4,NULL,f,NULL);
}

static void event_pre(GObject *s,GstClockTime t,GstPad *p,GstEvent *e) {
  (void)s; record(t,GST_OBJECT(p),6,NULL,GST_FLOW_OK,e);
}
static void event_post(GObject *s,GstClockTime t,GstPad *p,gboolean res) {
  (void)s; record(t,GST_OBJECT(p),7,NULL,res,NULL);
}
static void query_pre(GObject *s,GstClockTime t,GstPad *p,GstQuery *q) {
  (void)s; record(t,GST_OBJECT(p),8,NULL,GST_QUERY_TYPE(q),NULL);
}
static void query_post(GObject *s,GstClockTime t,GstPad *p,GstQuery *q,gboolean res) {
  (void)s;(void)q; record(t,GST_OBJECT(p),9,NULL,res,NULL);
}
static void state_pre(GObject *s,GstClockTime t,GstElement *e,GstStateChange change) {
  (void)s; record(t,GST_OBJECT(e),10,NULL,change,NULL);
}
static void state_post(GObject *s,GstClockTime t,GstElement *e,GstStateChange change,GstStateChangeReturn res) {
  (void)s;(void)change; record(t,GST_OBJECT(e),11,NULL,res,NULL);
}
static void send_pre(GObject *s,GstClockTime t,GstPad *p,GstEvent *e) {
  (void)s; record(t,GST_OBJECT(p),12,NULL,GST_FLOW_OK,e);
}
static void send_post(GObject *s,GstClockTime t,GstPad *p,GstFlowReturn res) {
  (void)s; record(t,GST_OBJECT(p),13,NULL,res,NULL);
}
static void element_query_pre(GObject *s,GstClockTime t,GstElement *e,GstQuery *q) {
  (void)s; record(t,GST_OBJECT(e),14,NULL,GST_QUERY_TYPE(q),NULL);
}
static void element_query_post(GObject *s,GstClockTime t,GstElement *e,GstQuery *q,gboolean res) {
  (void)s;(void)q; record(t,GST_OBJECT(e),15,NULL,res,NULL);
}
static void mp2_control_init(Mp2Control *self) {
  if(header) { g_printerr("MIPAD2_CONTROL duplicate tracer ignored\n"); return; }
  const char *path=g_getenv("MIPAD2_CONTROL_FILE"),*caps=g_getenv("MIPAD2_CONTROL_CAPACITY");
  uint64_t cap=caps?g_ascii_strtoull(caps,NULL,10):524288;
  if(!path || path[0]!='/' || cap<16 || cap>1048576) {
    g_printerr("MIPAD2_CONTROL invalid private file/capacity; disabled\n");return;
  }
  size_t size=sizeof(Header)+PAD_CAP*PAD_SIZE+cap*sizeof(Record);
  int fd=open(path,O_RDWR|O_CREAT|O_EXCL|O_CLOEXEC|O_NOFOLLOW,0600);
  if(fd<0) { g_printerr("MIPAD2_CONTROL exclusive open failed; disabled\n");return; }
  if(ftruncate(fd,(off_t)size)) {close(fd);return;}
  void *map=mmap(NULL,size,PROT_READ|PROT_WRITE,MAP_SHARED,fd,0);close(fd);
  if(map==MAP_FAILED) return;
  memset(map,0,size);header=map;
  atomic_init(&header->reserved,0);atomic_init(&header->pads,0);atomic_init(&header->overflow,0);
  header->version=2;header->record_size=sizeof(Record);header->capacity=cap;
  header->started=gst_util_get_timestamp();header->pid=(uint64_t)getpid();
  dictionary=(char*)map+sizeof(Header);records=(Record*)(dictionary+PAD_CAP*PAD_SIZE);
  if(!atomic_is_lock_free(&header->reserved)) {
    header=NULL;munmap(map,size);g_printerr("MIPAD2_CONTROL non-lock-free; disabled\n");return;
  }
  memcpy(header->magic,"MP2CTRL1",8);
  key=g_quark_from_static_string("mipad2-control:pad-id-v1");
  gst_tracing_register_hook(GST_TRACER(self),"pad-push-pre",G_CALLBACK(push_pre));
  gst_tracing_register_hook(GST_TRACER(self),"pad-push-post",G_CALLBACK(push_post));
  gst_tracing_register_hook(GST_TRACER(self),"pad-chain-pre",G_CALLBACK(chain_pre));
  gst_tracing_register_hook(GST_TRACER(self),"pad-chain-post",G_CALLBACK(chain_post));
  gst_tracing_register_hook(GST_TRACER(self),"pad-push-event-pre",G_CALLBACK(event_pre));
  gst_tracing_register_hook(GST_TRACER(self),"pad-push-event-post",G_CALLBACK(event_post));
  gst_tracing_register_hook(GST_TRACER(self),"pad-query-pre",G_CALLBACK(query_pre));
  gst_tracing_register_hook(GST_TRACER(self),"pad-query-post",G_CALLBACK(query_post));
  gst_tracing_register_hook(GST_TRACER(self),"element-change-state-pre",G_CALLBACK(state_pre));
  gst_tracing_register_hook(GST_TRACER(self),"element-change-state-post",G_CALLBACK(state_post));
  gst_tracing_register_hook(GST_TRACER(self),"pad-send-event-pre",G_CALLBACK(send_pre));
  gst_tracing_register_hook(GST_TRACER(self),"pad-send-event-post",G_CALLBACK(send_post));
  gst_tracing_register_hook(GST_TRACER(self),"element-query-pre",G_CALLBACK(element_query_pre));
  gst_tracing_register_hook(GST_TRACER(self),"element-query-post",G_CALLBACK(element_query_post));
  g_printerr("MIPAD2_CONTROL enabled v2 absolute-mono capacity=%"G_GUINT64_FORMAT" metadata-only=1\n",cap);
}
static void mp2_control_class_init(Mp2ControlClass *klass) {(void)klass;}
static gboolean plugin_init(GstPlugin *p) {
  return gst_tracer_register(p,"mipad2-control",mp2_control_get_type());
}
GST_PLUGIN_DEFINE(GST_VERSION_MAJOR,GST_VERSION_MINOR,mp2control,
    "Mi Pad2 bounded in-memory scalar pacing tracer",plugin_init,"0.1.0","LGPL",
    "mipad2-control-tracer","https://github.com/E7G/snapshot")
