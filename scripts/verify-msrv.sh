#!/usr/bin/env bash
# Exact minimum compiler, independent of the default toolchain. Honors CARGO_NET_OFFLINE.
set -euo pipefail
cd "$(dirname "$0")/.."
verification_bash="$BASH"
if command -v cygpath >/dev/null 2>&1; then
  verification_bash=$(cygpath -am "$verification_bash")
  case "$verification_bash" in
    *.[eE][xX][eE]) ;;
    *) verification_bash="${verification_bash}.exe" ;;
  esac
fi
export PI_RPC_TEST_BASH="$verification_bash"
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
"$python_command" scripts/test_gates.py \
  FeatureGates.test_windows_observer_follows_enabled_protocols \
  FeatureGates.test_later_pi13_failures_stop_after_successful_prefix \
  FeatureGates.test_readme_failure_stops_both_entrypoints ShellSelection
"$python_command" scripts/test_readme.py
"$python_command" scripts/test_bolt.py
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features --features pi_1_4) ;;
    session_only) flags=(--no-default-features --features session_file,pi_1_4) ;;
  esac
  cargo +1.88.0 check --locked ${flags[@]+"${flags[@]}"} --all-targets
  cargo +1.88.0 test --locked ${flags[@]+"${flags[@]}"}
  RUSTDOCFLAGS='-D warnings' cargo +1.88.0 doc --locked ${flags[@]+"${flags[@]}"} --no-deps
  "$python_command" scripts/check-rustdoc.py
done
for features in subprocess,pi_1_4 pi_1_5; do
  cargo +1.88.0 test --locked --no-default-features --features "$features" --doc
  RUSTDOCFLAGS='-D warnings' cargo +1.88.0 doc --locked --no-default-features --features "$features" --no-deps
  "$python_command" scripts/check-rustdoc.py
done

"$PI_RPC_TEST_BASH" scripts/verify-protocols.sh
