import unittest
from mp2_kernel_timing import ns,parse_trace,completion_brackets,match_stamp,scheduler_intervals,clip_intervals,baseline
def line(stamp,body,pid=528,cpu=1,comm='irq/146-isp_irq'):
 return f' {comm}-{pid} [{cpu:03}] ..... {stamp}: {body}\n'
class Tests(unittest.TestCase):
 def test_timestamp_exact(self):self.assertEqual(ns('12345.123456'),12345123456000)
 def test_reject_precision(self):
  with self.assertRaises(ValueError):ns('0.1234567891')
 def test_parse_function(self):
  r=parse_trace(line('1.000001','atomisp_buf_done <-atomisp_isr_thread [atomisp]'))
  self.assertTrue(r['syntax_complete']);self.assertEqual(r['records'][0]['pid'],528)
 def test_unknown_not_accepted(self):self.assertFalse(parse_trace('garbage')['syntax_complete'])
 def test_lost_not_accepted(self):self.assertFalse(parse_trace('CPU:1 [LOST 4 EVENTS]')['syntax_complete'])
 def test_bracket(self):
  t=parse_trace(line('1.000001','atomisp_buf_done <-atomisp_isr_thread')+line('1.000004','vb2_buffer_done <-atomisp_buf_done'))
  b=completion_brackets(t['records']);self.assertEqual(match_stamp(1000002000,b['brackets'])['matches'],1)
 def test_ambiguous(self):self.assertIsNone(match_stamp(2,[dict(begin=1,end=3),dict(begin=2,end=4)])['bracket'])
 def test_censored(self):
  b=completion_brackets(parse_trace(line('1.000001','atomisp_buf_done <-atomisp_isr_thread'))['records'])
  self.assertEqual(len(b['unmatched_atomisp_entry']),1);self.assertEqual(len(b['brackets']),0)
 def test_scheduler(self):
  t=parse_trace(line('1.000000','sched_switch: prev_comm=irq/146-isp_irq prev_pid=528 prev_prio=49 prev_state=D ==> next_comm=snapshot next_pid=99 next_prio=120')+line('1.030000','sched_waking: comm=irq/146-isp_irq pid=528 prio=49 target_cpu=001',pid=99,comm='snapshot')+line('1.031000','sched_switch: prev_comm=snapshot prev_pid=99 prev_prio=120 prev_state=R ==> next_comm=irq/146-isp_irq next_pid=528 next_prio=49'))
  s=scheduler_intervals(t['records'],528);self.assertEqual(s['intervals'][0]['post_wake_delay_ns'],1000000);self.assertEqual(s['intervals'][0]['state'],'D')
 def test_overlap(self):self.assertEqual(clip_intervals([dict(out=1,back=7)],3,5)[0]['overlap_ns'],2)
 def test_empty_baseline(self):self.assertEqual(baseline([],528)['video_completion_brackets_in_irq_thread'],0)
 def test_baseline(self):
  t=parse_trace(line('1.000001','atomisp_isr_thread <-irq_thread_fn')+line('1.000003','atomisp_buf_done <-atomisp_isr_thread')+line('1.000004','vb2_buffer_done <-atomisp_buf_done'))
  b=baseline(t['records'],528);self.assertEqual(b['handler_to_completion_entry_p50_ns'],2000)
 def test_baseline_offcpu(self):
  t=parse_trace(line('1.000000','atomisp_isr_thread <-irq_thread_fn')+line('1.000001','sched_switch: prev_comm=irq/146-isp_irq prev_pid=528 prev_prio=49 prev_state=D ==> next_comm=snapshot next_pid=99 next_prio=120')+line('1.030000','sched_switch: prev_comm=snapshot prev_pid=99 prev_prio=120 prev_state=R ==> next_comm=irq/146-isp_irq next_pid=528 next_prio=49')+line('1.031000','atomisp_buf_done <-atomisp_isr_thread')+line('1.031002','vb2_buffer_done <-atomisp_buf_done'))
  b=baseline(t['records'],528);self.assertEqual(b['longest_handler_spans'][0]['offcpu_overlap_ns'],29999000)
if __name__=='__main__':unittest.main()
