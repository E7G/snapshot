#!/usr/bin/env python3
"""Linux host regression: named-only raw-output tracing; no device access.

Requires C compiler, pkg-config and GStreamer development headers/plugins.
Builds original and proposed tracer in separate temporary plugin directories.
"""
from tempfile import TemporaryDirectory
if not __debug__:
    raise RuntimeError('Test assertions require ordinary Python, not -O')
with TemporaryDirectory(prefix='mipad2-control-named-test-') as td:

    from pathlib import Path
    import collections,hashlib,importlib.util,json,os,subprocess
    w=Path(__file__).resolve().parent
    base=Path(td);plugin=base/'plugin';plugin.mkdir()
    flags=subprocess.check_output(['pkg-config','--cflags','--libs','gstreamer-1.0'],text=True).split()
    binary=plugin/'libgstmp2control.so';runner=base/'runner'
    for src,target,extra in [('gstmp2control_named.c',binary,['-shared','-fPIC']),('gst_control_named_test.c',runner,[])]:
     q=subprocess.run(['cc','-std=c11','-O2','-g','-Wall','-Wextra','-Werror',*extra,str(w/src),'-o',str(target),*flags],capture_output=True,text=True)
     assert q.returncode==0,q.stderr
    spec=importlib.util.spec_from_file_location('decoder',w/'mp2_control_decode.py');d=importlib.util.module_from_spec(spec);spec.loader.exec_module(d)
    old=base/'original-plugin';old.mkdir()
    q=subprocess.run(['cc','-std=c11','-O2','-g','-Wall','-Wextra','-Werror','-shared','-fPIC',str(w/'gstmp2control.c'),'-o',str(old/'libgstmp2control.so'),*flags],capture_output=True,text=True)
    assert q.returncode==0,q.stderr
    results=[]
    for name,plugins,slow,cap in [('old-missing-witness',old,'0',65536),('named-normal',plugin,'0',65536),('named-delays',plugin,'1',65536),('named-overflow',plugin,'1',16)]:
     trace=base/(name+'.bin')
     env=dict(os.environ,GST_PLUGIN_PATH=str(plugins),GST_REGISTRY=str(base/(name+'-registry.bin')),GST_TRACERS='mipad2-control',MIPAD2_CONTROL_FILE=str(trace),MIPAD2_CONTROL_CAPACITY=str(cap),MP2_CONTROL_TEST_SLOW=slow)
     q=subprocess.run([str(runner)],env=env,capture_output=True,text=True,timeout=25);assert q.returncode==0,q.stderr
     t=d.read_trace(trace);calls=d.intervals(t);named={p:n for p,n in t['pads'].items() if n.startswith('GstCapsFilter\t')}
     assert not any('unrelated' in n for n in named.values()),named
     src=[p for p,n in named.items() if n.endswith('mipad2-native-raw-output.GstPad:src')]
     if name=='old-missing-witness':assert not named
     elif name=='named-overflow':assert not t['complete'] and t['overflow']>0
     else:
      assert t['complete'] and not calls['unmatched_pre'] and not calls['unmatched_post']
      assert len(src)==1 and all('mipad2-native-raw-output' in n for n in named.values())
      rr=[r for r in calls['rows'] if r['pad']==src[0] and r['operation']=='push'];assert len(rr)==12 and all(r['result']==0 for r in rr)
      native=[r for r in calls['rows'] if r['operation']=='push' and r['path'].startswith('GstVideoTestSrc\t')];assert len(native)==12
      for a,b in zip(native,rr):assert (a['pts'],a['offset'])==(b['pts'],b['offset']) and a['tid']==b['tid'] and a['begin']<=b['begin']<=b['end']<=a['end']
      if slow=='1':
       assert any(r['operation']=='push-event' and r['elapsed_ns']>=50_000_000 for r in calls['rows'])
       assert any(r['operation']=='pad-query' and r['elapsed_ns']>=50_000_000 for r in calls['rows'])
     results.append(dict(case=name,runner_exit=0,complete=t['complete'],overflow=t['overflow'],named_dictionary_entries=len(named),named_output_pushes=sum(r['operation']=='push' and r['pad'] in src for r in calls['rows']),unmatched_pre_count=sum(calls['unmatched_pre'].values()),unmatched_post_count=sum(calls['unmatched_post'].values())))
    # Temporary compiled binaries and traces are removed on context exit.
    print(json.dumps(dict(state='passed',source_sha256=hashlib.sha256((w/'gstmp2control_named.c').read_bytes()).hexdigest(),binary_sha256=hashlib.sha256(binary.read_bytes()).hexdigest(),cases=results,abi_unchanged=2,named_only_filter=True,no_payload_reads=True,tablet_installed=False,hardware_tested=False)))
