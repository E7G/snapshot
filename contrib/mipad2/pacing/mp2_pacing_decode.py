"""Read scalar-only trace after producer exits; never reads camera buffers."""
from pathlib import Path
import struct, collections, statistics, json, sys
NONE=(1<<64)-1
HEADER=struct.Struct('<8sII6Q')
RECORD=struct.Struct('<5Q2IiIQ')
DATA_START=64+256*512

def read_trace(path):
 raw=Path(path).read_bytes()
 if len(raw)<DATA_START:raise ValueError('short header/dictionary')
 magic,version,size,cap,reserved,npads,overflow,started,pid=HEADER.unpack_from(raw)
 if (magic,version,size)!=(b'MP2PACE1',1,64):raise ValueError('unsupported ABI')
 if cap<16 or cap>1048576 or len(raw)!=DATA_START+64*cap or npads>256:raise ValueError('bounds')
 pads={i+1:raw[64+i*512:64+(i+1)*512].split(b'\0',1)[0].decode('utf-8',errors='strict') for i in range(npads)}
 records=[];incomplete=0
 for i in range(min(cap,reserved)):
  ts,pts,offset,duration,tid,pad,op,flow,flags,commit=RECORD.unpack_from(raw,DATA_START+64*i)
  if commit!=i+1:incomplete+=1;continue
  if pad not in pads or op not in (1,2,3,4,5):raise ValueError('invalid record')
  records.append(dict(ts=ts,pts=pts,offset=offset,duration=duration,tid=tid,pad=pad,op=op,flow=flow,flags=flags,index=i))
 return dict(capacity=cap,reserved=reserved,pads=pads,overflow=overflow,incomplete=incomplete,started=started,pid=pid,records=records,
             complete=overflow==0 and incomplete==0 and reserved<=cap)

def summarize(trace):
 records=trace['records'];pads=trace['pads'];summary=[];stacks=collections.defaultdict(list)
 intervals=collections.defaultdict(list);unmatched=0
 for r in records:
  if r['op'] in (1,3):stacks[(r['tid'],r['pad'],r['op'])].append(r)
  elif r['op'] in (2,4):
   stack=stacks[(r['tid'],r['pad'],r['op']-1)]
   if not stack:unmatched+=1;continue
   first=stack.pop();delta=r['ts']-first['ts']
   if delta<0:raise ValueError('negative paired interval')
   intervals[(r['pad'],r['op']-1)].append(dict(duration_ns=delta,at_ts=first['ts'],pts=first['pts'],offset=first['offset'],flow=r['flow']))
 for id,name in pads.items():
  for op in (1,3):
   rows=[r for r in records if r['pad']==id and r['op']==op]
   if not rows:continue
   rows.sort(key=lambda r:r['ts']);gaps=[];bad=0
   for a,b in zip(rows,rows[1:]):
    if a['pts']==NONE or b['pts']==NONE:continue
    delta=b['pts']-a['pts']
    if delta<0:bad+=1
    if delta>50_000_000:
     gaps.append(dict(before_pts=a['pts'],after_pts=b['pts'],gap_ns=delta,before_offset=a['offset'],after_offset=b['offset'],at_ts=b['ts'],arrival_gap_ns=b['ts']-a['ts']))
   durations=intervals[(id,op)];sorted_ns=sorted(v['duration_ns'] for v in durations)
   summary.append(dict(pad=id,path=name,op='push' if op==1 else 'chain',buffers=len(rows),first_pts=rows[0]['pts'],last_pts=rows[-1]['pts'],pts_backwards=bad,
     pts_gaps_gt50ms=gaps,paired_calls=len(durations),non_ok_flow_counts=dict(collections.Counter(str(v['flow']) for v in durations if v['flow'])),
     call_p50_ns=statistics.median(sorted_ns) if sorted_ns else None,call_p95_ns=sorted_ns[min(len(sorted_ns)-1,int(len(sorted_ns)*.95))] if sorted_ns else None,
     longest_calls=sorted(durations,key=lambda v:v['duration_ns'],reverse=True)[:8]))
 return dict(**{k:v for k,v in trace.items() if k not in ['records','pads']},pads=summary,
   unmatched_post=unmatched,unmatched_pre=sum(map(len,stacks.values())),
   no_payload_read=True,intervals_include_nested_downstream_calls=True,
   caution='PTS gaps across camera switches/state transitions are retained, not automatically recording losses; align segments and MP4 before attribution.')

if __name__=='__main__':
 data=summarize(read_trace(sys.argv[1]));print(json.dumps(data))
