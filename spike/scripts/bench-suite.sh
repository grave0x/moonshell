#!/usr/bin/env bash
# Systematic bench suite: modes x entities x ticks with load sampling.
# Usage: bench-suite.sh [seconds] [runs]
set -u
cd "$(dirname "$0")/.."
BIN=target/release/moonshell-spike
SEC="${1:-6}"
OUT="${2:-/tmp/moonshell-bench.tsv}"
echo -e "ts\tmode\tentities\tticks\tload1\tavg_ms\tp50\tp99\tfps" > "$OUT"
load() { cut -d' ' -f1 /proc/loadavg; }
run() {
  local mode=$1 ent=$2 ticks=$3 zoom="${4:-}"
  local l0=$(load)
  if [ -n "$zoom" ]; then
    local line=$("$BIN" "$mode" "$ent" "$SEC" "$zoom" "$ticks" 2>/dev/null | grep '^RESULT')
  else
    local line=$("$BIN" "$mode" "$ent" "$SEC" "$ticks" 2>/dev/null | grep '^RESULT')
  fi
  local l1=$(load)
  if [ -z "$line" ]; then echo "  $mode $ent t$ticks FAILED" >&2; return; fi
  local avg p50 p99 fps
  avg=$(echo "$line" | sed -n 's/.*avg_ms=\([0-9.]*\).*/\1/p')
  p50=$(echo "$line" | sed -n 's/.*p50_ms=\([0-9.]*\).*/\1/p')
  p99=$(echo "$line" | sed -n 's/.*p99_ms=\([0-9.]*\).*/\1/p')
  fps=$(echo "$line" | sed -n 's/.*fps=\([0-9.]*\).*/\1/p')
  echo -e "$(date +%H:%M:%S)\t$mode\t$ent\t$ticks\t$l0/$l1\t$avg\t$p50\t$p99\t$fps" >> "$OUT"
  echo "  $mode $ent t$ticks -> ${fps}fps (load $l0->$l1)"
}
echo "== bench suite (sec=$SEC) -> $OUT =="
for mode in sim flow; do
  run $mode 100000 0
  run $mode 100000 60
done
# windowed modes: zoom 1.0, ticks 0
for mode in instanced flow-instanced; do
  run $mode 100000 0 1.0
done
echo "== done =="
cat "$OUT"
