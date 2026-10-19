# SPDX-License-Identifier: MIT
"""Synthetic RGB/NV12 mapping and zero-copy regression, installed real plugin.
Set public_api_control=True only for a separate control, not patched-code proof.
No camera, display, sound, private pixels or output media files.
"""
from pathlib import Path
import ctypes as C,json,time,subprocess
import argparse,hashlib
parser=argparse.ArgumentParser(description='Real-plugin synthetic RGB/NV12 orientation and default zero-copy regression')
parser.add_argument('--public-api-control',action='store_true',help='Diagnostic control, NOT validation of a patched binary')
public_api_control=parser.parse_args().public_api_control
gst=C.CDLL('libgstreamer-1.0.so.0');app=C.CDLL('libgstapp-1.0.so.0');base=C.CDLL('libgstbase-1.0.so.0');obj=C.CDLL('libgobject-2.0.so.0');ptr=C.c_void_p
def decl(lib,name,restype,args):
 f=getattr(lib,name);f.restype=restype;f.argtypes=args;return f
decl(gst,'gst_init',None,[ptr,ptr])(None,None)
parse=decl(gst,'gst_parse_launch',ptr,[C.c_char_p,ptr]);get=decl(gst,'gst_bin_get_by_name',ptr,[ptr,C.c_char_p])
capsfrom=decl(gst,'gst_caps_from_string',ptr,[C.c_char_p]);structure=decl(gst,'gst_caps_get_structure',ptr,[ptr,C.c_uint]);getint=decl(gst,'gst_structure_get_int',C.c_int,[ptr,C.c_char_p,ptr])
allocate=decl(gst,'gst_buffer_new_allocate',ptr,[ptr,C.c_size_t,ptr]);fill=decl(gst,'gst_buffer_fill',C.c_size_t,[ptr,C.c_size_t,ptr,C.c_size_t]);extract=decl(gst,'gst_buffer_extract',C.c_size_t,[ptr,C.c_size_t,ptr,C.c_size_t]);size=decl(gst,'gst_buffer_get_size',C.c_size_t,[ptr]);peek=decl(gst,'gst_buffer_peek_memory',ptr,[ptr,C.c_uint]);ref=decl(gst,'gst_mini_object_ref',ptr,[ptr]);miniunref=decl(gst,'gst_mini_object_unref',None,[ptr]);unref=decl(gst,'gst_object_unref',None,[ptr])
setstate=decl(gst,'gst_element_set_state',C.c_int,[ptr,C.c_int]);getstate=decl(gst,'gst_element_get_state',C.c_int,[ptr,ptr,ptr,C.c_uint64]);push=decl(app,'gst_app_src_push_buffer',C.c_int,[ptr,ptr]);eos=decl(app,'gst_app_src_end_of_stream',C.c_int,[ptr]);setcaps=decl(app,'gst_app_src_set_caps',None,[ptr,ptr]);pull=decl(app,'gst_app_sink_try_pull_sample',ptr,[ptr,C.c_uint64]);samplebuf=decl(gst,'gst_sample_get_buffer',ptr,[ptr]);samplecaps=decl(gst,'gst_sample_get_caps',ptr,[ptr]);setpass=decl(base,'gst_base_transform_set_passthrough',None,[ptr,C.c_int]);passthrough=decl(base,'gst_base_transform_is_passthrough',C.c_int,[ptr]);obj.g_object_set.argtypes=[ptr,C.c_char_p];obj.g_object_set.restype=None
def coords(method,x,y,w,h):
 return [(x,y),(y,h-1-x),(w-1-x,h-1-y),(w-1-y,x),(w-1-x,y),(x,h-1-y),(y,x),(w-1-y,h-1-x)][method]
