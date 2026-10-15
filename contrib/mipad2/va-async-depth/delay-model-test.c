/* SPDX-License-Identifier: LGPL-2.1-or-later */
/* Decision/queue-count model only. Not a VA, pool, EOS or DMA integration test. */
#include <assert.h>
#include <glib.h>
static unsigned output_delay(gboolean live, const char *request) {
  if (live) return g_strcmp0(request,"2")==0 ? 2 : 0;
  return 4;
}
int main(void) {
  const char *values[]={NULL,"","0","1","2","3","4","-1","2x","999999"};
  unsigned cases=0;
  for (unsigned live=0;live<2;live++) {
    for (unsigned i=0;i<G_N_ELEMENTS(values);i++) {
      unsigned depth=output_delay(live,values[i]);
      unsigned expected=live ? (i==4 ? 2:0):4;
      assert(depth==expected);cases++;
      for(unsigned n=0;n<=100;n++) {
        unsigned queued=0,output=0;
        for(unsigned frame=0;frame<n;frame++) {
          queued++;
          if(queued>depth) {queued--;output++;}
          assert(queued<=depth && output+queued==frame+1);
        }
        /* Model the existing upstream unconditional output-list drain. */
        while(queued) {queued--;output++;}
        assert(output==n && queued==0);cases++;
      }
    }
  }
  g_print("%u decision/count-model cases PASS; no VA/driver/performance proof\n",cases);
  return 0;
}
