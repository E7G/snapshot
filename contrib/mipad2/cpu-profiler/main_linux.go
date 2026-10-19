// SPDX-License-Identifier: MIT
// Bounded per-process user CPU IP/frame-pointer sampling. No ptrace, signals,
// memory-content reads, camera buffer mapping or system-wide PMU configuration.
package main

import (
	"debug/elf"
	"encoding/binary"
	"encoding/json"
	"flag"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"sort"
	"strconv"
	"strings"
	"sync/atomic"
	"syscall"
	"time"
	"unsafe"
)

type attr struct {
	Type, Size uint32
	Config, Frequency, SampleType, ReadFormat, Flags uint64
	Wakeup, BPType uint32
	Config1, Config2, BranchSampleType, SampleRegsUser uint64
	SampleStackUser uint32
	ClockID int32
	SampleRegsIntr uint64
	AuxWatermark uint32
	SampleMaxStack, Reserved uint16
	AuxSampleSize, Reserved3 uint32
}
type event struct { FD,TID int; Name string; MM,Data []byte; Tail uint64 }
type mapping struct { Start,End,Offset uint64; Path string }
type symbols struct { F *elf.File; Funcs []elf.Symbol }
type hotspot struct { Thread,Module,Function string; ExactSymbolBounds bool; Samples int }
type result struct {
	PID int `json:"pid"`; DurationSeconds float64 `json:"duration_seconds"`
	Frequency int `json:"requested_hz_per_running_thread"`
	Samples,Lost,UnknownRecords,ParseErrors,OpenErrors int
	Threads map[int]string; ThreadSamples map[string]int
	Hotspots []hotspot; Callchains map[string]int
	Notes []string; Selftest bool
}

