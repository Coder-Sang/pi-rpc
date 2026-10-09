#!/usr/bin/env bash
# No real Pi/model requests. Local example uses the compiled test fake.
set -euo pipefail
cd "$(dirname "$0")/.."
cargo fmt --check
rustfmt --edition 2024 --check tests/fixtures/fake_pi.rs
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features) ;;
    session_only) flags=(--no-default-features --features session_file) ;;
  esac
  cargo check --locked ${flags[@]+"${flags[@]}"} --all-targets
  cargo test --locked ${flags[@]+"${flags[@]}"}
  cargo clippy --locked ${flags[@]+"${flags[@]}"} --all-targets -- -D warnings
  RUSTDOCFLAGS='-D warnings' cargo doc --locked ${flags[@]+"${flags[@]}"} --no-deps
done
cargo run --locked --example injected --no-default-features
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
