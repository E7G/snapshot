import copy
import unittest

from mipad2_frame_arrivals_v3 import analyze, stats, NONE


def call(op, kind, pad, begin, end, tid=1, pts=10, offset=2, result=0):
    return dict(operation=op, path=kind+'\tfixture/'+pad, begin=begin, end=end,
                elapsed_ns=end-begin, tid=tid, pts=pts, offset=offset, result=result)


def frame(route):
    common = [call('push', 'GstV4l2Src', 'camera.GstPad:src', 0, 100),
              call('chain', 'GstIdentity', 'mipad2-capture-allocation-guard.GstPad:sink', 20, 90),
              call('chain', 'GstVaH264Enc', 'encoder.GstPad:sink', 90, 95, 4)]
    if route == 'direct':
        return common+[call('push', 'GstCapsFilter', 'mipad2-native-raw-output.GstPad:src', 40, 80)]
    return common+[call('chain', 'GstMultiQueue', 'decode.GstPad:sink_0', 30, 50, 2),
                   call('push', 'GstMultiQueue', 'decode.GstPad:src_0', 60, 70, 3)]


class Arrivals(unittest.TestCase):
    def test_multiqueue_v2_partitions_async_downstream(self):
        r=analyze(frame('multiqueue'), [(10, 2)], 'multiqueue')
        self.assertEqual([r[n]['median_ns'] for n in ['source_to_multiqueue_sink_arrival','multiqueue_sink_arrival_to_src_push_begin','multiqueue_src_push_to_encoder_arrival']], [30,30,30])
        self.assertEqual(r['multiqueue_src_encoder_different_tasks'], 1)
        self.assertEqual(r['multiqueue_src_encoder_exact_nested_same_task'], 0)

    def test_direct_partition_and_no_invented_mq(self):
        r=analyze(frame('direct'), [(10,2)], 'direct')
        self.assertEqual([r[n]['median_ns'] for n in ['source_to_guard_arrival','guard_to_direct_output_push_begin','direct_output_push_to_encoder_arrival']], [20,20,50])
        self.assertFalse(any('multiqueue' in key for key in r))
        self.assertEqual(r['source_encoder_different_tasks'], 1)

    def test_downstream_may_run_after_native_return(self):
        rows=frame('direct');rows[2].update(begin=1000,end=1005,elapsed_ns=5)
        r=analyze(rows,[(10,2)],'direct')
        self.assertEqual(r['native_push_to_encoder_chain']['max_ns'], 1000)
        self.assertEqual(r['native_push_return_before_encoder_arrival'], 1)

    def test_multiqueue_can_run_after_source_return(self):
        rows=frame('multiqueue');rows[0].update(end=95,elapsed_ns=95)
        rows[3].update(begin=100,end=110,elapsed_ns=10);rows[4].update(begin=120,end=130,elapsed_ns=10)
        rows[2].update(begin=140,end=145,elapsed_ns=5)
        self.assertEqual(analyze(rows,[(10,2)],'multiqueue')['native_push_return_before_encoder_arrival'],1)

    def test_no_assumed_mq_encoder_task_change(self):
        rows=frame('multiqueue');rows[2].update(begin=65,end=68,elapsed_ns=3,tid=3)
        r=analyze(rows,[(10,2)],'multiqueue')
        self.assertEqual(r['multiqueue_src_encoder_different_tasks'],0)
        self.assertEqual(r['multiqueue_src_encoder_exact_nested_same_task'],1)

    def test_non_ok_flow_is_not_hidden(self):
        rows=frame('direct');rows[0]['result']=-2;rows[2]['result']=-5
        r=analyze(rows,[(10,2)],'direct')
        self.assertEqual(r['non_ok_flow_counts']['native-source'],1)
        self.assertEqual(r['non_ok_flow_counts']['encoder-input'],1)

    def test_aggregate_only(self):
        r=analyze(frame('direct'),[(10,2)],'direct')
        encoded=__import__('json').dumps(r)
        for forbidden in ['fixture/', 'GstV4l2Src', '"tid"', '"pts"', '"offset"', '"path"']:
            self.assertNotIn(forbidden,encoded)

    def test_all_frames_including_first_and_last(self):
        rows=[];keys=[]
        for i in range(12):
            keys.append((100+i,20+i))
            for c in frame('direct'):
                c.update(pts=100+i,offset=20+i,begin=c['begin']+i*1000,end=c['end']+i*1000)
                rows.append(c)
        r=analyze(rows,keys,'direct')
        self.assertEqual(r['encoder_frames'],12)
        self.assertTrue(all(v['count']==12 for v in r.values() if isinstance(v,dict) and 'count' in v))

    def test_duplicate_input(self):
        with self.assertRaisesRegex(ValueError,'duplicate encoder'):
            analyze(frame('direct'),[(10,2),(10,2)],'direct')

    def test_missing_or_ambiguous_stages(self):
        for route in ['direct','multiqueue']:
            rows=frame(route)
            for i in range(len(rows)):
                with self.subTest(route=route,index=i,change='missing'):
                    with self.assertRaises(ValueError):analyze(rows[:i]+rows[i+1:],[(10,2)],route)
                with self.subTest(route=route,index=i,change='duplicate'):
                    with self.assertRaises(ValueError):analyze(rows+[copy.deepcopy(rows[i])],[(10,2)],route)

    def test_unmatched_identity_does_not_nearest_match(self):
        with self.assertRaises(ValueError):analyze(frame('direct'),[(11,2)],'direct')

    def test_reject_none_or_malformed_input(self):
        for keys in [[],[(NONE,2)],[(10,NONE)],[(True,2)],[(10,)],[(10,'2')],[(10,-1)]]:
            with self.subTest(keys=keys):
                with self.assertRaises(ValueError):analyze(frame('direct'),keys,'direct')

    def test_invalid_interval_fields_and_clock(self):
        for field,value in [('begin',-1),('tid','1'),('pts',None),('elapsed_ns',999),('path',None),('result',None),('end',-1)]:
            rows=frame('direct');rows[0][field]=value
            with self.subTest(field=field):
                with self.assertRaises(ValueError):analyze(rows,[(10,2)],'direct')

    def test_guard_and_direct_must_be_nested_same_task(self):
        for index,change in [(1,dict(tid=9)),(3,dict(tid=9)),(3,dict(begin=10,end=15,elapsed_ns=5)),(3,dict(begin=80,end=95,elapsed_ns=15))]:
            rows=frame('direct');rows[index].update(change)
            with self.subTest(index=index,change=change):
                with self.assertRaises(ValueError):analyze(rows,[(10,2)],'direct')

    def test_wrong_or_hidden_source_route_rejected(self):
        rows=frame('direct')+frame('multiqueue')[3:]
        with self.assertRaisesRegex(ValueError,'multiqueue observed'):analyze(rows,[(10,2)],'direct')
        with self.assertRaises(ValueError):analyze(frame('multiqueue'),[(10,2)],'direct')
        with self.assertRaises(ValueError):analyze(frame('direct'),[(10,2)],'multiqueue')
        with self.assertRaises(ValueError):analyze(frame('direct'),[(10,2)],'guess')

    def test_reversed_arrival_is_not_clamped(self):
        for route in ['direct','multiqueue']:
            rows=frame(route);rows[2].update(begin=35,end=37,elapsed_ns=2)
            with self.assertRaises(ValueError):analyze(rows,[(10,2)],route)

    def test_wrong_named_pad_not_accepted(self):
        rows=frame('direct');rows[3]['path']=rows[3]['path'].replace('.GstPad:src','.GstPad:sink')
        with self.assertRaises(ValueError):analyze(rows,[(10,2)],'direct')

    def test_legacy_statistics_convention(self):
        values=list(range(1,101));r=stats(values)
        self.assertEqual((r['median_ns'],r['p95_ns'],r['p99_ns']),(51,96,100))
        with self.assertRaises(ValueError):stats([])


if __name__=='__main__':unittest.main(verbosity=2)
