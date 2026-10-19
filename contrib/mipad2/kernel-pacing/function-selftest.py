"""Own exact-function trace instance. Never writes shared trace_options."""
from pathlib import Path
import os,time,json,re
assert os.geteuid()==0
root=Path('/sys/kernel/tracing');base=Path('/var/tmp/mipad2-kernel-pacing-20261019');assert base.is_dir()
instance=root/'instances/mipad2-pacing-function-selftest-20261019';assert not instance.exists()
keys=['current_tracer','tracing_on','trace_clock','trace_options','set_ftrace_filter','set_ftrace_notrace','set_graph_function','set_graph_notrace']
def global_state():
 return dict(files={k:(root/k).read_text().strip() for k in keys if (root/k).is_file()},options={p.name:p.read_text().strip() for p in (root/'options').iterdir() if p.name.startswith('funcgraph-') or p.name in ['sleep-time','graph-time']},kprobes=(root/'kprobe_events').read_text().strip())
before=global_state();r=dict(pid=os.getpid(),before=before,started_at=time.time(),synthetic_only=True,no_trace_options_writes=True)
def write(name,value):(instance/name).write_text(str(value)+'\n')
try:
 instance.mkdir();write('tracing_on',0);write('buffer_size_kb',256);write('trace_clock','mono')
 write('set_ftrace_filter','__x64_sys_getpid\n__x64_sys_clock_nanosleep');write('set_ftrace_pid',os.getpid());write('current_tracer','function')
 assert global_state()==before
 (instance/'trace').write_text('');write('tracing_on',1)
 for _ in range(8):os.getpid()
 a=time.monotonic_ns();write('trace_marker','MP2_SYNTHETIC_BEFORE mono='+str(a));time.sleep(.035);b=time.monotonic_ns();write('trace_marker','MP2_SYNTHETIC_AFTER mono='+str(b))
 write('tracing_on',0);trace=(instance/'trace').read_text();(base/'function-selftest.trace').write_text(trace)
 r['trace_head']=trace.splitlines()[:40];r['stats']={p.parent.name:p.read_text() for p in (instance/'per_cpu').glob('cpu*/stats')}
 r['getpid_lines']=[l for l in trace.splitlines() if '__x64_sys_getpid <-' in l];r['sleep_lines']=[l for l in trace.splitlines() if '__x64_sys_clock_nanosleep <-' in l]
 assert len(r['getpid_lines'])>=8 and len(r['sleep_lines'])==1
 marks=[(float(t),int(n)) for t,n in re.findall(r'(\d+\.\d+):\s+tracing_mark_write: MP2_SYNTHETIC_\w+ mono=(\d+)',trace)]
 assert len(marks)==2 and .030<marks[1][0]-marks[0][0]<.2
 assert all(-2000<int(ts*1e9)-mono<2_000_000 for ts,mono in marks),marks
 r['marker_clock_bound_ns']=[int(ts*1e9)-mono for ts,mono in marks]
 for v in r['stats'].values():
  for k in ['overrun','commit overrun','dropped events']:
   match=re.search(r'^'+re.escape(k)+r':\s*(\d+)',v,re.M)
   if match:assert int(match.group(1))==0,v
 r.update(passed=True,finished_at=time.time())
except BaseException as e:r.update(passed=False,failure=repr(e),finished_at=time.time());raise
finally:
 if instance.exists():write('tracing_on',0);write('current_tracer','nop');instance.rmdir()
 r['after']=global_state();r['global_state_changed']=r['after']!=before
 (base/'function-selftest.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r));assert not r['global_state_changed']
