#!/usr/bin/env bash
# No real Pi/model requests. Local example uses the compiled test fake.
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
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
"$python_command" scripts/check-format.py
"$python_command" scripts/test_gates.py
"$python_command" scripts/test_readme.py
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features --features pi_1_4) ;;
    session_only) flags=(--no-default-features --features session_file,pi_1_4) ;;
  esac
  cargo check --locked ${flags[@]+"${flags[@]}"} --all-targets
  cargo test --locked ${flags[@]+"${flags[@]}"}
  cargo clippy --locked ${flags[@]+"${flags[@]}"} --all-targets -- -D warnings
  RUSTDOCFLAGS='-D warnings' cargo doc --locked ${flags[@]+"${flags[@]}"} --no-deps
  "$python_command" scripts/check-rustdoc.py
done
for features in subprocess,pi_1_4 pi_1_5; do
  cargo test --locked --no-default-features --features "$features" --doc
  RUSTDOCFLAGS='-D warnings' cargo doc --locked --no-default-features --features "$features" --no-deps
  "$python_command" scripts/check-rustdoc.py
done
cargo run --locked --example injected --no-default-features --features pi_1_4
cargo run --locked --example injected --all-features
cargo run --locked --example native_session --no-default-features --features session_file
cargo run --locked --example native_session --all-features
mkdir -p target/verification
fake="$PWD/target/verification/fake-pi"
case "$(uname -s)" in
  MINGW*|MSYS*|CYGWIN*) fake="${fake}.exe" ;;
esac
rustc --edition=2024 tests/fixtures/fake_pi.rs -o "$fake"
# Native Windows programs need a drive-qualified path, not an MSYS /c/... path.
if command -v cygpath >/dev/null 2>&1; then
  fake=$(cygpath -m "$fake")
fi
cargo run --locked --example local -- "$fake"
cargo run --locked --example local --all-features -- "$fake"

"$PI_RPC_TEST_BASH" scripts/verify-protocols.sh --clippy
