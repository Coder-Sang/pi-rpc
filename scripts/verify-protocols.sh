#!/usr/bin/env bash
# Every protocol/subprocess/session_file combination, including no features.
set -euo pipefail
cd "$(dirname "$0")/.."
strict_clippy=false
if [ "$#" -eq 1 ] && [ "$1" = --clippy ]; then
  strict_clippy=true
elif [ "$#" -ne 0 ]; then
  echo 'usage: bash scripts/verify-protocols.sh [--clippy]' >&2
  exit 2
fi
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
feature_names=(pi_1_3 pi_1_4 pi_1_5 subprocess session_file)
for ((mask = 0; mask < 32; mask++)); do
  features=
  for ((offset = 0; offset < 5; offset++)); do
    if (( mask & (1 << offset) )); then
      features="${features:+$features,}${feature_names[offset]}"
    fi
  done
  flags=(--locked --no-default-features)
  if [ -n "$features" ]; then
    flags+=(--features "$features")
  fi
  echo "Feature combination: ${features:-none}"
  cargo check "${flags[@]}" --all-targets
  if "$strict_clippy"; then
    cargo clippy "${flags[@]}" --all-targets -- -D warnings
  fi
  if (( (mask & 7) == 0 )); then
    # Codec/native-file utilities must work without any protocol profile.
    cargo test "${flags[@]}"
  else
    cargo test "${flags[@]}" --test protocol --test public_paths --test versioned
  fi
done
"$python_command" scripts/check-protocol-types.py
