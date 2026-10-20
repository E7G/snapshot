"""Aggregate actual source/guard/encoder timing without assuming an unseen tail.

This deliberately does not prove direct-output-pad or multiqueue topology. The
strict route analyzer remains separate; a missing route witness is not fitted.
"""
from collections import defaultdict
from mipad2_frame_arrivals_v3 import require, stats, _key, _within


def analyze(rows, encoder_input_keys):
    keys=[_key(k) for k in encoder_input_keys]
    require(bool(keys), 'no encoder inputs')
    require(len(set(keys))==len(keys), 'duplicate encoder input identity')
    indexed=defaultdict(list)
    for row in rows:
        if row.get('operation') not in ('push','chain'):
            continue
        for field in ('begin','end','elapsed_ns','tid','pts','offset'):
            require(type(row.get(field)) is int and row[field]>=0,
                    'invalid interval field: '+field)
        require(row['end']>=row['begin'] and row['elapsed_ns']==row['end']-row['begin'],
                'inconsistent completed interval clock')
        require(isinstance(row.get('path'),str), 'invalid interval path')
        require(type(row.get('result')) is int, 'missing flow result')
        indexed[(row['pts'],row['offset'])].append(row)
    populations=defaultdict(list); non_ok=defaultdict(int)
    different=returned=nested=0
    for key in keys:
        frame=indexed[key]
        def select(operation, klass, suffix):
            found=[x for x in frame if x['operation']==operation and
                   x['path'].startswith(klass+'\t') and x['path'].endswith(suffix)]
            require(len(found)==1, 'missing or ambiguous '+klass+' '+operation)
            return found[0]
        source=select('push','GstV4l2Src','.GstPad:src')
        guard=select('chain','GstIdentity','mipad2-capture-allocation-guard.GstPad:sink')
        encoder=select('chain','GstVaH264Enc','.GstPad:sink')
        require(_within(guard,source), 'guard not nested on source task')
        require(source['begin']<=guard['begin']<=encoder['begin'], 'arrival order reversed')
        for name,call in [('native-source',source),('allocation-guard',guard),('encoder-input',encoder)]:
            non_ok[name]+=int(call['result']!=0)
        prefix=guard['begin']-source['begin']; suffix=source['end']-guard['end']
        require(prefix+guard['elapsed_ns']+suffix==source['elapsed_ns'], 'source call partition mismatch')
        guard_to_encoder=encoder['begin']-guard['begin']
        require(prefix+guard_to_encoder==encoder['begin']-source['begin'], 'encoder arrival partition mismatch')
        for name,value in [('full_native_push',source['elapsed_ns']),('pre_guard_prefix',prefix),
                           ('guard_downstream_chain',guard['elapsed_ns']),('post_guard_suffix',suffix),
                           ('native_push_to_encoder_chain',encoder['begin']-source['begin']),
                           ('guard_arrival_to_encoder_chain',guard_to_encoder)]:
            populations[name].append(value)
        different+=int(source['tid']!=encoder['tid'])
        returned+=int(source['end']<encoder['begin'])
        nested+=int(_within(encoder,source))
    result={name:dict(stats(values),above40ms=sum(x>40_000_000 for x in values),
                      above50ms=sum(x>50_000_000 for x in values),
                      above100ms=sum(x>100_000_000 for x in values))
            for name,values in populations.items()}
    result.update(encoder_frames=len(keys),source_encoder_different_tasks=different,
                  native_push_return_before_encoder_arrival=returned,
                  encoder_nested_on_native_task=nested,non_ok_flow_counts=dict(non_ok),
                  all_frames_partitioned_exactly=True,aggregate_only=True,no_payload_read=True,
                  route_topology_proven=False,
                  scope='Exact encoder-input keys; source/guard completed call wall times and encoder arrival only. Prefix includes everything before guard, not isolated memcpy CPU time. No unseen direct/MQ tail, optical continuity, unique lock identity or CPU-time inference. All frames retained.')
    return result
