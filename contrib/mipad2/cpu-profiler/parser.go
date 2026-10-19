// SPDX-License-Identifier: MIT
package main

import (
	"encoding/binary"
	"fmt"
)

// The producer/consumer protocol is handled separately. This parser consumes one
// copied PERF_RECORD_SAMPLE: IP,TID,TIME,CALLCHAIN, in the kernel ABI order.
type sample struct { IP uint64; PID, TID uint32; Time uint64; Chain []uint64 }

func parseSample(p []byte) (sample, error) {
	var s sample
	if len(p)<32 { return s,fmt.Errorf("short sample %d",len(p)) }
	s.IP=binary.LittleEndian.Uint64(p)
	s.PID=binary.LittleEndian.Uint32(p[8:]);s.TID=binary.LittleEndian.Uint32(p[12:])
	s.Time=binary.LittleEndian.Uint64(p[16:]);n:=binary.LittleEndian.Uint64(p[24:])
	if n>127 || n>uint64((len(p)-32)/8) { return s,fmt.Errorf("invalid callchain length %d",n) }
	for i:=uint64(0);i<n;i++ { pc:=binary.LittleEndian.Uint64(p[32+i*8:]);if pc<0xfffffffffffff000 && pc!=0 { s.Chain=append(s.Chain,pc) } }
	return s,nil
}

func ringCopy(r []byte, start uint64, n int) ([]byte,error) {
	if len(r)==0 || n<0 || n>len(r) { return nil,fmt.Errorf("invalid ring read %d/%d",n,len(r)) }
	p:=make([]byte,n);offset:=int(start%uint64(len(r)));k:=copy(p,r[offset:]);copy(p[k:],r[:n-k]);return p,nil
}
