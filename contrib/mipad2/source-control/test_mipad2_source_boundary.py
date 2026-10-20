import copy,json,unittest
from mipad2_source_boundary import analyze
from test_mipad2_frame_arrivals_v3 import frame

class Boundary(unittest.TestCase):
    def test_common_partition_for_both_routes(self):
        for route in ['direct','multiqueue']:
            r=analyze(frame(route),[(10,2)])
            self.assertEqual([r[n]['median_ns'] for n in ['pre_guard_prefix','guard_downstream_chain','post_guard_suffix']], [20,70,10])
            self.assertEqual(r['native_push_to_encoder_chain']['median_ns'],90)
            self.assertFalse(r['route_topology_proven'])

    def test_no_unseen_tail_required_or_invented(self):
        rows=frame('direct')[:3];r=analyze(rows,[(10,2)])
        self.assertEqual(r['encoder_frames'],1)
        self.assertFalse(r['route_topology_proven'])
        self.assertFalse(any('direct_output' in k or 'multiqueue' in k for k in r))

    def test_strict_route_still_rejects_missing_tail(self):
        from mipad2_frame_arrivals_v3 import analyze as route_analyze
        with self.assertRaises(ValueError):route_analyze(frame('direct')[:3],[(10,2)],'direct')

    def test_source_returns_before_async_encoder(self):
        rows=frame('direct')[:3];rows[2].update(begin=1000,end=1005,elapsed_ns=5)
        r=analyze(rows,[(10,2)])
        self.assertEqual(r['native_push_return_before_encoder_arrival'],1)
        self.assertEqual(r['encoder_nested_on_native_task'],0)

    def test_all_frames_and_thresholds(self):
        rows=[];keys=[]
        for i,n in enumerate([10_000_000,40_000_001,110_000_000]):
            start=i*1_000_000_000;keys.append((10+i,2+i))
            for j,c in enumerate(frame('direct')[:3]):
                if j==0:c.update(begin=start,end=start+n,elapsed_ns=n)
                if j==1:c.update(begin=start+1,end=start+n-1,elapsed_ns=n-2)
                if j==2:c.update(begin=start+2,end=start+3,elapsed_ns=1)
                c.update(pts=10+i,offset=2+i);rows.append(c)
        r=analyze(rows,keys)
        self.assertEqual(r['encoder_frames'],3)
        self.assertEqual(r['full_native_push']['above40ms'],2)
        self.assertEqual(r['full_native_push']['above100ms'],1)

    def test_missing_ambiguous_invalid_identity_rejected(self):
        rows=frame('direct')[:3]
        for value,keys in [(rows[:2],[(10,2)]),(rows+[copy.deepcopy(rows[0])],[(10,2)]),(rows,[(11,2)]),(rows,[(10,2),(10,2)]),(rows,[])]:
            with self.assertRaises(ValueError):analyze(value,keys)

    def test_clock_nesting_order_and_type_rejected(self):
        for index,change in [(0,dict(elapsed_ns=999)),(1,dict(tid=9)),(2,dict(begin=10,end=15,elapsed_ns=5)),(0,dict(tid=True)),(0,dict(result=None))]:
            rows=frame('direct')[:3];rows[index].update(change)
            with self.assertRaises(ValueError):analyze(rows,[(10,2)])

    def test_no_raw_identifiers_non_ok_retained(self):
        rows=frame('direct')[:3];rows[0]['result']=-2
        r=analyze(rows,[(10,2)])
        self.assertEqual(r['non_ok_flow_counts']['native-source'],1)
        encoded=json.dumps(r)
        for forbidden in ['fixture/','"tid"','"pts"','"offset"','"path"']:
            self.assertNotIn(forbidden,encoded)

if __name__=='__main__':unittest.main(verbosity=2)
