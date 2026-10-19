"""Synthetic binary-ABI and explicit right-censoring regressions, no camera."""
import unittest, tempfile, struct
from pathlib import Path
from mp2_pacing_decode import HEADER, RECORD, DATA_START, NONE, read_trace, summarize

def fixture(records=None, reserved=None, overflow=0):
 if records is None:records=[(10,1),(20,3),(30,4),(50,2)]
 n=len(records);raw=bytearray(DATA_START+16*64)
 HEADER.pack_into(raw,0,b'MP2PACE1',1,64,16,n if reserved is None else reserved,1,overflow,0,11)
 name=b'GstIdentity\t/GstPipeline:test/GstIdentity:test.GstPad:sink'
 raw[64:64+len(name)]=name
 for i,(ts,op) in enumerate(records):
  RECORD.pack_into(raw,DATA_START+i*64,ts,123 if op in (1,3) else NONE,5,NONE,22,1,op,0,0,i+1)
 return raw

class DecodeTests(unittest.TestCase):
 def decode(self,raw):
  with tempfile.TemporaryDirectory() as temp:
   p=Path(temp)/'scalar.bin';p.write_bytes(raw);return read_trace(p)
 def test_balanced(self):
  t=self.decode(fixture());s=summarize(t)
  self.assertTrue(t['complete']);self.assertEqual(s['unmatched_pre'],0);self.assertEqual(s['unmatched_post'],0)
  self.assertEqual([p['call_p50_ns'] for p in s['pads']],[40,10])
 def test_right_censored_not_balanced(self):
  s=summarize(self.decode(fixture([(10,1)])))
  self.assertTrue(s['complete']) # Committed ring completeness is NOT lifecycle balance.
  self.assertEqual(s['unmatched_pre'],1);self.assertEqual(s['pads'][0]['paired_calls'],0)
  self.assertIsNone(s['pads'][0]['call_p50_ns'])
 def test_unmatched_post(self):
  s=summarize(self.decode(fixture([(50,2)])));self.assertEqual(s['unmatched_post'],1)
 def test_missing_commit(self):
  raw=fixture();struct.pack_into('<Q',raw,DATA_START+56,0)
  t=self.decode(raw);self.assertFalse(t['complete']);self.assertEqual(t['incomplete'],1)
 def test_overflow(self):
  t=self.decode(fixture(reserved=17,overflow=1));self.assertFalse(t['complete']);self.assertEqual(t['overflow'],1)
 def test_invalid_pad(self):
  raw=fixture();struct.pack_into('<I',raw,DATA_START+40,257)
  with self.assertRaises(ValueError):self.decode(raw)
 def test_invalid_opcode(self):
  raw=fixture();struct.pack_into('<I',raw,DATA_START+44,7)
  with self.assertRaises(ValueError):self.decode(raw)
 def test_bad_magic(self):
  raw=fixture();raw[:8]=b'BADMAGIC'
  with self.assertRaises(ValueError):self.decode(raw)
 def test_truncated(self):
  with self.assertRaises(ValueError):self.decode(fixture()[:-1])
 def test_negative_pair_duration(self):
  with self.assertRaises(ValueError):summarize(self.decode(fixture([(50,1),(10,2)])))

if __name__=='__main__':unittest.main()
