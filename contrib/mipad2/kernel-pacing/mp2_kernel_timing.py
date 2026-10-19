"""Scalar tracefs function/IRQ/scheduler parsing. No camera memory or images."""
import re,bisect,collections
LINE=re.compile(r'^\s*(.+)-(\d+)\s+\[(\d+)\]\s+\S+\s+(\d+\.\d+):\s+(.*)$')
def ns(text):
 whole,dot,part=text.partition('.')
 if not dot or len(part)>9:raise ValueError('unsupported timestamp precision')
 return int(whole)*10**9+int(part.ljust(9,'0'))

def parse_trace(text):
 records=[];unknown=[];lost=[]
 for number,line in enumerate(text.splitlines(),1):
  if not line.strip() or line.startswith('#'):continue
  if 'LOST' in line or 'MISSED' in line:lost.append(dict(line=number,text=line));continue
  m=LINE.match(line)
  if not m:unknown.append(dict(line=number,text=line));continue
  comm,pid,cpu,stamp,body=m.groups();row=dict(ts=ns(stamp),pid=int(pid),cpu=int(cpu),comm=comm.strip(),line=number)
  if ' <-' in body:row.update(kind='function',name=body.split(' <-',1)[0].strip(),caller=body.split(' <-',1)[1])
  elif ': ' in body:
   name,detail=body.split(': ',1);row.update(kind='event',name=name,detail=detail,fields=dict(re.findall(r'(\w+)=(\S+)',detail)))
  else:unknown.append(dict(line=number,text=line));continue
  records.append(row)
 records.sort(key=lambda r:(r['ts'],r['line']))
 return dict(records=records,unknown=unknown,lost=lost,syntax_complete=not unknown and not lost,printed_timestamp_resolution_ns=1000)

def completion_brackets(records):
 pending={};brackets=[];unmatched=[]
 for r in records:
  if r['kind']!='function':continue
  if r['name']=='atomisp_buf_done':
   if r['pid'] in pending:unmatched.append(pending[r['pid']])
   pending[r['pid']]=r
  elif r['name']=='vb2_buffer_done' and r['pid'] in pending:
   first=pending.pop(r['pid']);brackets.append(dict(begin=first['ts'],end=r['ts'],pid=r['pid'],cpu_begin=first['cpu'],cpu_end=r['cpu'],begin_line=first['line'],end_line=r['line']))
 return dict(brackets=brackets,unmatched_atomisp_entry=unmatched+list(pending.values()))

def match_stamp(stamp,brackets):
 # Both printed trace timestamps and timeval driver stamps truncate to us.
 # Equal edges/duplicates remain ambiguous; never guess nearest ownership.
 found=[b for b in brackets if b['begin']<=stamp<=b['end']]
 return dict(matches=len(found),bracket=found[0] if len(found)==1 else None)

def scheduler_intervals(records,tid):
 off=None;wake=None;rows=[];unmatched_in=0
 for r in records:
  if r['kind']!='event':continue
  f=r.get('fields',{})
  if r['name'] in ('sched_waking','sched_wakeup') and int(f.get('pid',-1))==tid:
   if off is not None and wake is None:wake=r['ts']
  if r['name']!='sched_switch':continue
  if int(f.get('next_pid',-1))==tid:
   if off is None:unmatched_in+=1
   else:rows.append(dict(out=off['ts'],back=r['ts'],duration=r['ts']-off['ts'],state=off['fields']['prev_state'],wake=wake,post_wake_delay_ns=r['ts']-wake if wake is not None else None,cpu_out=off['cpu'],cpu_back=r['cpu']))
   off=None;wake=None
  if int(f.get('prev_pid',-1))==tid:off=r;wake=None
 return dict(intervals=rows,right_censored=off,left_censored_count=unmatched_in)

def clip_intervals(intervals,begin,end):
 return [dict(v,overlap_ns=max(0,min(end,v['back'])-max(begin,v['out']))) for v in intervals if v['out']<end and v['back']>begin]