func openEvent(tid int)(*event,error){
	a:=attr{Type:1,Config:0,Frequency:49,SampleType:1|2|4|32,Flags:1|1<<5|1<<6|1<<10|1<<21,Wakeup:1,SampleMaxStack:24}
	a.Size=uint32(unsafe.Sizeof(a));if a.Size!=120{return nil,fmt.Errorf("unexpected perf attr size%d",a.Size)}
	fd,_,errno:=syscall.Syscall6(syscall.SYS_PERF_EVENT_OPEN,uintptr(unsafe.Pointer(&a)),uintptr(tid),^uintptr(0),^uintptr(0),8,0)
	if errno!=0{return nil,errno}
	mm,err:=syscall.Mmap(int(fd),0,9*os.Getpagesize(),syscall.PROT_READ|syscall.PROT_WRITE,syscall.MAP_SHARED)
	if err!=nil{syscall.Close(int(fd));return nil,err}
	offset:=binary.LittleEndian.Uint64(mm[1040:]);size:=binary.LittleEndian.Uint64(mm[1048:])
	if offset==0||size==0||offset+size>uint64(len(mm)){syscall.Munmap(mm);syscall.Close(int(fd));return nil,fmt.Errorf("invalid ring geometry %d/%d",offset,size)}
	name,_:=os.ReadFile(fmt.Sprintf("/proc/%d/comm",tid))
	e:=&event{FD:int(fd),TID:tid,Name:strings.TrimSpace(string(name)),MM:mm,Data:mm[offset:offset+size]}
	_,_,errno=syscall.Syscall(syscall.SYS_IOCTL,fd,0x2400,0)
	if errno!=0{e.close();return nil,errno};return e,nil
}
func(e *event)close(){syscall.Syscall(syscall.SYS_IOCTL,uintptr(e.FD),0x2401,0);syscall.Munmap(e.MM);syscall.Close(e.FD)}
func(e *event)drain(r *result, consume func(sample,string)){
	head:=atomic.LoadUint64((*uint64)(unsafe.Pointer(&e.MM[1024])))
	if head<e.Tail||head-e.Tail>uint64(len(e.Data)){r.ParseErrors++;e.Tail=head}
	for e.Tail<head{
		h,_:=ringCopy(e.Data,e.Tail,8);typ:=binary.LittleEndian.Uint32(h);n:=int(binary.LittleEndian.Uint16(h[6:]))
		if n<8||n>len(e.Data)||uint64(n)>head-e.Tail{r.ParseErrors++;e.Tail=head;break}
		p,_:=ringCopy(e.Data,e.Tail,uint64ToInt(n));e.Tail+=uint64(n)
		switch typ {
		case 9:s,err:=parseSample(p[8:]);if err!=nil{r.ParseErrors++}else{r.Samples++;r.ThreadSamples[e.Name]++;consume(s,e.Name)}
		case 2:if len(p)>=24{r.Lost+=int(binary.LittleEndian.Uint64(p[16:]))}else{r.ParseErrors++}
		default:r.UnknownRecords++
		}
	}
	atomic.StoreUint64((*uint64)(unsafe.Pointer(&e.MM[1032])),e.Tail)
}
func uint64ToInt(n int)int{return n}
func getMaps(pid int)[]mapping{
	p,_:=os.ReadFile(fmt.Sprintf("/proc/%d/maps",pid));var maps []mapping
	for _,l:=range strings.Split(string(p),"\n"){
		f:=strings.Fields(l);if len(f)<5||!strings.Contains(f[1],"x"){continue};bounds:=strings.Split(f[0],"-");if len(bounds)!=2{continue}
		a,_:=strconv.ParseUint(bounds[0],16,64);b,_:=strconv.ParseUint(bounds[1],16,64);o,_:=strconv.ParseUint(f[2],16,64)
		path:="[anonymous-executable]";if len(f)>5{path=strings.Join(f[5:]," ")};maps=append(maps,mapping{a,b,o,path})
	};return maps
}
type resolver struct{ Maps []mapping; ELF map[string]*symbols; Cache map[uint64]hotspot }
func(s *resolver)resolve(pc uint64)hotspot{
	if h,ok:=s.Cache[pc];ok{return h};h:=hotspot{Module:"[unmapped]",Function:"[unresolved]"}
	for _,m:=range s.Maps{if pc<m.Start||pc>=m.End{continue};h.Module=m.Path;h.Function="[unresolved-code-offset]";if !strings.HasPrefix(m.Path,"/"){break}
		sy,ok:=s.ELF[m.Path];if !ok{
			f,e:=elf.Open(m.Path);if e!=nil{s.ELF[m.Path]=nil;break};sy=&symbols{F:f};for _,dyn:=range []bool{false,true}{var v []elf.Symbol;if dyn{v,_=f.DynamicSymbols()}else{v,_=f.Symbols()};for _,x:=range v{if (elf.ST_TYPE(x.Info)==elf.STT_FUNC||elf.ST_TYPE(x.Info)==elf.STT_GNU_IFUNC)&&x.Size>0{sy.Funcs=append(sy.Funcs,x)}}};sort.Slice(sy.Funcs,func(i,j int)bool{return sy.Funcs[i].Value<sy.Funcs[j].Value});s.ELF[m.Path]=sy
		};if sy==nil{break}
		off:=pc-m.Start+m.Offset;address:=off
		for _,p:=range sy.F.Progs{if p.Type==elf.PT_LOAD&&off>=p.Off&&off<p.Off+p.Filesz{address=p.Vaddr+off-p.Off;break}}
		idx:=sort.Search(len(sy.Funcs),func(i int)bool{return sy.Funcs[i].Value>address})-1
		if idx>=0{x:=sy.Funcs[idx];if address>=x.Value&&address-x.Value<x.Size{h.Function=x.Name;h.ExactSymbolBounds=true}}
		if !h.ExactSymbolBounds{h.Function=fmt.Sprintf("[unresolved-code-offset:0x%x]",address)};break
	};s.Cache[pc]=h;return h
}

//go:noinline
func profileBusyWork(until time.Time){var n uint64=7;for time.Now().Before(until){for i:=0;i<4000;i++{n=n*6364136223846793005+1};atomic.StoreUint64(&busySink,n)}}
var busySink uint64

