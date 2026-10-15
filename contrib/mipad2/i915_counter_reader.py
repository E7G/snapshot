# SPDX-License-Identifier: GPL-3.0-or-later
"""Read-only x86_64 perf_event_open counters; never change sysctl/RPS settings."""
from pathlib import Path
import ctypes, os, platform, struct

class I915Counters:
    def __init__(self):
        assert platform.machine()=='x86_64'
        self.fds={}
        pmu=Path('/sys/bus/event_source/devices/i915')
        kind=int((pmu/'type').read_text())
        cpu=int((pmu/'cpumask').read_text().strip().split(',')[0].split('-')[0])
        libc=ctypes.CDLL(None,use_errno=True);libc.syscall.restype=ctypes.c_long
        try:
            for name in ['rcs0-busy','vcs0-busy','bcs0-busy','vecs0-busy','rc6-residency']:
                config=int((pmu/'events'/name).read_text().strip().split('=')[1],0)
                attr=ctypes.create_string_buffer(128)
                # PERF_FORMAT_TOTAL_TIME_ENABLED|PERF_FORMAT_TOTAL_TIME_RUNNING.
                struct.pack_into('=IIQQQQQ',attr,0,kind,128,config,0,0,3,0)
                fd=libc.syscall(ctypes.c_long(298),ctypes.byref(attr),ctypes.c_int(-1),ctypes.c_int(cpu),ctypes.c_int(-1),ctypes.c_ulong(8))
                if fd<0:raise OSError(ctypes.get_errno(),name)
                self.fds[name]=int(fd)
        except BaseException:
            self.close();raise
    def read(self):
        result={}
        for name,fd in self.fds.items():
            data=os.read(fd,24);assert len(data)==24
            value,enabled,running=struct.unpack('=QQQ',data)
            result[name]=dict(value_ns=value,time_enabled_ns=enabled,time_running_ns=running)
        return result
    def close(self):
        for fd in self.fds.values():os.close(fd)
        self.fds.clear()
    def __enter__(self):return self
    def __exit__(self,*args):self.close()
