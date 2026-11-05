#!/usr/bin/env bash
# All eight additive protocol combinations, including zero enabled protocols.
set -euo pipefail
cd "$(dirname "$0")/.."
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
for ((mask = 0; mask < 8; mask++)); do
  features=subprocess
  for ((offset = 0; offset < 3; offset++)); do
    if (( mask & (1 << offset) )); then
      features="$features,pi_1_$((offset + 3))"
    fi
  done
  echo "Protocol feature combination: $features"
  cargo check --locked --no-default-features --features "$features" --lib
  cargo test --locked --no-default-features --features "$features" \
    --test protocol --test public_paths --test versioned
 done
"$python_command" scripts/check-protocol-types.py
