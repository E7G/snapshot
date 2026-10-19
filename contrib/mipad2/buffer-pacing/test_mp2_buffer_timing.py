import unittest
from mp2_buffer_timing import fields,parse_trace,identify_owners,zero_intervals,analyze
def row(name,t,**f):return dict(kind='event',name=name,ts=t,pid=9,cpu=1,fields={k:str(v) for k,v in f.items()})
def vb(name,t,owned=0,stamp=1234567,index=1):return row(name,t,owner='opaque',queued=2,owned_by_drv=owned,timestamp=stamp,index=index,type=1)
class Tests(unittest.TestCase):
 def test_space_comma(self):self.assertEqual(fields('minor = 0, index = 4, timestamp = 1000')['index'],'4')
 def test_nested_type_does_not_override(self):
  f=fields('type = VIDEO_CAPTURE, flags = MAPPED|DONE, timecode = { type = 30FPS, flags = 0 }, sequence = 7')
  self.assertEqual(f['type'],'VIDEO_CAPTURE');self.assertEqual(f['flags'],'MAPPED|DONE');self.assertEqual(f['sequence'],'7')
 def test_owner_join_truncates_us(self):
  a=identify_owners([vb('vb2_dqbuf',10),row('v4l2_dqbuf',11,index=1,timestamp=1234000,minor=0)])
  self.assertEqual(a['owners']['opaque'],1)
 def test_missing_owner(self):self.assertEqual(identify_owners([row('v4l2_dqbuf',11,index=1,timestamp=1234000,minor=0)])['missing'],1)
 def test_ambiguous_owner(self):
  a=identify_owners([vb('vb2_dqbuf',10),vb('vb2_dqbuf',9),row('v4l2_dqbuf',11,index=1,timestamp=1234000,minor=0)]);self.assertEqual(a['ambiguous'],1)
 def test_wrong_minor(self):self.assertEqual(identify_owners([vb('vb2_dqbuf',10),row('v4l2_dqbuf',11,index=1,timestamp=1234000,minor=2)])['unique'],0)
 def test_zero_interval(self):
  z=zero_intervals([vb('vb2_buf_done',1,0),vb('vb2_dqbuf',2,0),vb('vb2_buf_queue',9,1)]);self.assertEqual(z['intervals'][0]['duration_ns'],8)
 def test_zero_censored(self):self.assertEqual(zero_intervals([vb('vb2_buf_done',1,0)])['right_censored_begin'],1)
 def test_no_zero(self):self.assertFalse(zero_intervals([vb('vb2_buf_done',1,2)])['intervals'])
 def test_anomaly_join(self):
  rs=[vb('vb2_buf_done',1000,0,1000),vb('vb2_dqbuf',2000,0,1000),row('v4l2_dqbuf',3000,index=1,timestamp=1000,minor=0),vb('vb2_buf_queue',50000,1,1000),vb('vb2_buf_done',79000,0,79000)]
  a=analyze(rs,[dict(cycle=0,values={'driver-prev':1000,'driver':79000})]);self.assertEqual(a['anomalies'][0]['observed_zero_driver_owned_ns'],49000);self.assertFalse(a['owner_tokens_exported'])
 def test_not_nearest_stamp(self):
  rs=[vb('vb2_dqbuf',1),row('v4l2_dqbuf',2,index=1,timestamp=1234000,minor=0)];a=analyze(rs,[dict(cycle=0,values={'driver-prev':5000,'driver':79000})]);self.assertEqual(a['anomalies'][0]['before_matches'],0)
if __name__=='__main__':unittest.main()
