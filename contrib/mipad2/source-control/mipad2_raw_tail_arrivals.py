"""Strict named native-raw tail witnesses; aggregate metadata only.

Recording/EOS selection and complete-interval qualification belong to the caller.
No absent output pad, queue, or task boundary is guessed from a route flag.
"""
from collections import defaultdict
from mipad2_frame_arrivals_v3 import require, stats, _key, _within
from mipad2_source_boundary import analyze as source_boundary

GUARD_SRC='mipad2-capture-allocation-guard.GstPad:src'
RAW_OUTPUT='mipad2-native-raw-output.GstPad:src'
QUEUE_SINK='mipad2-native-raw-queue.GstPad:sink'
QUEUE_SRC='mipad2-native-raw-queue.GstPad:src'


def analyze(rows, encoder_input_keys, route):
    require(route in ('raw-direct','raw-async-named-queue'), 'unsupported raw tail route')
    keys=[_key(k) for k in encoder_input_keys]
    report=source_boundary(rows,keys)  # validates identities, clocks and common stages
    indexed=defaultdict(list)
    for row in rows:
        if row.get('operation') in ('push','chain'):
            indexed[(row['pts'],row['offset'])].append(row)
    populations=defaultdict(list); non_ok=defaultdict(int)
    worker_before_source_return=worker_encoder_different=output_after_source_return=0
    for key in keys:
        frame=indexed[key]
        def select(operation, klass, suffix):
            matches=[r for r in frame if r['operation']==operation and
                     r['path'].startswith(klass+'\t') and r['path'].endswith(suffix)]
            require(len(matches)==1, 'missing or ambiguous named '+klass+' '+suffix)
            return matches[0]
        source=select('push','GstV4l2Src','.GstPad:src')
        guard=select('chain','GstIdentity','mipad2-capture-allocation-guard.GstPad:sink')
        guard_src=select('push','GstIdentity',GUARD_SRC)
        output=select('push','GstCapsFilter',RAW_OUTPUT)
        encoder=select('chain','GstVaH264Enc','.GstPad:sink')
        require(_within(guard_src,guard), 'guard output not nested on guard task')
        require(not any(r['path'].startswith('GstMultiQueue\t') and
                        '/GstWrapperCameraBinSrc:' in r['path'] for r in frame),
                'source multiqueue observed on claimed raw tail')
        require(output['begin']<=encoder['begin'], 'encoder precedes named raw output')
        if route=='raw-direct':
            require(not any(r['path'].startswith('GstQueue\t') and
                            r['path'].endswith((QUEUE_SINK,QUEUE_SRC)) for r in frame),
                    'native queue observed on claimed synchronous route')
            require(_within(output,guard_src), 'direct output not nested on native guard task')
            cuts=[source['begin'],guard['begin'],output['begin'],encoder['begin']]
            names=['source_to_guard_arrival','guard_arrival_to_raw_output_begin',
                   'raw_output_push_to_encoder_arrival']
        else:
            sink=select('chain','GstQueue',QUEUE_SINK)
            worker=select('push','GstQueue',QUEUE_SRC)
            require(_within(sink,guard_src), 'native queue sink not nested on source guard output')
            require(worker['tid']!=source['tid'], 'native queue did not create a worker boundary')
            require(_within(output,worker), 'raw output not nested on native queue worker')
            # Worker may consume before sink chain returns. Do not invent a
            # sink-return -> worker-start ordering or synchronous encoder tail.
            cuts=[source['begin'],sink['begin'],worker['begin'],output['begin'],encoder['begin']]
            names=['source_to_native_queue_sink_arrival',
                   'native_queue_sink_arrival_to_worker_push_begin',
                   'native_queue_worker_push_to_raw_output_begin',
                   'raw_output_push_to_encoder_arrival']
            populations['native_queue_sink_chain'].append(sink['elapsed_ns'])
            populations['native_queue_worker_push'].append(worker['elapsed_ns'])
            worker_before_source_return+=int(worker['begin']<source['end'])
            worker_encoder_different+=int(worker['tid']!=encoder['tid'])
            non_ok['native-queue-sink']+=int(sink['result']!=0)
            non_ok['native-queue-worker']+=int(worker['result']!=0)
        values=[b-a for a,b in zip(cuts,cuts[1:])]
        require(all(v>=0 for v in values), 'raw tail arrival order reversed')
        require(sum(values)==encoder['begin']-source['begin'], 'raw tail per-frame partition mismatch')
        for name,value in zip(names,values):populations[name].append(value)
        populations['raw_output_push'].append(output['elapsed_ns'])
        populations['guard_src_push'].append(guard_src['elapsed_ns'])
        output_after_source_return+=int(output['begin']>source['end'])
        non_ok['named-raw-output']+=int(output['result']!=0)
        non_ok['guard-output']+=int(guard_src['result']!=0)
    report.update({n:dict(stats(v),above40ms=sum(x>40_000_000 for x in v),
                         above100ms=sum(x>100_000_000 for x in v)) for n,v in populations.items()})
    report.update(route=route,route_topology_proven=True,actual_named_output_witnesses=len(keys),
                  source_arrival_to_encoder_partitioned_exactly=True,
                  raw_output_begins_after_native_return=output_after_source_return,
                  tail_non_ok_flow_counts=dict(non_ok),queue_limit_proven_by_timing_trace=False,
                  scope='Exact named pad/type/task witnesses on every encoder-input identity. Arrival intervals and nested wall times only; queue sink-arrival->worker-start includes blocked enqueue, not isolated residence or CPU time. No queue-capacity, optical, unique-lock, driver-ownership or Android parity inference; no missing-stage fitting or warmup trimming.')
    if route=='raw-async-named-queue':
        report.update(native_worker_begins_before_source_return=worker_before_source_return,
                      native_worker_encoder_different_tasks=worker_encoder_different)
    return report
