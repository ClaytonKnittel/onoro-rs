## Perf commands:

```sh
cargo bench --no-run --profile profiled --bench BENCH
perf record -e cycles:pp -g -a -F 999 --call-graph dwarf,16384 -- BENCH_BINARY "<benchmark_name>" --bench
perf script -F comm,pid,tid,cpu,time,event,ip,sym,dso,trace | stackcollapse-perf.pl | flamegraph.pl > onoro.svg
```
