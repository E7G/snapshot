import unittest
from mp2_source_filters_v2 import install_source_filters


class SourceFilters(unittest.TestCase):
    def setup_fake(self, old=()):
        filters={name:' || '.join('common_pid == '+str(t) for t in old) or 'common_pid == -1'
                 for name in ['raw_syscalls/sys_enter','raw_syscalls/sys_exit']}
        events=[]
        writes=[]
        markers=[]
        def write(name,value):
            filters[name]=value
            writes.append(name)
            for tid in [11,22]:
                for op in ['sys_enter','sys_exit']:
                    if 'common_pid == '+str(tid) in filters['raw_syscalls/'+op]:
                        events.append((tid,op))
        return filters,events,writes,markers,write

    def test_exit_before_enter_and_commit_last(self):
        f,e,w,m,write=self.setup_fake([11])
        result=install_source_filters(write,lambda n:'('+f[n]+')',m.append,99,[22,11,22])
        self.assertEqual(w[0],'raw_syscalls/sys_exit')
        self.assertEqual(w[-1],'raw_syscalls/sys_enter')
        self.assertEqual(result['source_tasks'],[11,22])
        self.assertEqual(m[-1],'MP2_SOURCE_TASKS 11,22')
        self.assertTrue(result['committed'])
        pending={}
        for tid,op in e:
            if op=='sys_enter':
                self.assertNotIn(tid,pending)
                pending[tid]=True
            else:
                pending.pop(tid,None)  # permitted installation-boundary left censor
        self.assertFalse(pending)
        self.assertEqual(sum(t==11 and op=='sys_enter' for t,op in e),len(w))
        self.assertEqual(sum(t==11 and op=='sys_exit' for t,op in e),len(w))

    def test_old_enter_first_order_has_real_negative_control(self):
        f,e,w,m,write=self.setup_fake()
        write('raw_syscalls/sys_enter','common_pid == 22')
        write('sched/sched_switch','prev_pid == 22 || next_pid == 22')
        write('raw_syscalls/sys_exit','common_pid == 22')
        pending=False;repeated=0
        for tid,op in e:
            if tid!=22:continue
            if op=='sys_enter':repeated+=pending;pending=True
            else:pending=False
        self.assertGreater(repeated,0)

    def test_bad_readback_never_publishes_commit(self):
        f,e,w,m,write=self.setup_fake()
        with self.assertRaises(RuntimeError):
            install_source_filters(write,lambda n:'common_pid == 999',m.append,99,[11])
        self.assertFalse(any(v.startswith('MP2_SOURCE_TASKS ') for v in m))
        self.assertEqual(w,['raw_syscalls/sys_exit'])

    def test_invalid_scope_never_writes(self):
        for tids,irq in [([],99),([0],99),([-1],99),(['11'],99),([11],0)]:
            f,e,w,m,write=self.setup_fake()
            with self.assertRaises(ValueError):
                install_source_filters(write,lambda n:f[n],m.append,irq,tids)
            self.assertFalse(w);self.assertFalse(m)


if __name__=='__main__':unittest.main()
