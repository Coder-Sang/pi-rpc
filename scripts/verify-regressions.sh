#!/usr/bin/env bash
# Native, fake-only boundary regressions in both Debug and Release.
# Uses the selected toolchain (including RUSTUP_TOOLCHAIN=1.85.0).
set -euo pipefail
cd "$(dirname "$0")/.."
rustc -vV
cargo --version
for profile in debug release; do
  flags=()
  if [ "$profile" = release ]; then
    flags=(--release)
  fi
  echo "UI, float, stack and close regressions: $profile"
  cargo test --locked --all-features ${flags[@]+"${flags[@]}"} \
    --lib --test ui_reply --test ui_close --test client --test subprocess \
    --test protocol --test json_depth --test encoding_limits \
    --test session_depth --test session_encoding_limits --test session_numbers
done
