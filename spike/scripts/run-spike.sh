#!/usr/bin/env bash
# Moonshell P0 spike runner — all modes, 100k, short window (load-tolerant).
# Usage: run-spike.sh [entities] [seconds]
set -u
cd "$(dirname "$0")/.."
BIN=target/release/moonshell-spike
ENT="${1:-100000}"
SEC="${2:-8}"
echo "load: $(cut -d' ' -f1-3 /proc/loadavg)"
for spec in "sim" "flow" "sprites 1.0" "flow-sprites 1.0" "instanced 1.0"; do
  set -- $spec
  MODE=$1; ZOOM="${2:-}"
  echo "== $MODE ${ENT}x${SEC}s =="
  if [ -n "$ZOOM" ]; then
    "$BIN" "$MODE" "$ENT" "$SEC" "$ZOOM" 2>&1 | grep -E 'RESULT|ERROR|panic' || echo "  $MODE failed"
  else
    "$BIN" "$MODE" "$ENT" "$SEC" 2>&1 | grep -E 'RESULT|ERROR|panic' || echo "  $MODE failed"
  fi
  echo
done
echo "load after: $(cut -d' ' -f1-3 /proc/loadavg)"
