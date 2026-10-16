#!/usr/bin/env bash
# Exact minimum compiler, independent of the default toolchain. Honors CARGO_NET_OFFLINE.
set -euo pipefail
cd "$(dirname "$0")/.."
export RUSTUP_TOOLCHAIN=1.88.0
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
version=$(rustc --version)
case "$version" in
  "rustc 1.88.0 "*) ;;
  *) echo "Expected exactly rustc 1.88.0, got: $version" >&2; exit 1 ;;
esac
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features) ;;
    session_only) flags=(--no-default-features --features session_file) ;;
  esac
  cargo +1.88.0 check --locked ${flags[@]+"${flags[@]}"} --all-targets
  cargo +1.88.0 test --locked ${flags[@]+"${flags[@]}"}
  RUSTDOCFLAGS='-D warnings' cargo +1.88.0 doc --locked ${flags[@]+"${flags[@]}"} --no-deps
  "$python_command" scripts/check-rustdoc.py
done
