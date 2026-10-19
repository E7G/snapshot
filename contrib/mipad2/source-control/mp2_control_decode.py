"""Absolute monotonic control tracer ABI2; scalar only, inspect after producer exit."""
import struct,collections
from pathlib import Path
HEADER=struct.Struct('<8sII6Q');RECORD=struct.Struct('<5Q2IiIQ');DATA_START=64+512*512
PAIRS={1:2,3:4,6:7,8:9,10:11,12:13,14:15}
KINDS={1:'push',3:'chain',6:'push-event',8:'pad-query',10:'state',12:'send-event',14:'element-query'}
def read_trace(path):
 raw=Path(path).read_bytes();assert len(raw)>=DATA_START
 magic,version,size,cap,reserved,npads,overflow,started,pid=HEADER.unpack_from(raw)
 assert (magic,version,size)==(b'MP2CTRL1',2,64) and 16<=cap<=1048576 and len(raw)==DATA_START+64*cap and npads<=512
 pads={i+1:raw[64+i*512:64+(i+1)*512].split(b'\0',1)[0].decode() for i in range(npads)}
 records=[];incomplete=0
 for i in range(min(cap,reserved)):
  ts,pts,offset,duration,tid,pad,op,flow,flags,commit=RECORD.unpack_from(raw,DATA_START+i*64)
  if commit!=i+1:incomplete+=1;continue
  assert pad in pads and op in set(PAIRS)|set(PAIRS.values())
  records.append(dict(ts=ts,pts=pts,offset=offset,duration=duration,tid=tid,pad=pad,op=op,flow=flow,flags=flags,index=i))
 return dict(capacity=cap,reserved=reserved,pads=pads,overflow=overflow,incomplete=incomplete,started=started,pid=pid,records=records,complete=not overflow and not incomplete and reserved<=cap)
def intervals(t):
 stacks=collections.defaultdict(list);rows=[];missing=collections.Counter();reverse={v:k for k,v in PAIRS.items()}
 for r in t['records']:
  if r['op'] in PAIRS:stacks[(r['tid'],r['pad'],r['op'])].append(r)
  elif r['op'] in reverse:
   op=reverse[r['op']];stack=stacks[(r['tid'],r['pad'],op)]
   if not stack:missing[KINDS[op]]+=1;continue
   a=stack.pop();assert r['ts']>=a['ts']
   rows.append(dict(begin=a['ts'],end=r['ts'],elapsed_ns=r['ts']-a['ts'],tid=a['tid'],pad=a['pad'],path=t['pads'][a['pad']],operation=KINDS[op],argument=a['flags'] if op in [6,12] else a['flow'],result=r['flow'],pts=a['pts'],offset=a['offset']))
 return dict(rows=rows,unmatched_post=dict(missing),unmatched_pre=dict(collections.Counter({str(k):len(v) for k,v in stacks.items() if v})))
def summarize(t):
 d=intervals(t);counts=collections.Counter(r['operation'] for r in d['rows']);long=[r for r in d['rows'] if r['elapsed_ns']>=40_000_000]
 return dict(complete=t['complete'],reserved=t['reserved'],capacity=t['capacity'],overflow=t['overflow'],incomplete=t['incomplete'],objects=len(t['pads']),operations=dict(counts),unmatched_pre=d['unmatched_pre'],unmatched_post=d['unmatched_post'],long_calls=long,clock='CLOCK_MONOTONIC absolute via gst_util_get_timestamp, not tracer callback ts',no_payload_read=True)
