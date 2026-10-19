// SPDX-License-Identifier: MIT
package main

import("encoding/binary";"testing")
func TestSample(t *testing.T){
 p:=make([]byte,56);v:=[]uint64{0x1234, uint64(77)|(uint64(88)<<32),123456,3,0xfffffffffffffe00,0x1234,0x4567}
 for i,x:=range v {binary.LittleEndian.PutUint64(p[i*8:],x)}
 s,e:=parseSample(p);if e!=nil||s.IP!=0x1234||s.PID!=77||s.TID!=88||s.Time!=123456||len(s.Chain)!=2||s.Chain[1]!=0x4567{t.Fatalf("%+v %v",s,e)}
 for _,p:=range [][]byte{p[:12],p[:32],append(append([]byte{},p[:24]...),255,255,255,255,255,255,255,255)} {if _,e:=parseSample(p);e==nil{t.Fatal("accepted short/overflow record")}}
}
func TestRingWrap(t *testing.T){
 r:=[]byte{0,1,2,3,4,5,6,7};for _,start:=range []uint64{6,14,1022}{p,e:=ringCopy(r,start,4);if e!=nil||string(p)!=string([]byte{6,7,0,1}){t.Fatal(p,e)}}
 if _,e:=ringCopy(r,0,9);e==nil{t.Fatal("oversize accepted")}
}
