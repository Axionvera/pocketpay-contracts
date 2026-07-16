#!/usr/bin/env sh

set -eu

wasm_path="${1:-target/wasm32-unknown-unknown/release/savings_vault.wasm}"

if [ ! -f "$wasm_path" ]; then
  echo "error: release WASM file not found: $wasm_path" >&2
  echo "hint: run 'make build-release' to build the contract and report its size" >&2
  exit 1
fi

bytes=$(wc -c < "$wasm_path" | tr -d '[:space:]')
human_size=$(awk -v bytes="$bytes" 'BEGIN {
  split("B KiB MiB GiB", units, " ")
  size = bytes
  unit = 1
  while (size >= 1024 && unit < 4) {
    size /= 1024
    unit++
  }
  if (unit == 1) {
    printf "%d %s", size, units[unit]
  } else {
    printf "%.2f %s", size, units[unit]
  }
}')

printf 'WASM artifact: %s\n' "$wasm_path"
printf 'WASM size: %s (%s bytes)\n' "$human_size" "$bytes"
