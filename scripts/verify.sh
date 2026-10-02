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
  cargo check ${flags[@]+"${flags[@]}"} --all-targets
  cargo test ${flags[@]+"${flags[@]}"}
  cargo clippy ${flags[@]+"${flags[@]}"} --all-targets -- -D warnings
done
cargo run --example injected --no-default-features
cargo run --example injected --all-features
cargo run --example native_session --no-default-features --features session_file
cargo run --example native_session --all-features
mkdir -p target/verification
rustc --edition=2024 tests/fixtures/fake_pi.rs -o target/verification/fake-pi
cargo run --example local -- "$PWD/target/verification/fake-pi"
cargo run --example local --all-features -- "$PWD/target/verification/fake-pi"
RUSTDOCFLAGS='-D warnings' cargo doc --all-features --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --no-default-features --no-deps
