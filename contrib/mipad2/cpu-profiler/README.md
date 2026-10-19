# Bounded per-process user CPU sampler (diagnostic v1)

Linux/amd64, Go standard library only; no perf package/global sysctl changes.
Source exactly matches the recorded Go1.23.4 build receipt in the evidence JSON.

```sh
go test -count=10 parser.go parser_test.go
CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath \
  -o /absolute/private/mp2-cpuprof main_linux.go parser.go
/absolute/private/mp2-cpuprof --selftest --output=/absolute/private/new-selftest.json
sudo -n /absolute/private/mp2-cpuprof --pid=1234 --seconds=24 \
  --output=/absolute/private/new-profile.json
```

Use a verified OWN test process PID and fresh output. Samples49Hz per running
thread using software CPU-clock events, excludes kernel/hypervisor, copies ring
records with acquire/release ordering, bounds checks records, reports losses and
opens newly created threads. Requires normal perf permissions; do not weaken
system-wide policy. Nonzero open/parse/lost/empty-sample results fail the helper.
No ptrace/signals, target memory-content reads, camera maps or retained buffers.
Only code-address/frame-pointer metadata is sampled; ELF/maps are read for symbol
resolution. Callchains can be incomplete/ORC-JIT unresolved; no false nearest-
symbol attribution outside known symbol bounds. CPU samples are not walltime,
energy, exact CPU savings or optical-parity proof. Profiling adds overhead.
All raw camera media/cores/individual pixel digests stay on the device.
