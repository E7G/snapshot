"""Standard V4L2/VB2 scalar metadata joins; never dereferences owner/payload."""
import re,bisect,collections
from mp2_kernel_timing import parse_trace as parse_kernel
V4L2={'v4l2_qbuf','v4l2_dqbuf','vb2_v4l2_buf_queue','vb2_v4l2_buf_done','vb2_v4l2_qbuf','vb2_v4l2_dqbuf'}
VB2={'vb2_buf_queue','vb2_buf_done','vb2_qbuf','vb2_dqbuf'}
def fields(detail):
 head=detail.split(', timecode =',1)[0]
 values=dict(re.findall(r'(\w+)\s*=\s*([^\s,{}]+)',head))
 seq=re.search(r'(?:^|[,} ])sequence\s*=\s*(\d+)',detail)
 if seq:values['sequence']=seq.group(1)
 return values
def parse_trace(text):
 p=parse_kernel(text)
 for r in p['records']:
  if r['kind']=='event' and r['name'] in V4L2|VB2:r['fields']=fields(r['detail'])
 return p
def number(row,key):return int(row['fields'][key])

def identify_owners(records,minor=0):
 # V4L2 timeval truncates the raw VB2 nanosecond stamp to microseconds.
 # Join successful dequeue events by thread/index/stamp and bounded ordering.
 rows=collections.defaultdict(list)
 for r in records:
  if r['name']=='vb2_dqbuf':rows[(r['pid'],number(r,'index'),number(r,'timestamp')//1000)].append(r)
 owners=collections.Counter();unique=ambiguous=missing=0
 for r in records:
  if r['name']!='v4l2_dqbuf' or number(r,'minor')!=minor:continue
  candidates=[x for x in rows[(r['pid'],number(r,'index'),number(r,'timestamp')//1000)] if 0<=r['ts']-x['ts']<=10_000_000]
  if len(candidates)==1:owners[candidates[0]['fields']['owner']]+=1;unique+=1
  elif candidates:ambiguous+=1
  else:missing+=1
 return dict(owners=owners,unique=unique,ambiguous=ambiguous,missing=missing)

def zero_intervals(rows):
 current=None;intervals=[]
 for r in rows:
  owned=number(r,'owned_by_drv')
  if owned==0 and current is None:current=r
  elif owned>0 and current is not None:
   intervals.append(dict(begin=current['ts'],end=r['ts'],duration_ns=r['ts']-current['ts'],begin_event=current['name'],end_event=r['name'],queued_at_zero=number(current,'queued'),return_index=number(r,'index')));current=None
 return dict(intervals=intervals,right_censored_begin=current['ts'] if current else None)

def compact(r):
 keys=['index','timestamp','queued','owned_by_drv','sequence','minor','flags','type']
 return dict(ts=r['ts'],event=r['name'],pid=r['pid'],**{k:r['fields'][k] for k in keys if k in r.get('fields',{})})

def analyze(records,diagnostics,minor=0):
 association=identify_owners(records,minor);owners=association.pop('owners')
 streams=[];anomalies=[]
 for queue,owner in enumerate(owners,1):
  rows=[r for r in records if r['name'] in VB2 and r.get('fields',{}).get('owner')==owner]
  zs=zero_intervals(rows);done=[r for r in rows if r['name']=='vb2_buf_done'];ends=[v['end'] for v in zs['intervals']]
  requeues=[];pending={};right_censored=0
  for r in rows:
   idx=number(r,'index')
   if r['name']=='vb2_dqbuf':
    if idx in pending:right_censored+=1
    pending[idx]=r
   elif r['name']=='vb2_buf_queue' and idx in pending:
    prev=pending.pop(idx);requeues.append(r['ts']-prev['ts'])
  vals=sorted(requeues)
  streams.append(dict(queue=queue,owner_verified_dequeues=owners[owner],buffer_indices=sorted(set(number(r,'index') for r in rows)),events=dict(collections.Counter(r['name'] for r in rows)),
    owned_by_driver_histogram={name:dict(collections.Counter(str(number(r,'owned_by_drv')) for r in rows if r['name']==name)) for name in VB2},
    zero_driver_owned_periods=zs['intervals'],zero_right_censored_begin=zs['right_censored_begin'],requeue_p50_ns=vals[len(vals)//2] if vals else None,requeue_p99_ns=vals[min(len(vals)-1,int(len(vals)*.99))] if vals else None,requeue_max_ns=max(vals) if vals else None,requeue_right_censored=right_censored+len(pending)))
  times=[r['ts'] for r in rows]
  for d in diagnostics:
   a=d['values']['driver-prev'];b=d['values']['driver']
   before=[r for r in done if number(r,'timestamp')//1000==a//1000]
   after=[r for r in done if number(r,'timestamp')//1000==b//1000]
   # Preserve ambiguous/not-matched diagnostics; do not assign nearest owner.
   entry=dict(queue=queue,cycle=d['cycle'],driver_prev=a,driver=b,driver_gap_ns=b-a,before_matches=len(before),after_matches=len(after),same_minor_owner_verified=True)
   if len(before)==len(after)==1:
    begin=before[0]['ts'];end=after[0]['ts'];first=bisect.bisect_right(ends,begin)
    overlaps=[]
    for v in zs['intervals'][first:]:
     if v['begin']>=end:break
     overlaps.append(dict(v,overlap_ns=max(0,min(end,v['end'])-max(begin,v['begin']))))
    lo=bisect.bisect_left(times,begin-100_000_000);hi=bisect.bisect_right(times,end+50_000_000)
    entry.update(before=compact(before[0]),after=compact(after[0]),zero_driver_owned_overlaps=overlaps,context=[compact(v) for v in rows[lo:hi]],
      observed_zero_driver_owned_ns=sum(v['overlap_ns'] for v in overlaps))
   anomalies.append(entry)
 return dict(owner_association=association,queue_count=len(owners),queues=streams,anomalies=anomalies,owner_tokens_exported=False,
  caveat='Driver-owned VB2 count is not the private CSS queue depth. Shutdown/stream-switch zero periods are retained, not automatically recording starvation. Successful metadata joins require unique indexed driver stamps; no frame/payload memory is read.')
