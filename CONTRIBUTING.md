# Contributing

Read the [README](README.md), [connection contract](docs/connections.md) and [compatibility notes](docs/compatibility.md) before changing behavior. The current Git version is 0.2.3 development; the published version remains 0.2.2. Preserve public paths, field types, feature names and the Rust 1.88 minimum throughout 0.2.x.

## Development requirements

Install stable Rust with rustfmt/Clippy and exact Rust **1.88.0**, Bash, Git, Python **3.11+** and a native C/assembly compiler. Windows requires Git Bash, MSVC and the Windows SDK. Stack-switching dependencies need the native compiler; installing a Rust target alone does not establish platform compatibility.

Run from the repository root. Download toolchains and dependencies before offline gates. On Windows, use `python` if `python3` is unavailable. The scripts honor Cargo offline settings and effective target directories unless a generated fixture path is documented separately.

## Verification

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
RUSTUP_TOOLCHAIN=1.88.0 CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
RUSTUP_TOOLCHAIN=1.88.0 CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
python3 scripts/test_checks.py
python3 scripts/check-docs.py .
git diff --check
```

The feature matrix covers default, no defaults, session-only and all features. Stable runs strict formatting/Clippy/Rustdoc; both compilers test generated HTML links, stress and Debug/Release boundaries. Tests use in-memory transports and a std-only compiled fake. They never execute an installed Pi or submit a model request. Separately opt-in [real acceptance](docs/acceptance.md) requires prepared isolated environments and explicit authorization.

CI runs Linux/macOS/Windows × stable/exact 1.88.0. Local macOS results alone do not establish the other platforms. See [verification](docs/verification.md) for evidence scope and [publishing](docs/publishing.md#final-verification-before-upload) for package, consumer and release gates.

## Changes and reviews

Keep changes focused. Add regressions for behavior changes, preserving feature coverage and deterministic lifecycle synchronization. Run the applicable gates; record the compiler, platform and results without claiming tests not executed. New runtime APIs belong in the 0.2.3 Unreleased section of [CHANGELOG](CHANGELOG.md); user-facing changes should explain resulting behavior rather than development history.

Keep README examples usable with the published dependency, and label Git-only APIs explicitly. README is included in crate Rustdoc, so link project documents with HTTPS repository URLs there. Guide-local relative links are appropriate in other Markdown files. Update Markdown and generated-HTML checks when document inventories or anchors change.

Do not commit credentials, private agent settings, generated conversations or diagnostics. Keep candidate reports, first-failure logs and archive hashes under Cargo's ignored target directory, outside packaged source. License and source-provenance records remain part of the crate. `.env.example` is allowed; local `.env` and auth/settings files are excluded. See [security reporting](SECURITY.md).

## Reporting issues

Include the pi-rpc version or commit, enabled features, `rustc -Vv`, OS/target, concrete Pi version and a minimal reproduction. Prefer an in-memory or fake-Pi reproduction. Describe expected/actual behavior and the terminal error without attaching private prompts, history, authentication or raw stderr.

## Informational measurements

```bash
cargo run --release --no-default-features --features session_file --example session_builder_bench
cargo run --release --no-default-features --features session_file --example session_validation_bench
cargo test --locked --release --all-features --lib image_encoding_measurement -- --ignored --nocapture
```

These print median timings and, for image encoding, reserved output bytes. They do not measure RSS and have no performance pass/fail threshold. They use synthetic data only.
