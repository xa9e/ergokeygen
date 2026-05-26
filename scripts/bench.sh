#!/usr/bin/env bash
set -euo pipefail
IFS=$'\n\t'

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
BIN="$ROOT/target/release/ergokeygen"
RUNS="${RUNS:-5}"

bench() {
  local name="$1"
  shift

  printf 'case=%s runs=%s\n' "$name" "$RUNS"
  local total_ms=0
  local best_ms=
  local last_lines=0

  for run in $(seq 1 "$RUNS"); do
    local start_ns end_ns elapsed_ms lines
    start_ns="$(date +%s%N)"
    lines="$("$BIN" "$@" | wc -l)"
    end_ns="$(date +%s%N)"
    elapsed_ms="$(((end_ns - start_ns) / 1000000))"
    total_ms="$((total_ms + elapsed_ms))"
    if [[ -z "$best_ms" || "$elapsed_ms" -lt "$best_ms" ]]; then
      best_ms="$elapsed_ms"
    fi
    last_lines="$lines"
    printf '  run=%s ms=%s lines=%s\n' "$run" "$elapsed_ms" "$lines"
  done

  printf '  best_ms=%s avg_ms=%s lines=%s\n\n' "$best_ms" "$((total_ms / RUNS))" "$last_lines"
}

cargo build --release --quiet --manifest-path "$ROOT/Cargo.toml"

bench lower_len4_reference gen --reference --min 4 --max 4 --limit 5000 --beam 4096 --prefer-hand left --mode onehand --chars lower
bench lower_len4_fast_v1 gen --engine fast-v1 --min 4 --max 4 --limit 5000 --beam 4096 --prefer-hand left --mode onehand --chars lower
bench lower_len4_6_fast_v1 gen --engine fast-v1 --min 4 --max 6 --limit 20000 --beam 4096 --prefer-hand left --mode onehand --chars lower
bench lowerdigits_len4_6_fast_v1 gen --engine fast-v1 --min 4 --max 6 --limit 20000 --beam 4096 --prefer-hand left --mode onehand --chars lowerdigits
