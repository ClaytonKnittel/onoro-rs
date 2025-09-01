#!/usr/bin/sh

# usage: ./profile.sh <binary> [args...] > onoro.svg

rm perf.data
perf record -e cycles:pp -g -a -F 999 --call-graph dwarf,16384 -- $@ >/dev/null
perf script -F comm,pid,tid,cpu,time,event,ip,sym,dso,trace | stackcollapse-perf.pl | flamegraph.pl