func main(){
	pid:=flag.Int("pid",0,"target process");duration:=flag.Int("seconds",20,"bounded seconds,1..45");output:=flag.String("output","","new output JSON path");selftest:=flag.Bool("selftest",false,"profile only this helper's busy thread");flag.Parse()
	if *selftest{*pid=os.Getpid();*duration=2;runtime.LockOSThread()}
	if *pid<=0||*duration<1||*duration>45||*output==""{panic("pid/output/seconds required")};if _,e:=os.Stat(*output);!os.IsNotExist(e){panic("output already exists or inaccessible")}
	r:=result{PID:*pid,Frequency:49,Threads:map[int]string{},ThreadSamples:map[string]int{},Callchains:map[string]int{},Selftest:*selftest,Notes:[]string{"Userspace CPU-clock sampling only, no kernel/walltime attribution.","Callchains use frame pointers and may be incomplete; bounded symbol ranges only, ORC/JIT remains unresolved.","No ptrace, signals, process-memory-content reads, camera-buffer maps, policy changes or media export. Profiling has nonzero overhead."}}
	resolve:=resolver{ELF:map[string]*symbols{},Cache:map[uint64]hotspot{}};hits:=map[string]*hotspot{};var events []*event;seen:=map[int]bool{}
	add:=func(){entries,e:=os.ReadDir(fmt.Sprintf("/proc/%d/task",*pid));if e!=nil{return};for _,t:=range entries{tid,e:=strconv.Atoi(t.Name());if e!=nil||seen[tid]{continue};seen[tid]=true;ev,e:=openEvent(tid);if e!=nil{r.OpenErrors++;fmt.Fprintln(os.Stderr,"perf-open",tid,e);continue};events=append(events,ev);r.Threads[tid]=ev.Name}}
	add();if len(events)==0{panic("no perf event opened; no test launched")};defer func(){for _,e:=range events{e.close()};for _,s:=range resolve.ELF{if s!=nil{s.F.Close()}}}()
	start:=time.Now();if *selftest{profileBusyWork(start.Add(time.Duration(*duration)*time.Second))}
	consume:=func(s sample,name string){
		h:=resolve.resolve(s.IP);h.Thread=name;k:=name+"|"+h.Module+"|"+h.Function;if hits[k]==nil{v:=h;hits[k]=&v};hits[k].Samples++
		var chain []string;for i,pc:=range s.Chain{if i>=12{break};x:=resolve.resolve(pc);chain=append(chain,filepath.Base(x.Module)+":"+x.Function)};if len(chain)>0{r.Callchains[name+" | "+strings.Join(chain," <- ")]++}
	}
	// Refresh mappings before decoding the copied samples, never read target pages.
	resolve.Maps=getMaps(*pid);nextAdd:=time.Now()
	for time.Since(start)<time.Duration(*duration)*time.Second{
		if time.Now().After(nextAdd){add();resolve.Maps=getMaps(*pid);resolve.Cache=map[uint64]hotspot{};nextAdd=time.Now().Add(time.Second)}
		for _,e:=range events{e.drain(&r,consume)};time.Sleep(20*time.Millisecond)
	}
	resolve.Maps=getMaps(*pid);resolve.Cache=map[uint64]hotspot{};for _,e:=range events{syscall.Syscall(syscall.SYS_IOCTL,uintptr(e.FD),0x2401,0);e.drain(&r,consume)}
	r.DurationSeconds=time.Since(start).Seconds();for _,h:=range hits{r.Hotspots=append(r.Hotspots,*h)};sort.Slice(r.Hotspots,func(i,j int)bool{return r.Hotspots[i].Samples>r.Hotspots[j].Samples})
	b,err:=json.MarshalIndent(r,"","  ");if err!=nil{panic(err)};f,err:=os.OpenFile(*output,os.O_CREATE|os.O_EXCL|os.O_WRONLY,0600);if err!=nil{panic(err)};_,err=f.Write(append(b,'\n'));f.Close();if err!=nil{panic(err)}
	fmt.Printf("user CPU samples=%d lost=%d parse-errors=%d threads=%d output=%s\n",r.Samples,r.Lost,r.ParseErrors,len(events),*output)
	if r.Samples==0||r.Lost!=0||r.ParseErrors!=0||r.OpenErrors!=0{os.Exit(2)}
	if *selftest{found:=false;for _,h:=range r.Hotspots{if strings.Contains(h.Function,"profileBusyWork"){found=true}};if !found{panic("self-test hot function not resolved")}}
}
