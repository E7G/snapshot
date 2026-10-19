"""Own-instance source filters: exit first, commit only after verified readback.

Callbacks operate on an already created private trace instance. Never pass a
global tracefs root. Marker timestamps from the actual trace, not these clock
samples, define the valid syscall-pairing interval for each newly added source.
"""
import re
import time


def install_source_filters(write_filter, read_filter, marker, irq_tid, sources,
                           now_ns=time.monotonic_ns):
    sources = sorted(set(sources))
    if not sources or not all(isinstance(t, int) and t > 0 for t in sources):
        raise ValueError('nonempty positive source TIDs required')
    if not isinstance(irq_tid, int) or irq_tid <= 0:
        raise ValueError('positive IRQ TID required')
    begin = now_ns()
    members = ','.join(map(str, sources))
    marker('MP2_SOURCE_FILTER_BEGIN tids=' + members)
    syscall = ' || '.join('common_pid == ' + str(t) for t in sources)
    all_threads = sorted(set([irq_tid] + sources))
    wakes = ' || '.join('pid == ' + str(t) for t in all_threads)
    switches = ' || '.join('prev_pid == ' + str(t) + ' || next_pid == ' + str(t)
                          for t in all_threads)
    # A new source may have left-censored exits during installation. Never
    # enable its entries before its exits, and never temporarily remove old TIDs.
    ordered = [
        ('raw_syscalls/sys_exit', syscall),
        ('sched/sched_waking', wakes),
        ('sched/sched_wakeup', wakes),
        ('sched/sched_switch', switches),
        ('raw_syscalls/sys_enter', syscall),
    ]
    actual = {}
    canonical = lambda value: re.sub(r'[\s()]', '', value)
    for name, value in ordered:
        write_filter(name, value)
        actual[name] = read_filter(name).strip()
        if canonical(actual[name]) != canonical(value):
            raise RuntimeError('source filter readback mismatch: ' + name)
    # Retain the old marker format for existing strict on-device decoders.
    # No caller should publish its source_tasks gate before this marker succeeds.
    marker('MP2_SOURCE_TASKS ' + members)
    end = now_ns()
    return dict(source_tasks=sources,irq_tid=irq_tid,begin_ns=begin,end_ns=end,
                event_filters=actual,syscall_install_order=['sys_exit','sys_enter'],
                committed=True,raw_args_exported=False)
