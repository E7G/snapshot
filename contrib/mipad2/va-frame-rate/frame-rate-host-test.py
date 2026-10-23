from pathlib import Path
from fractions import Fraction
import subprocess,json,tempfile,random,re,math,time,argparse,hashlib
parser=argparse.ArgumentParser(description='Compile actual upstream/fixed VA frame-rate submission against an independent bounded rational oracle')
parser.add_argument('--upstream-file',required=True,type=Path)
args=parser.parse_args()
original=args.upstream_file.read_text()
assert hashlib.sha256(original.encode()).hexdigest()=='6b104dd3909f47c3b5903c83e646ab933b11cfb4d484be1d08cde74a30237dba'
with tempfile.TemporaryDirectory(prefix='mp2-va-patch-') as directory:
 root=Path(directory);p=root/'sys/va/gstvabaseenc.c';p.parent.mkdir(parents=True);p.write_text(original)
 patch=Path(__file__).with_name('0001-pack-bounded-rational-frame-rate.patch')
 for flags in [['--check'],[]]:subprocess.run(['git','-C',str(root),'-c','core.autocrlf=false','apply',*flags,str(patch)],check=True)
 fixed=p.read_text()

def extract(s,name):
 m=re.search(r'^(?:static )?gboolean\n'+name+r' \(',s,re.M);assert m
 start=s.index('{',m.start());depth=1;end=start+1
 while depth:
  depth+=(s[end]=='{')-(s[end]=='}');end+=1
 return s[m.start():end]+'\n'
model=r'''
#include <stdio.h>
#include <stdint.h>
#include <string.h>
#include <limits.h>
typedef unsigned guint; typedef uint64_t guint64; typedef uint32_t guint32;typedef int gboolean;
#define TRUE 1
#define FALSE 0
#define G_MAXINT INT_MAX
#define MIN(a,b) ((a)<(b)?(a):(b))
#define GST_ERROR_OBJECT(...) do{}while(0)
#define GST_VIDEO_INFO_FPS_N(x) ((x)->n)
#define GST_VIDEO_INFO_FPS_D(x) ((x)->d)
typedef struct { struct { int n,d; } in_info; void *encoder; } GstVaBaseEnc;
typedef struct { int unused; } GstVaEncodePicture;
typedef unsigned VAEncMiscParameterType;
typedef struct {uint32_t framerate;uint32_t flags;uint32_t reserved[4];} VAEncMiscParameterFrameRate;
#define VAEncMiscParameterTypeFrameRate 7
#define VAEncMiscParameterBufferType 8
static unsigned called,captured;static int inject;
static int gst_va_encoder_add_param(void *enc,GstVaEncodePicture *pic,int type,void *data,unsigned size){
 (void)enc;(void)pic;(void)type;(void)size;called++;memcpy(&captured,(char *)data+4,4);return !inject;
}
'''
main=r'''
int main(void){unsigned n,d;int failure;GstVaEncodePicture pic={0};
 while(scanf("%u %u %d",&n,&d,&failure)==3){GstVaBaseEnc b={.in_info={(int)n,(int)d}};called=0;captured=0;inject=failure;
  int result=gst_va_base_enc_add_frame_rate_parameter(&b,&pic);
  printf("%d %u %u\n",result,called,captured);
 }return 0;}
'''
def run(source,cases,label):
 helper=extract(source,'gst_va_base_enc_pack_frame_rate') if 'gst_va_base_enc_pack_frame_rate' in source else ''
 code=model+helper+extract(source,'gst_va_base_enc_add_frame_rate_parameter')+main
 with tempfile.TemporaryDirectory(prefix='mp2-va-fr-') as directory:
  p=Path(directory);(p/'test.c').write_text(code)
  q=subprocess.run(['cc','-O1','-g','-Wall','-Wextra','-Werror','-fsanitize=address,undefined','-fno-omit-frame-pointer','-no-pie',str(p/'test.c'),'-o',str(p/'test')],capture_output=True,text=True);assert q.returncode==0,q.stderr
  q=subprocess.run([str(p/'test')],input=''.join(f'{n} {d} {error}\n' for n,d,error in cases),capture_output=True,text=True,timeout=30);assert q.returncode==0 and not q.stderr,q.stderr
 rows=[tuple(map(int,line.split())) for line in q.stdout.splitlines()];assert len(rows)==len(cases)
 return rows
def oracle(n,d,limit):
 best=None
 for q in range(1,limit+1):
  p=max(1,min(limit,(2*n*q+d)//(2*d)));error=abs(n*q-d*p)
  if best is None or error*best[1]<best[2]*q:best=(p,q,error)
 return Fraction(best[0],best[1])

rng=random.Random(720026);rates=[(30,1),(30000,1001),(78125,2604),(78125,5208),(120000,4004),(65535,1),(1,65535),(INT:=2147483647,INT),(INT,1),(1,INT),(0,1),(30,0)]
rates += [(rng.randint(1,2147483647),rng.randint(1,2147483647)) for _ in range(120)]
rates += [(rng.randint(1,2147483647),rng.randint(1,100000)) for _ in range(60)]
cases=[(n,d,e) for n,d in rates for e in [0,1]]
baseline=run(original,[(78125,2604,0)],'baseline');packed=baseline[0][2];before=Fraction(packed&65535,packed>>16 or 1);assert before==Fraction(12589,2604) and abs(before-Fraction(78125,2604))>20
rows=run(fixed,cases,'fixed');assert all(len(row)==3 for row in rows)
for (n,d,error),(ret,calls,packed) in zip(cases,rows):
 valid=n>0 and d>0 and n<=2147483647 and d<=2147483647 and n<=65535*d and d<=65535*n
 if not valid:assert (ret,calls,packed)==(0,0,0);continue
 assert calls==1 and ret==int(not error)
 p,q=packed&65535,packed>>16;assert 1<=p<=65535 and 1<=q<=65535
 assert abs(Fraction(p,q)-Fraction(n,d))==abs(oracle(n,d,65535)-Fraction(n,d)),(n,d,p,q)
# Exhaustively verify the same compiled algorithm at an intentionally small bound.
small=fixed.replace('const guint64 limit = 0xffff;','const guint64 limit = 8;');smallcases=[(n,d,0) for n in range(1,64) for d in range(1,64)]
for (n,d,_),(ret,calls,packed) in zip(smallcases,run(small,smallcases,'small')):
 if n>8*d or d>8*n:assert (ret,calls)==(0,0)
 else:assert ret==calls==1 and abs(Fraction(packed&65535,packed>>16)-Fraction(n,d))==abs(oracle(n,d,8)-Fraction(n,d)),(n,d,packed)
record=dict(state='actual-source-parameter-host-test-pass',baseline_compiled=True,baseline_hardware_fraction=str(before),desired_fraction='78125/2604',fixed_hardware_fraction=str(Fraction(rows[4][2]&65535,rows[4][2]>>16)),production_cases=len(cases),bounded_exhaustive_cases=len(smallcases),oracle='Independent enumerate-every-denominator nearest fraction; exact integer comparisons allow equidistant ties',actual_production_dispatch_compiled=True,sanitizers='ASan/UBSan',invalid_rates_rejected_before_va_submission=True,va_submission_error_propagated=True,hardware_tested=False,scope='Actual generic VA frame-rate parameter function, synthetic VA submission endpoint only; not buffer encoding, native hardware or application cadence proof')
print(json.dumps(record,indent=2))
