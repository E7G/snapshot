import copy,json,unittest
from mipad2_raw_tail_arrivals import analyze
from test_mipad2_frame_arrivals_v3 import call

def raw_frame(route):
    rows=[call('push','GstV4l2Src','native.GstPad:src',0,100),
          call('chain','GstIdentity','mipad2-capture-allocation-guard.GstPad:sink',20,90),
          call('push','GstIdentity','mipad2-capture-allocation-guard.GstPad:src',25,85),
          call('chain','GstVaH264Enc','encoder.GstPad:sink',100,105,4)]
    if route=='raw-direct':
        return rows+[call('push','GstCapsFilter','mipad2-native-raw-output.GstPad:src',40,80)]
    return rows+[call('chain','GstQueue','mipad2-native-raw-queue.GstPad:sink',30,70),
                 call('push','GstQueue','mipad2-native-raw-queue.GstPad:src',60,80,2),
                 call('push','GstCapsFilter','mipad2-native-raw-output.GstPad:src',65,75,2)]

class TailArrivals(unittest.TestCase):
    def test_real_direct_witness_partition(self):
        r=analyze(raw_frame('raw-direct'),[(10,2)],'raw-direct')
        self.assertEqual([r[x]['median_ns'] for x in ['source_to_guard_arrival','guard_arrival_to_raw_output_begin','raw_output_push_to_encoder_arrival']],[20,20,60])
        self.assertTrue(r['route_topology_proven'])
        self.assertEqual(r['actual_named_output_witnesses'],1)

    def test_queue_witness_partition_allows_overlap_and_async_encoder(self):
        r=analyze(raw_frame('raw-async-named-queue'),[(10,2)],'raw-async-named-queue')
        self.assertEqual([r[x]['median_ns'] for x in ['source_to_native_queue_sink_arrival','native_queue_sink_arrival_to_worker_push_begin','native_queue_worker_push_to_raw_output_begin','raw_output_push_to_encoder_arrival']],[30,30,5,35])
        self.assertEqual(r['native_worker_begins_before_source_return'],1)
        self.assertEqual(r['native_worker_encoder_different_tasks'],1)
        self.assertFalse(r['queue_limit_proven_by_timing_trace'])

    def test_worker_and_encoder_may_run_after_source_return(self):
        rows=raw_frame('raw-async-named-queue')
        rows[5].update(begin=200,end=230,elapsed_ns=30);rows[6].update(begin=210,end=220,elapsed_ns=10);rows[3].update(begin=1000,end=1005,elapsed_ns=5)
        r=analyze(rows,[(10,2)],'raw-async-named-queue')
        self.assertEqual(r['raw_output_begins_after_native_return'],1)
        self.assertEqual(r['native_push_return_before_encoder_arrival'],1)

    def test_missing_or_duplicate_witness_rejected(self):
        for route in ['raw-direct','raw-async-named-queue']:
            rows=raw_frame(route)
            for i in range(len(rows)):
                with self.subTest(route=route,index=i):
                    with self.assertRaises(ValueError):analyze(rows[:i]+rows[i+1:],[(10,2)],route)
                    with self.assertRaises(ValueError):analyze(rows+[copy.deepcopy(rows[i])],[(10,2)],route)

    def test_wrong_tail_flags_do_not_replace_actual_witnesses(self):
        with self.assertRaises(ValueError):analyze(raw_frame('raw-direct'),[(10,2)],'raw-async-named-queue')
        with self.assertRaises(ValueError):analyze(raw_frame('raw-async-named-queue'),[(10,2)],'raw-direct')
        with self.assertRaises(ValueError):analyze(raw_frame('raw-direct'),[(10,2)],'guess')

    def test_wrong_named_output_does_not_accept_any_capsfilter(self):
        rows=raw_frame('raw-direct');rows[-1]['path']=rows[-1]['path'].replace('mipad2-native-raw-output','unrelated')
        with self.assertRaises(ValueError):analyze(rows,[(10,2)],'raw-direct')

    def test_wrong_worker_or_nesting_rejected(self):
        for index,change in [(4,dict(tid=2)),(5,dict(tid=1)),(6,dict(tid=1)),(6,dict(begin=50,end=55,elapsed_ns=5)),(2,dict(tid=3))]:
            rows=raw_frame('raw-async-named-queue');rows[index].update(change)
            with self.subTest(index=index,change=change):
                with self.assertRaises(ValueError):analyze(rows,[(10,2)],'raw-async-named-queue')

    def test_source_multiqueue_cannot_hide_in_claimed_raw_route(self):
        rows=raw_frame('raw-direct')+[call('chain','GstMultiQueue','/GstWrapperCameraBinSrc:source/decode.GstPad:sink_0',30,35)]
        with self.assertRaisesRegex(ValueError,'source multiqueue'):analyze(rows,[(10,2)],'raw-direct')

    def test_reversed_order_never_clamped(self):
        rows=raw_frame('raw-async-named-queue');rows[5].update(begin=10,end=80,elapsed_ns=70)
        with self.assertRaises(ValueError):analyze(rows,[(10,2)],'raw-async-named-queue')

    def test_non_ok_flows_retained(self):
        rows=raw_frame('raw-async-named-queue');rows[4]['result']=-2;rows[6]['result']=-5
        r=analyze(rows,[(10,2)],'raw-async-named-queue')
        self.assertEqual(r['tail_non_ok_flow_counts']['native-queue-sink'],1)
        self.assertEqual(r['tail_non_ok_flow_counts']['named-raw-output'],1)

    def test_all_frames_and_aggregate_only(self):
        rows=[];keys=[]
        for i in range(12):
            keys.append((10+i,2+i))
            for r in raw_frame('raw-async-named-queue'):
                r.update(pts=10+i,offset=2+i,begin=r['begin']+i*1000,end=r['end']+i*1000);rows.append(r)
        out=analyze(rows,keys,'raw-async-named-queue');self.assertEqual(out['encoder_frames'],12)
        self.assertTrue(all(v['count']==12 for v in out.values() if isinstance(v,dict) and 'count' in v))
        for forbidden in ['fixture/','"tid"','"pts"','"offset"','"path"']:
            self.assertNotIn(forbidden,json.dumps(out))

    def test_duplicate_or_unmatched_identity_and_invalid_clock_rejected(self):
        for keys in [[],[(11,2)],[(10,2),(10,2)],[(10,-1)]]:
            with self.assertRaises(ValueError):analyze(raw_frame('raw-direct'),keys,'raw-direct')
        rows=raw_frame('raw-direct');rows[4]['elapsed_ns']=999
        with self.assertRaises(ValueError):analyze(rows,[(10,2)],'raw-direct')

if __name__=='__main__':unittest.main(verbosity=2)