def layout(fmt,w,h):
 stride=((w*(3 if fmt=='RGB' else 1)+3)//4)*4
 return stride,stride*h+(stride*h//2 if fmt=='NV12' else 0)
results=[]
for fmt in ['RGB','NV12']:
 for method in range(8):
  w,h=4,2;stride,n=layout(fmt,w,h);raw=bytearray(n)
  for y in range(h):
   for x in range(w):
    for c in range(3 if fmt=='RGB' else 1):raw[y*stride+x*(3 if fmt=='RGB' else 1)+c]=11+20*y+3*x+c
  if fmt=='NV12':
   for y in range(h//2):
    for x in range(w//2):
     raw[stride*h+y*stride+2*x:stride*h+y*stride+2*x+2]=bytes([80+10*x+20*y,120+10*x+20*y])
  pipe=parse(b'appsrc name=src format=time ! videoflip name=flip ! appsink name=sink sync=false',None);assert pipe
  src=get(pipe,b'src');sink=get(pipe,b'sink');flip=get(pipe,b'flip')
  caps=capsfrom(('video/x-raw,format='+fmt+',width=4,height=2,framerate=30/1').encode());assert caps;setcaps(src,caps);miniunref(caps)
  obj.g_object_set(flip,b'method',C.c_int(method),ptr())
  if public_api_control and method==0:setpass(flip,1)
  original=allocate(None,n,None);memory=peek(original,0);input_array=(C.c_ubyte*n).from_buffer(raw);assert fill(original,0,C.cast(input_array,ptr),n)==n
  sample=None
  try:
   assert setstate(pipe,4)!=0
   assert push(src,ref(original))==0 and eos(src)==0
   sample=pull(sink,5_000_000_000);assert sample
   buffer=samplebuf(sample);st=structure(samplecaps(sample),0);ow=C.c_int();oh=C.c_int();assert getint(st,b'width',C.byref(ow)) and getint(st,b'height',C.byref(oh))
   ew,eh=(h,w) if method in [1,3,6,7] else (w,h);assert (ow.value,oh.value)==(ew,eh)
   outstride,outsize=layout(fmt,ew,eh);assert size(buffer)>=outsize
   data=(C.c_ubyte*outsize)();assert extract(buffer,0,C.cast(data,ptr),outsize)==outsize
   checked=0;different=0
   for y in range(eh):
    for x in range(ew):
     sx,sy=coords(method,x,y,w,h)
     for c in range(3 if fmt=='RGB' else 1):
      expected=raw[sy*stride+sx*(3 if fmt=='RGB' else 1)+c];actual=data[y*outstride+x*(3 if fmt=='RGB' else 1)+c];different+=actual!=expected;checked+=1
   if fmt=='NV12':
    for y in range(eh//2):
     for x in range(ew//2):
      sx,sy=coords(method,x,y,w//2,h//2)
      for c in range(2):different+=data[outstride*eh+y*outstride+2*x+c]!=raw[stride*h+sy*stride+2*sx+c];checked+=1
   alias=peek(buffer,0)==memory
   item=dict(format=fmt,method=method,geometry_correct=True,checked_active_component_values=checked,wrong_component_values=different,same_pixel_memory=alias,active_passthrough=bool(passthrough(flip)))
   item['passed']=different==0 and (method!=0 or alias);results.append(item)
  finally:
   if sample:miniunref(sample)
   miniunref(original);assert setstate(pipe,1)!=0
   state=C.c_int();pending=C.c_int();assert getstate(pipe,C.byref(state),C.byref(pending),5_000_000_000)!=0 and state.value==1
   for p in [src,sink,flip,pipe]:unref(p)
r=dict(at=time.time(),public_api_control=public_api_control,synthetic_only=True,binary_patch_status_not_inferred=True,results=results,all_pixel_mappings_passed=all(x['wrong_component_values']==0 for x in results),zero_copy_and_mapping_gate=all(x['passed'] for x in results),system_plugin_changed=False)
factory=decl(gst,'gst_element_factory_find',ptr,[C.c_char_p])(b'videoflip');assert factory
plugin=decl(gst,'gst_plugin_feature_get_plugin',ptr,[ptr])(factory);assert plugin
filename=decl(gst,'gst_plugin_get_filename',C.c_char_p,[ptr])(plugin);assert filename
path=Path(filename.decode());r['actual_plugin_filename']=str(path);r['actual_plugin_sha256']=hashlib.sha256(path.read_bytes()).hexdigest();r['no_camera_access']=True
unref(plugin);unref(factory)
print(json.dumps(r));raise SystemExit(0 if r['zero_copy_and_mapping_gate'] else 1)
