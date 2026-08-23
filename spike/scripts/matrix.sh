#!/usr/bin/env bash
# Culling/zoom matrix for the sprites mode. Usage: matrix.sh [runs-per-cell]
set -u
cd "$(dirname "$0")/.."
BIN=target/release/moonshell-spike
RUNS="${1:-3}"
SECONDS="${2:-8}"
echo "load before: $(cut -d' ' -f1-3 /proc/loadavg)"
echo "run | entities zoom avg_ms p50_ms p99_ms fps"
for zoom in 0.25 1.0 4.0; do
  for r in $(seq 1 "$RUNS"); do
    line=$("$BIN" sprites 100000 "$SECONDS" "$zoom" 2>/dev/null | grep '^RESULT')
    echo "$line" | sed -E "s/RESULT mode=sprites //; s/entities=([0-9]+) spawn_ms=[0-9.]+ frames=[0-9]+ avg_ms=([0-9.]+) p50_ms=([0-9.]+) p95_ms=[0-9.]+ p99_ms=([0-9.]+) max_ms=[0-9.]+ fps=([0-9.]+)/\1 | \1 \2 \3 \4 \5/"
  done
done
echo "load after: $(cut -d' ' -f1-3 /proc/loadavg)"
