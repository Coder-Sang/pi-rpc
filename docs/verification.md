# Actual verification results

Completed 2026-10-02 on macOS aarch64, in `/Users/sang/Com/Github/pi-rpc`.
Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
No real Pi process or paid model request was used. The local executable tests/example use `tests/fixtures/fake_pi.rs`, compiled with rustc. No credentials/global configuration were read, printed or modified, and nothing was pushed, published or remotely created. Implementation was performed by the current agent without changing provider/model or spawning another implementation agent.

## Final reproducible command

```bash
bash scripts/verify.sh
```

**Actual result: exit 0**, after fixing macOS Bash 3.2's empty-array/nounset incompatibility in the first script attempt. The delivered script was itself executed successfully, not just inspected. Full output was captured in the session scratch file `pi-rpc-final-verification.log`. Earlier full matrix and example runs were also captured as `pi-rpc-feature-matrix.log` and `pi-rpc-examples-docs.log`.

## Feature matrix

Each row ran the actual public crate and its integration tests, not copied source substitutes. `--all-targets` also compiles applicable examples and tests. Strict clippy had **zero warnings**, including no-default-features dead code.

| Feature flags | `cargo check FLAGS --all-targets` | `cargo test FLAGS` | `cargo clippy FLAGS --all-targets -- -D warnings` |
|---|---|---|---|
| default (no flags) | PASS | 42 integration + 3 doctests, 0 failed | PASS |
| `--all-features` | PASS | 47 integration + 3 doctests, 0 failed | PASS |
| `--no-default-features` | PASS | 31 integration + 3 doctests, 0 failed | PASS |
| `--no-default-features --features session_file` | PASS | 36 integration + 3 doctests, 0 failed | PASS |

Other commands actually run and passed:

```bash
cargo fmt --check
rustfmt --edition 2024 --check tests/fixtures/fake_pi.rs
cargo run --example injected --no-default-features
cargo run --example injected --all-features
cargo run --example native_session --no-default-features --features session_file
cargo run --example native_session --all-features
rustc --edition=2024 tests/fixtures/fake_pi.rs -o target/verification/fake-pi
cargo run --example local -- "$PWD/target/verification/fake-pi"
cargo run --example local --all-features -- "$PWD/target/verification/fake-pi"
RUSTDOCFLAGS='-D warnings' cargo doc --all-features --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --no-default-features --no-deps
git diff --check
```

Both local-example runs reported `streaming=false pending=0` and terminal `ProcessExit(Some(0))` through the same `PiProcess` API used for real Pi. In-memory/native examples exited 0. README code examples are included as crate docs and compile as three doctests. With no default features, the local subprocess snippet is intentionally feature-gated.

## Requirement-to-test coverage

- **22 injected transport tests** (`tests/client.rs`): partial/coalesced CRLF and Unicode records, immediate reading before submission, out-of-order ID responses, abort during long execution, cancellation without withdrawal, timeout and ignored late response, no auto-retry, prompt dispositions, cancelled new/switch/fork/clone, nonterminal rejection, uncorrelated parse response, malformed known event failure, explicit reader transport error, wrong-command ID failure, independent count/byte overflow, event budget release, stalled writer count/byte bounds, owner vs clone drop, admitted-write drain/close, close deadline, record limits/unterminated EOF, default UI cancellation, manual no-ack UI and stop queue race.
- **9 protocol tests** (`tests/protocol.rs`): exhaustive **33** command round trips and all **33** success data schemas, all **24** known ordinary event kinds, **12** delta-only assistant variants, extensible native messages, UI answer checks, strict known shapes, required nullable fields and required open payload presence.
- **5 native session tests** (`tests/session_file.rs`, optional): all **11** entry kinds, native messages/system tools and sections, opaque metadata, context replacement object/null, native parentId absence vs null, branch edits/retain-none compaction/multiple roots, cycles/orphans/duplicates, legacy rejection, CRLF and file/record/entry bounds.
- **11 local process tests** (`tests/subprocess.rs`, default feature): real OS fake-executable pipes and fragmented replies, version mismatch/oversized/hung probes, continuously drained bounded stderr, process exit status, malformed final record, explicit terminate/reap, close deadline cleanup, event overflow cleanup, owner drop with surviving clients, descendant-held stdout after direct-child exit, and fd1 closed while direct child remains alive.

The wrong-command ID regression uses Tokio `multi_thread`, four workers and **100 iterations** per run. Both caller and connection are asserted to receive the same Protocol reason. The process race tests require terminal state within 700 ms: descendant-held stdout resolves `ProcessExit(Some(9))` and the pending command, while unexpected stdout EOF with a living child resolves `Eof` and confirms the direct child was reaped. Required nullable tests distinguish missing fields from explicit JSON null through both RPC decoding and native file parsing. Missing tool args/partialResult/result are rejected, while JSON null remains accepted.

## Fixes encountered and verified

- Corrected documentation/test counts from initial 32/25 assumptions to the exhaustively checked **33 commands / 24 events**.
- Explicitly delivered Protocol to a removed pending waiter on ID/command mismatch before returning, eliminating the sender-drop race. Multithread regression passes.
- Added independent process-exit wakeup with a bounded 50 ms ready-output drain, and a bounded 100 ms child-status race grace after stdout EOF.
- Added required-nullable custom deserialization for native parentId/context replacement and entries/tree/text, plus success-envelope presence checks for nullable cycle data.
- Added required open tool property presence and nested assistant update/role/content validation without closing arbitrary tool/provider/extension fields.
- Fixed one strict clippy guarded-match warning and a too-short test startup budget for copied macOS fake executables.
- Fixed the verification script for macOS Bash 3.2, then reran the full delivered script successfully.

## Remaining boundaries, not hidden blockers

There are **no unresolved build/test/lint blockers on this machine**. Live paid-model acceptance was deliberately not attempted. The stop continuation discrepancy is tested as client behavior with a mock race, not as a live upstream model execution. Remote Sandbox integration is the generic real AsyncRead/AsyncWrite boundary tested with Tokio byte pipes, not a particular vendor SDK deployment.

Only macOS aarch64 with the stated toolchain was exercised. The manifest's Rust 1.85 minimum was not separately tested with an installed 1.85 toolchain, and Windows/Linux process behavior was not run here. Native timestamps are preserved/presence-checked, not calendrically parsed. Native v1/v2 migration, complete Pi model-context projection, vendor history conversion and remote session delivery are intentionally excluded. Child ownership is direct-child only, not detached descendants. Exit output after the bounded 50 ms drain is not guaranteed. Owner cleanup requires a live Tokio runtime; explicit close/terminate is preferred before runtime shutdown. All are documented in README/compatibility notes. License remains undecided and `publish = false` remains set.
