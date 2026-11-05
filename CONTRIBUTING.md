# Contributing

Read the [README](README.md), [connection contract](docs/connections.md) and [compatibility notes](docs/compatibility.md) before changing behavior. The current Git version is 0.3.0 development; published 0.2.2 retains its historical API. Follow the [0.3 migration contract](docs/migration-0.3.md): explicit protocol types, additive features, no provider inference, shared lifecycle and Rust 1.88 minimum.

## Development requirements

Install current stable Rust (local 0.3 checks use 1.99.1) with rustfmt/Clippy and exact Rust **1.88.0**, Bash, Git, Python **3.11+** and a native C/assembly compiler. Windows requires Git Bash, MSVC and the Windows SDK. Stack-switching dependencies need the native compiler; installing a Rust target alone does not establish platform compatibility.

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

The four full configurations cover defaults, all features, Transport-only Pi14 and session_file with Pi14. Both compilers additionally check all 32 combinations of the five optional features, including builds without Pi14 or any protocol profile. Stable runs strict Clippy for every combination and checks formatting of every Rust file under src/tests/examples, including macro include! files and standalone fixtures. Both compilers test generated HTML links, stress and Debug/Release boundaries. Tests use in-memory transports and a std-only compiled fake. They never execute an installed Pi or submit a model request. Separately opt-in [real acceptance](docs/acceptance.md) requires prepared isolated environments and explicit authorization.

CI runs Linux/macOS/Windows × stable/exact 1.88.0. Local macOS results alone do not establish the other platforms. See [verification](docs/verification.md) for evidence scope and [publishing](docs/publishing.md#final-verification-before-upload) for package, consumer and release gates.

## Changes and reviews

Keep changes focused. Add regressions for behavior changes, preserving feature coverage and deterministic lifecycle synchronization. Run the applicable gates; record the compiler, platform and results without claiming tests not executed. New runtime APIs belong in the 0.3.0 Unreleased section of [CHANGELOG](CHANGELOG.md); user-facing changes should explain resulting behavior rather than development history.

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

Protocol tests run the complete schema suite for each enabled profile. `verify-protocols.sh` checks every combination with all targets and runs protocol/public-path/dispatch tests; profile-free builds run their full codec/native-file suite. Pass `--clippy` for strict checks, as verify.sh does; the MSRV entrypoint does not require the optional Clippy component. Compiler-positive controls precede cross-version type failures. The four full configurations always include Pi14, preserving the existing transport/resource regressions; Pi13/Pi15 have independent command/event/UI/startup/close coverage. Do not weaken fixtures, deadlines or assertions to obtain passing checks.

`check-format.py` complements Cargo's formatting discovery with explicit Rust sources. `test_gates.py`, run by verify.sh, verifies all feature selections and fail-fast behavior using command observers, and rejects deliberately unformatted macro includes/fixtures in temporary crates even when cargo fmt passes. Both toolchains also compile a cfg-only probe to ensure the Windows child observer is absent without protocols and present for every enabled profile. This checks the declaration's conditions without executing a Windows process.

Both verification entrypoints export `PI_RPC_TEST_BASH` from their current Bash executable, converting it to an absolute native .exe path in Git Bash on Windows. They replace any inherited selector and reuse that executable for Python gate observers and the nested matrix. Python rejects missing, relative or unusable paths and never searches for a bare bash name. This variable configures verification tools only. To run test_gates.py directly, supply the same native absolute Bash path in `PI_RPC_TEST_BASH`.

Shell-selection regressions place a synthetic WSL Bash first on PATH while the declared entrypoint shell still executes both observers. Windows path conversion covers spaces and existing/lacking .exe suffixes. Later-row failure controls run both matrix modes, succeed through the profile-free row, then inject check/test/Clippy failures in the Pi13-only row and require the exact successful prefix with no later commands or type checker. These deterministic fixtures do not start WSL or Pi and are not Windows runtime acceptance.

Temporary Shell scripts use explicit UTF-8/LF output; Git attributes do not control Python-created fixtures. Both toolchains emulate Windows default text translation, check the raw script bytes and execute both entrypoint observers. A deliberately CRLF-corrupted fixture is rejected before launch, independently of Bash's exit status or newline-tolerance options.

Fake-process fixtures reuse immutable executable bytes: Linux/other Unix use hard links; macOS uses symbolic links to one stable source inode; Windows retains copies and direct-child handle observations. macOS case labels live in owned directories, with independent version/ready/probe/RPC sidecars. A deterministic regression checks resolved inode identity and macOS symlink/single-hard-link invariants, avoiding the known APFS/Gatekeeper [hard-link path condition](https://github.com/rust-lang/cargo/issues/10060). Ready failures report only synthetic fixture markers and owned PIDs, never host Pi configuration or diagnostics.
