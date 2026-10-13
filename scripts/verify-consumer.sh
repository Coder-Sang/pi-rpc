#!/usr/bin/env bash
# Fresh downstream resolution from an archive bound to this clean source commit.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf -- "$work"' EXIT
if command -v cygpath >/dev/null 2>&1; then
  root_native=$(cygpath -m "$root")
  work_native=$(cygpath -m "$work")
else
  root_native="$root"
  work_native="$work"
fi
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
# Resolve relative --archive arguments before changing the caller's directory.
consumer_target=$(PYTHONUTF8=1 "$python_command" "$root_native/scripts/prepare_consumer.py" \
  --source-root "$root_native" --destination "$work_native" "$@")
export CARGO_TARGET_DIR="$consumer_target"
manifest="$work_native/consumer/Cargo.toml"
cd "$root"
rustc --version
cargo generate-lockfile --manifest-path "$manifest"
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features) ;;
    session_only) flags=(--no-default-features --features session_file) ;;
  esac
  echo "Packaged independent consumer: $configuration"
  cargo test --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"}
  cargo run --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"}
  if [ "$configuration" = all ] || [ "$configuration" = session_only ]; then
    cargo run --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"} --example native_session
  fi
done
