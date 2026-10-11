#!/usr/bin/env bash
# Repeated fake-only transport acceptance. Honors CARGO_NET_OFFLINE.
set -euo pipefail
cd "$(dirname "$0")/.."
rounds="${1-3}"
if [ "$#" -gt 1 ] || [[ ! "$rounds" =~ ^[1-9][0-9]?$ ]] || [ "$rounds" -gt 20 ]; then
  echo 'usage: bash scripts/verify-stress.sh [rounds: 1..20, default 3]' >&2
  exit 2
fi
for ((round = 1; round <= rounds; round++)); do
  echo "Transport stress round $round/$rounds"
  cargo test --locked --all-features --test client --test subprocess --test ui_close --test ui_reply --test json_depth --test encoding_limits
done
