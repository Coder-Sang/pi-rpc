#!/usr/bin/env bash
# Exact minimum compiler, independent of the default toolchain. Honors CARGO_NET_OFFLINE.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.85.0
version=$(rustc --version)
case "$version" in
  "rustc 1.85.0 "*) ;;
  *) echo "Expected exactly rustc 1.85.0, got: $version" >&2; exit 1 ;;
esac
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features) ;;
    session_only) flags=(--no-default-features --features session_file) ;;
  esac
  cargo +1.85.0 check --locked ${flags[@]+"${flags[@]}"} --all-targets
  cargo +1.85.0 test --locked ${flags[@]+"${flags[@]}"}
done