def analyze(records,diagnostics,tid):
 brackets=completion_brackets(records);sched=scheduler_intervals(records,tid)
 anomalies=[]
 for d in diagnostics:
  vals=d['values'];a=vals['driver-prev'];b=vals['driver'];ma=match_stamp(a,brackets['brackets']);mb=match_stamp(b,brackets['brackets'])
  inside=[r for r in records if a<=r['ts']<=b]
  threads=[r for r in records if r['kind']=='function' and r['name']=='atomisp_isr_thread' and r['pid']==tid and r['ts']<=b]
  latest=threads[-1] if threads else None
  anomalies.append(dict(cycle=d['cycle'],driver_prev=a,driver=b,gap_ns=b-a,before_match=ma,after_match=mb,
   irq_entries=[dict(ts=r['ts'],cpu=r['cpu']) for r in inside if r['name']=='irq_handler_entry'],
   irq_exits=[dict(ts=r['ts'],cpu=r['cpu'],fields=r['fields']) for r in inside if r['name']=='irq_handler_exit'],
   threaded_handler_entries=[dict(ts=r['ts'],pid=r['pid']) for r in inside if r['name']=='atomisp_isr_thread'],
   latest_thread_entry=dict(ts=latest['ts'],age_to_driver_ns=b-latest['ts']) if latest else None,
   irq_thread_offcpu=clip_intervals(sched['intervals'],a,b),
   functions_in_gap=[dict(ts=r['ts'],pid=r['pid'],name=r['name']) for r in inside if r['kind']=='function'],
   caution='Function entries only: no invented return durations. IRQ delivery is not sensor SOF; contiguous software sequence does not prove optical continuity.'))
 return dict(anomalies=anomalies,completion_brackets=len(brackets['brackets']),unmatched_completion_entries=len(brackets['unmatched_atomisp_entry']),scheduler_intervals=len(sched['intervals']),scheduler_left_censored=sched['left_censored_count'],scheduler_right_censored=bool(sched['right_censored']),
  completion_pairing_caution='atomisp_buf_done also handles statistics/nonvideo buffer types. Entries without observed VB2 completion are not automatically dropped video frames or unbalanced lifecycle calls.')

def baseline(records,tid):
 entries=[r['ts'] for r in records if r['kind']=='function' and r['name']=='atomisp_isr_thread' and r['pid']==tid]
 sched=scheduler_intervals(records,tid);spans=[]
 # One thread's off-CPU intervals do not overlap. Avoid scanning all intervals
 # for every video completion (important when analyzing on low-power tablets).
 offs=[v['out'] for v in sched['intervals']];backs=[v['back'] for v in sched['intervals']]
 for bracket in completion_brackets(records)['brackets']:
  if bracket['pid']!=tid:continue
  ix=bisect.bisect_right(entries,bracket['begin'])-1
  if ix<0:continue
  begin=entries[ix];end=bracket['begin']
  first=bisect.bisect_right(backs,begin);last=bisect.bisect_left(offs,end)
  off=clip_intervals(sched['intervals'][first:last],begin,end)
  spans.append(dict(handler_entry=begin,completion_entry=end,elapsed_ns=end-begin,
   offcpu=off,offcpu_overlap_ns=sum(v['overlap_ns'] for v in off)))
 values=sorted(v['elapsed_ns'] for v in spans)
 return dict(video_completion_brackets_in_irq_thread=len(spans),
  handler_to_completion_entry_p50_ns=values[len(values)//2] if values else None,
  handler_to_completion_entry_p99_ns=values[min(len(values)-1,int(len(values)*.99))] if values else None,
  longest_handler_spans=sorted(spans,key=lambda v:v['elapsed_ns'],reverse=True)[:12],
  irq_thread_offcpu_states=dict(collections.Counter(v['state'] for v in sched['intervals'])),
  semantic_caution='atomisp_buf_done handles statistics and other buffer types too; entries without a subsequent VB2 completion are not automatically lost frames or lifecycle failures. Entry-only spans do not identify a blocking primitive or return time.')
