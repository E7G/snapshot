"""Strict scalar frame-arrival attribution. No payload reads or timing rewrites.

Inputs are completed control-tracer intervals and encoder-input PTS/offset keys
selected by an independent recording/EOS observation. Output is aggregate-only:
no per-frame identifiers, thread IDs, owner tokens, raw paths or imagery.
"""
from collections import defaultdict

NONE = (1 << 64) - 1
GUARD = 'mipad2-capture-allocation-guard.GstPad:sink'
DIRECT = 'mipad2-native-raw-output.GstPad:src'


def require(condition, message):
    if not condition:
        raise ValueError(message)


def stats(values):
    values = sorted(values)
    require(bool(values), 'empty interval population')
    n = len(values)
    # Preserve V2's upper-middle/floor-index convention for exact regression.
    return dict(count=n, median_ns=values[n // 2],
                p95_ns=values[min(n - 1, int(.95 * n))],
                p99_ns=values[min(n - 1, int(.99 * n))],
                max_ns=values[-1], mean_ns=sum(values) / n,
                above30ms=sum(x > 30_000_000 for x in values))


def _key(value):
    require(len(value) == 2, 'frame key must have PTS and offset')
    require(all(type(x) is int and 0 <= x < NONE for x in value),
            'missing/invalid encoder frame identity')
    return tuple(value)


def _within(child, parent):
    return (child['tid'] == parent['tid'] and
            parent['begin'] <= child['begin'] <= child['end'] <= parent['end'])


def analyze(rows, encoder_input_keys, route):
    """Analyze every supplied encoder input exactly once, never nearest-match.

    route='multiqueue' reproduces V2's three source/MQ/encoder intervals.
    route='direct' requires a real named native-raw output push, no MQ on the
    matched frame, and a same-task nested source->guard->direct source tail.
    Downstream encoder work may be queued/on another task in either route.
    Unknown topology, ambiguous identities, missing stages and reversed clocks
    raise even under python -O. Non-OK flows are retained as aggregate counts.
    """
    require(route in ('multiqueue', 'direct'), 'unsupported route')
    keys = [_key(k) for k in encoder_input_keys]
    require(bool(keys), 'no encoder inputs')
    require(len(set(keys)) == len(keys), 'duplicate encoder input identity')
    indexed = defaultdict(list)
    for row in rows:
        if row.get('operation') not in ('push', 'chain'):
            continue
        for field in ('begin', 'end', 'elapsed_ns', 'tid', 'pts', 'offset'):
            require(type(row.get(field)) is int and row[field] >= 0,
                    'invalid interval field: ' + field)
        require(row['end'] >= row['begin'] and
                row['elapsed_ns'] == row['end'] - row['begin'],
                'inconsistent completed interval clock')
        require(isinstance(row.get('path'), str), 'invalid interval path')
        require(type(row.get('result')) is int, 'missing interval flow result')
        indexed[(row['pts'], row['offset'])].append(row)

    populations = defaultdict(list)
    non_ok = defaultdict(int)
    source_encoder_different = source_return_before_encoder = 0
    mq_source_different = mq_encoder_different = mq_encoder_nested = 0

    def select(frame, operation, klass, suffix=None):
        selected = [x for x in frame if x['operation'] == operation and
                    x['path'].startswith(klass + '\t') and
                    (suffix is None or x['path'].endswith(suffix))]
        require(len(selected) == 1, 'missing or ambiguous ' + klass + ' ' + operation)
        return selected[0]

    def account(name, call):
        non_ok[name] += int(call['result'] != 0)

    for key in keys:
        frame = indexed[key]
        source = select(frame, 'push', 'GstV4l2Src')
        guard = select(frame, 'chain', 'GstIdentity', GUARD)
        encoder = select(frame, 'chain', 'GstVaH264Enc', '.GstPad:sink')
        require(_within(guard, source), 'guard is not nested on native source task')
        require(source['begin'] <= encoder['begin'], 'encoder precedes native source')
        for name, call in [('native-source', source), ('allocation-guard', guard),
                           ('encoder-input', encoder)]:
            account(name, call)
        populations['full_native_push'].append(source['elapsed_ns'])
        populations['pre_guard_prefix'].append(guard['begin'] - source['begin'])
        populations['guard_downstream_chain'].append(guard['elapsed_ns'])
        populations['native_push_to_encoder_chain'].append(encoder['begin'] - source['begin'])
        source_encoder_different += int(source['tid'] != encoder['tid'])
        source_return_before_encoder += int(source['end'] < encoder['begin'])

        if route == 'multiqueue':
            sink = select(frame, 'chain', 'GstMultiQueue')
            output = select(frame, 'push', 'GstMultiQueue')
            require(source['begin'] <= sink['begin'] <= output['begin'] <= encoder['begin'],
                    'multiqueue arrival order reversed')
            intervals = [sink['begin'] - source['begin'],
                         output['begin'] - sink['begin'],
                         encoder['begin'] - output['begin']]
            names = ['source_to_multiqueue_sink_arrival',
                     'multiqueue_sink_arrival_to_src_push_begin',
                     'multiqueue_src_push_to_encoder_arrival']
            populations['multiqueue_worker_chain'].append(sink['elapsed_ns'])
            mq_source_different += int(sink['tid'] != source['tid'])
            mq_encoder_different += int(output['tid'] != encoder['tid'])
            mq_encoder_nested += int(_within(encoder, output))
            account('multiqueue-sink', sink)
            account('multiqueue-output', output)
        else:
            require(not any(x['path'].startswith('GstMultiQueue\t') for x in frame),
                    'multiqueue observed on claimed direct matched frame')
            output = select(frame, 'push', 'GstCapsFilter', DIRECT)
            require(_within(output, guard), 'direct output is not nested on native guard task')
            require(output['begin'] <= encoder['begin'], 'encoder precedes direct output')
            intervals = [guard['begin'] - source['begin'],
                         output['begin'] - guard['begin'],
                         encoder['begin'] - output['begin']]
            names = ['source_to_guard_arrival', 'guard_to_direct_output_push_begin',
                     'direct_output_push_to_encoder_arrival']
            account('direct-output', output)
        require(sum(intervals) == encoder['begin'] - source['begin'],
                'per-frame arrival partition does not conserve total')
        for name, value in zip(names, intervals):
            populations[name].append(value)

    report = {name: stats(values) for name, values in populations.items()}
    report.update(route=route, encoder_frames=len(keys),
                  source_encoder_different_tasks=source_encoder_different,
                  native_push_return_before_encoder_arrival=source_return_before_encoder,
                  non_ok_flow_counts=dict(non_ok), all_frames_partitioned_exactly=True,
                  no_payload_read=True, aggregate_only=True,
                  percentile_convention='V2 upper-middle median and floor-index quantiles; percentile sums are not additive',
                  scope='Arrival intervals and nested call wall times only, not CPU/memcpy time, isolated queue residence, mutex attribution, optical latency, frame-content conservation or driver ownership. All inputs included; no warmup removal.')
    if route == 'multiqueue':
        report.update(different_source_and_multiqueue_tasks=mq_source_different,
                      multiqueue_src_encoder_different_tasks=mq_encoder_different,
                      multiqueue_src_encoder_exact_nested_same_task=mq_encoder_nested)
    return report
