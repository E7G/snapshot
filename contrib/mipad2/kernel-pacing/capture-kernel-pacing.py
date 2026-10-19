"""Bounded own tracefs instance. No shared options, probes or global writes."""
from pathlib import Path
import os,time,json,re,signal,traceback
assert os.geteuid()==0
root=Path('/sys/kernel/tracing');base=Path('/var/tmp/mipad2-kernel-pacing-soak-20261019')
report=base/'capture.json';stop=base/'capture.stop';instance=root/'instances/mipad2-kernel-pacing-soak-20261019'
assert not report.exists() and not stop.exists() and not instance.exists()
keys=['current_tracer','tracing_on','trace_clock','trace_options','set_ftrace_filter','set_ftrace_notrace','set_graph_function','set_graph_notrace']
def global_state():
 return dict(files={k:(root/k).read_text().strip() for k in keys if (root/k).is_file()},options={p.name:p.read_text().strip() for p in (root/'options').iterdir() if p.name.startswith('funcgraph-') or p.name in ['sleep-time','graph-time']},kprobes=(root/'kprobe_events').read_text().strip())
before=global_state();assert before['files']['current_tracer']=='nop' and before['kprobes']==''
r=dict(pid=os.getpid(),boot_id=Path('/proc/sys/kernel/random/boot_id').read_text().strip(),phase='preparing',started_at=time.time(),before=before,no_global_or_trace_options_writes=True)
def save():
 p=report.with_suffix('.next');p.write_text(json.dumps(r,indent=2)+'\n');p.replace(report)
def write(name,value):(instance/name).write_text(str(value)+'\n')
requested=False
def end(signum,frame):
 global requested
 requested=True;r['signal']=signum
for sig in [signal.SIGTERM,signal.SIGHUP,signal.SIGINT]:signal.signal(sig,end)
save()
try:
 irq=int(Path('/sys/bus/pci/devices/0000:00:03.0/irq').read_text());assert irq>0
 irq_lines=[l for l in Path('/proc/interrupts').read_text().splitlines() if l.split(':',1)[0].strip()==str(irq)];assert len(irq_lines)==1,irq_lines
 r['irq']=irq;r['interrupt_line']=irq_lines[0]
 threads=[]
 for p in Path('/proc').iterdir():
  if not p.name.isdigit():continue
  try:
   comm=(p/'comm').read_text().strip()
   if comm.startswith('irq/'+str(irq)+'-'):threads.append((int(p.name),comm))
  except (FileNotFoundError,ProcessLookupError):pass
 assert len(threads)==1,threads;tid,comm=threads[0];r['irq_thread']=dict(pid=tid,comm=comm)
 funcs=[l.split()[0] for l in (root/'available_filter_functions').read_text().splitlines()]
 chosen=[v for v in funcs if v in ['atomisp_isr','atomisp_isr_thread','atomisp_buf_done','vb2_buffer_done'] or v.startswith(('atomisp_qbuf_wrapper.','atomisp_dqbuf_wrapper.','atomisp_s_ctrl.'))]
 assert set(['atomisp_isr','atomisp_isr_thread','atomisp_buf_done','vb2_buffer_done'])<=set(chosen)
 r['functions']=chosen
 instance.mkdir();write('tracing_on',0);write('buffer_size_kb',16384);write('trace_clock','mono');write('set_ftrace_filter','\n'.join(chosen));write('current_tracer','function')
 assert set(l.split()[0] for l in (instance/'set_ftrace_filter').read_text().splitlines())==set(chosen)
 events={'irq/irq_handler_entry':'irq == '+str(irq),'irq/irq_handler_exit':'irq == '+str(irq),'sched/sched_waking':'pid == '+str(tid),'sched/sched_wakeup':'pid == '+str(tid),'sched/sched_switch':'prev_pid == '+str(tid)+' || next_pid == '+str(tid)}
 r['event_filters']={}
 for event,flt in events.items():
  p=instance/'events'/event;assert (p/'filter').is_file();(p/'filter').write_text(flt+'\n');(p/'enable').write_text('1\n');r['event_filters'][event]=(p/'filter').read_text().strip()
 assert global_state()==before
 (instance/'trace').write_text('');write('tracing_on',1)
 r.update(phase='capturing',active_at=time.time(),instance=str(instance),buffer_total_kb=(instance/'buffer_total_size_kb').read_text().strip());save()
 mono=time.monotonic_ns();write('trace_marker','MP2_KERNEL_BEGIN mono='+str(mono))
 deadline=time.monotonic()+600
 while not requested and not stop.exists() and time.monotonic()<deadline:time.sleep(.25)
 r['stop_reason']='signal' if requested else 'requested' if stop.exists() else 'bounded-timeout'
 mono=time.monotonic_ns();write('trace_marker','MP2_KERNEL_END mono='+str(mono));write('tracing_on',0)
 r['stats']={p.parent.name:p.read_text() for p in (instance/'per_cpu').glob('cpu*/stats')}
 r['loss_counts']={}
 for cpu,stats in r['stats'].items():
  r['loss_counts'][cpu]={}
  for key in ['overrun','commit overrun','dropped events']:
   m=re.search(r'^'+re.escape(key)+r':\s*(\d+)',stats,re.M);assert m,(cpu,key,stats);r['loss_counts'][cpu][key]=int(m.group(1))
 trace=(instance/'trace').read_text();path=base/'capture.trace';assert not path.exists();path.write_text(trace)
 r['trace_bytes']=path.stat().st_size;r['trace_lines']=len(trace.splitlines());r['loss_gate']=all(v==0 for c in r['loss_counts'].values() for v in c.values())
 r.update(phase='captured',captured_at=time.time());save();assert r['loss_gate'],r['loss_counts']
except BaseException as e:
 r.update(phase='failed',failure=repr(e),exception=traceback.format_exc());save();raise
finally:
 if instance.exists():
  write('tracing_on',0);write('events/enable',0);write('current_tracer','nop');instance.rmdir()
 r['after']=global_state();r['global_state_changed']=r['after']!=before;r['instance_removed']=not instance.exists();r['terminal']=True;r['finished_at']=time.time();save()
 assert not r['global_state_changed']
