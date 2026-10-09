# Actual verification results

Current target: **Pi 1.1.0**, upgraded on 2026-10-09. See the current-release results below. The 2026-10-02 observations retained later in this file apply to the previous Pi 0.99.2 target, not the newly pinned release.

## Current release verification: Pi 1.1.0 (2026-10-09)

Verified on macOS aarch64 with rustc/cargo 1.97.1. The target is the official `v1.1.0` release commit `abe508e1b89912adde45528136c3221eb69acdd7`, resolved via GitHub's latest stable release API and tag ref. Fixed source archives were compared to the former 0.99.2 commit, with current hashes in [compatibility.md](compatibility.md).

### Actual complete matrix

`CARGO_NET_OFFLINE=true bash scripts/verify.sh` returned **exit 0**, both before and after final response-shape tightening. The final combined matrix and real-release command completed at 2026-10-09 03:07 UTC, exit 0. Full local scratch logs: `pi-upgrade/final-verification.log` and `pi-upgrade/final-release.log`.

| Features | check --all-targets | Integration tests | Unit / doctests | clippy --all-targets -D warnings |
|---|---|---|---|---|
| default | PASS | 53 passed | 2 / 3 passed | PASS |
| --all-features | PASS | 59 passed | 2 / 3 passed | PASS |
| --no-default-features | PASS | 42 passed | 2 / 3 passed | PASS |
| --no-default-features --features session_file | PASS | 48 passed | 2 / 3 passed | PASS |

**222 passing test executions across four configurations**, not 222 distinct tests. No failures or lint warnings. Crate/fake-fixture formatting, injected/native/fake-local examples and strict all-features/no-default rustdoc all passed. The added release_smoke example compiled in both subprocess-enabled all-targets checks.

### Upgrade-specific coverage and observed improvement

- Required boolean `agent_settled.aborted`: false/true round trips, missing/null/wrong types fail. Client tests observe both settlement values without inventing per-prompt correlation.
- Optional numeric tool `durationMs`: omitted/zero/fractional values, no null/wrong types, no accidental swallowing into extension JSON. Client receives the typed timing field, and malformed values terminate explicitly with Protocol.
- Native assistant/tool-result `durationMs`: preserves fields through RPC/native entries and complete SessionFile encode/parse, rejects malformed numeric fields but does not constrain unknown-role or nested open payloads.
- Former Pi 0.99.2 local executable is rejected by the real fake-process version probe; current 1.1.0 fake is accepted. Existing declared-version and configuration rejection checks remain passing.
- Real empty-session `get_last_assistant_text` initially failed the new release smoke with `Protocol("malformed known record: response")`. Isolated raw RPC observation showed `data:{}`. After accepting omitted text as None, the exact same real-release smoke passed. Protocol/client regression accepts omitted/null/string text, rejects wrong text types, missing data and non-object data. Adding array/null/scalar boundaries first reproduced Serde's permissive array-to-struct decoding, then passed after the explicit response object guard. Required tree/native nullable fields stay strict.
- Existing bounded queues, transport errors, process-exit/EOF races and 3000/1000-iteration terminal delivery regressions still pass in the complete matrix.

### Real official release acceptance, no model request

Downloaded only to scratch, not installed globally: official `pi-darwin-arm64.tar.gz`, SHA-256 **3455b13de35c15a5893cdebc922678199e90a7ce99b06cbc23f37860e90d63c7**, matched the release API digest. Executed:

```bash
CARGO_NET_OFFLINE=true bash scripts/verify-release.sh /absolute/path/to/release/pi/pi
```

**Actual output:** `Pi 1.1.0: 10 read-only RPCs passed; clean exit and terminal delivery verified`.

Through the public PiProcess/PiClient API, the executable's exact --version probe succeeded, then get_state, get_messages, get_available_models, get_available_thinking_levels, get_session_stats, get_commands, get_entries, get_tree, get_last_assistant_text and get_fork_messages all decoded. The fresh session had streaming=false, pending=0, no messages/assistant text. Close returned ProcessExit(Some(0)); the event consumer received the same terminal exactly once then None.

The script uses env -i, fresh temporary HOME/PI_CODING_AGENT_DIR/cwd/TMPDIR, disables session persistence/tools/extensions/MCP/skills/templates/themes, and submits no prompt/bash/model request. No existing credentials/global settings were read or modified. It pins Cargo build and runner output to the same explicit target directory. For npm shebang compatibility it exposes only the resolved Node executable via a temporary symlink, not the host PATH. Independent review identified both target-dir drift and missing non-system Node before this correction. After the fix, direct binary and temporary `/usr/bin/env node` wrapper smoke runs passed with CARGO_TARGET_DIR set to an alternate scratch path; that alternate path was not created. The wrapper test is synthetic Node-launch coverage, not acceptance of an installed npm package.

Remaining boundaries: paid/live prompt, tool timing/abort generation were not exercised against a model (new fields are covered by wire/client/native regressions); no remote vendor deployment, Linux/Windows or Rust 1.85 MSRV verification. License remains undecided, publish=false, no push/publication. The historical independent external harness was not rerun during this upgrade; its in-crate stress coverage was rerun.

## Real model acceptance: Claude Opus 4.6 (2026-10-09)

After the upgrade, the user explicitly requested a live model request. The installed `/opt/homebrew/bin/pi` reported **1.1.0**, and its public `get_available_models` RPC listed the configured `anthropic/claude-opus-4-6`. The requested spelling `claude-opus-4.6` is not Pi's model ID; `claude-opus-4-6` is the matching Opus 4.6, not a substituted model.

Actual command (fresh empty scratch cwd, existing Pi-managed authentication):

```bash
cargo run --offline --example model_smoke -- /opt/homebrew/bin/pi /absolute/path/to/empty-work anthropic
```

**Exit 0**, completed at 2026-10-09 03:20:25 UTC. Exactly one fixed prompt was submitted, requesting `PI_RPC_OPUS_OK`. No tool execution, automatic retry, extensions/MCP/skills/templates/themes or persistent session were enabled. Discovery used only public RPC model IDs/providers, no credential content or diagnostics were printed. Pi used its existing provider configuration/authentication; the test did not edit those files.

Actual observed output:

```text
Available claude-opus-4-6 providers: anthropic
Prompt accepted: started
Session tokens=936 reported_cost=0.00562
Reply=PI_RPC_OPUS_OK; updates=9; durationMs=6033; agent_settled.aborted=false; clean exit and terminal delivery PASS
```

The cost is Pi's reported estimate, not an independently observed invoice. Public `set_model` and `get_state` confirmed model/provider, `prompt` returned Started, events delivered agent_start, nine message updates including text deltas, assistant message_end (model=claude-opus-4-6, stopReason=stop, durationMs=6033), agent_end without retry and agent_settled with aborted=false. Reassembled text and get_last_assistant_text both matched the token. get_messages included user/assistant, final state was not streaming with pending=0. Close reported ProcessExit(Some(0)), and the event consumer saw the same terminal error exactly once followed by None.

`examples/model_smoke.rs` is a reproducible opt-in entry point with a 180-second deadline and forced cleanup on error/timeout. With no provider argument it only lists the requested model's available providers. It is compiled by the normal feature matrix, but never run as a paid request by verify.sh. No second paid request was made for local regression validation. After the example and its README/verification documentation were finalized, `CARGO_NET_OFFLINE=true bash scripts/verify.sh` returned exit 0 at 03:22:06 UTC: all four check/test/clippy configurations, formatting, examples and strict docs passed (222 test executions, zero failures). This compiles the exact paid-tested example through public crate interfaces but does not issue another model request.

This now exercises real model generation, assistant timing and non-aborted settlement, beyond the earlier read-only release smoke. Real tool timing, interrupted/aborted runs, retries, remote vendor deployment, Linux/Windows and Rust 1.85 remain outside this observation.

## Historical verification: Pi 0.99.2 (2026-10-02)

Completed 2026-10-02 on macOS aarch64, in `/Users/sang/Com/Github/pi-rpc`.
Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
No real Pi process or paid model request was used. The local executable tests/example use `tests/fixtures/fake_pi.rs`, compiled with rustc. No credentials/global configuration were read, printed or modified, and nothing was pushed, published or remotely created. Implementation was performed by the current agent without changing provider/model or spawning another implementation agent.

## Final reproducible command

```bash
bash scripts/verify.sh
```

**Actual result: exit 0**, after fixing macOS Bash 3.2's empty-array/nounset incompatibility in the first script attempt. The delivered script was itself executed successfully, not just inspected. Full output was captured in the session scratch file `pi-rpc-final-verification.log`. Earlier full matrix and example runs were also captured as `pi-rpc-feature-matrix.log` and `pi-rpc-examples-docs.log`.

**Latest final run after the independent terminal-event race repair:** 2026-10-02 13:47 UTC, `bash scripts/verify.sh` again returned **exit 0**, with full output in `pi-rpc-terminal-race-final-matrix.log` in session scratch. The matrix below reflects this repaired code. The earlier successful runs were real observations but did not expose the event-consumer-before-owner race identified by the external harness.

## Feature matrix

Each row ran the actual public crate and its integration tests, not copied source substitutes. `--all-targets` also compiles applicable examples and tests. Strict clippy had **zero warnings**, including no-default-features dead code.

| Feature flags | `cargo check FLAGS --all-targets` | `cargo test FLAGS` | `cargo clippy FLAGS --all-targets -- -D warnings` |
|---|---|---|---|
| default (no flags) | PASS | 46 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--all-features` | PASS | 51 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--no-default-features` | PASS | 35 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--no-default-features --features session_file` | PASS | 40 integration + 2 unit + 3 doctests, 0 failed | PASS |

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

- **26 injected transport tests** (`tests/client.rs`): partial/coalesced CRLF and Unicode records, immediate reading before submission, out-of-order ID responses, abort during long execution, cancellation without withdrawal, timeout and ignored late response, no auto-retry, prompt dispositions, cancelled new/switch/fork/clone, nonterminal rejection, uncorrelated parse response, malformed known event failure, explicit reader transport error, wrong-command ID failure, independent count/byte overflow, event budget release, stalled writer count/byte bounds, owner vs clone drop, admitted-write drain/close, close deadline, record limits/unterminated EOF, default UI cancellation, manual no-ack UI and stop queue race. Four added event-first regressions cover 3000 immediately failing readers, 1000 buffered event then failure sequences, EOF/task panic, and owner drop after a cancelled receive.
- **2 deterministic terminal publication unit tests** (`src/client.rs`): event channel deliberately closed while terminal publication is delayed, cancelled recv retains the error, and disappearing terminal authority returns Closed exactly once.
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

### Independent terminal-event race: reproduced, repaired, externally verified

The safe test file `/Users/sang/Documents/Codex/2026-10-02/task-2/review-harness/src/lib.rs` was read without changing the external harness. On the unchanged crate, this exact command failed at **iteration 2**:

```bash
cargo test --offline --manifest-path /Users/sang/Documents/Codex/2026-10-02/task-2/review-harness/Cargo.toml
```

Observed failure: `events.recv()` returned `None`, while `owner.wait_terminal()` returned `Transport("review transport error")`. The reader dropped its event sender before the supervisor published the terminal watch. The earlier transport-failure test called owner.wait_terminal first, so it accidentally hid this interleaving. Previous feedback-loop closure claims were therefore too broad for terminal delivery.

The fix makes `PiEvents::recv` drain the event channel and then **wait for the authoritative terminal reason**, marking the error delivered only when actually returning it. A cancelled wait does not consume the terminal error. Missing terminal authority defensively yields Closed once instead of hanging. No new queue, detached task or polling loop was introduced.

The unchanged external command then passed **twice**, each running all **3000 iterations** on four Tokio workers (0.12 s and 0.11 s test execution). It was rerun after the full final matrix as well. In-crate stress adds **3000 immediate reader failures** consumed before waiting on the owner and **1000 buffered event then failure** connections, asserting precisely `buffered events -> one matching terminal error -> None`. Additional multithread tests cover reader-task panic, EOF and owner drop, with bounded receive deadlines. Two deterministic unit tests force delayed terminal publication, cancel the waiting recv and check terminal-authority disappearance. These observations directly test the previously failing acceptance order, rather than only inspecting the repair.

The final script observed **46/51/35/40 integration tests**, plus **2 unit tests and 3 doctests in each feature configuration**, with zero failures. Every requested check --all-targets, test, clippy --all-targets -D warnings and fmt --check passed. Strict all-features/no-default docs and runnable injected/native_session examples passed. Local examples were only executed against `target/verification/fake-pi`, never a live Pi installation. No paid model requests were made. No existing source or harness work was removed. Git diff checks passed, license remains undecided, and publish=false is retained.

### Post-completion feedback-loop observation

#### Whole-result rerun after finalized requirement mapping

At 2026-10-02 **13:50 UTC**, the complete delivered result at local commit `ff62407` was checked again, not just the new race tests. `bash scripts/verify.sh` returned **exit 0**. Session scratch log: `pi-rpc-whole-result-reverification.log`. All mapped executable checks ran after the requirement-to-test mapping was finalized. The unchanged external harness was also rerun and passed its 3000 four-thread iterations in **0.11 s**. The working tree was clean before recording these observations.

| Requirement / changed public output | Check actually run over complete result | Observed outcome |
|---|---|---|
| `PiEvents::recv`: terminal error reaches consumer before owner wait | `event_consumer_gets_exactly_one_terminal_before_waiting_for_owner_3000_runs` in all four configurations, unchanged external `terminal_error_always_reaches_event_consumer` | Both passed every iteration, no premature `None` |
| Buffered events stay ordered, one terminal error, then `None` | `buffered_event_precedes_exactly_one_terminal_error_1000_runs`, `owner_drop_wakes_waiting_events_and_preserves_buffered_event_order` | Passed in all four configurations |
| Delayed terminal publication and cancelled recv retain delivery | `closed_event_channel_waits_for_publication_and_cancelled_recv_keeps_error` | Passed in all four configurations |
| Missing terminal authority does not hang or repeat an error | `terminal_authority_disappearing_after_event_close_returns_closed_once` | Passed in all four configurations |
| EOF and reader panic wake event consumer with live owner | `event_first_eof_and_panicking_reader_do_not_hang_with_live_owner` | Passed with bounded receives in all four configurations |
| Bounded transport, explicit overflow and released budgets | Count/byte overflow, stalled-writer bounds and event-budget-release tests in `tests/client.rs` | All passed in all four configurations, no silent-drop acceptance |
| Matching caller/owner reasons and remaining RPC lifecycle | All 26 client integration tests, including 100-run wrong-command/same-ID race, cancellation, timeout, close, UI and stop-race tests | All passed in each configuration |
| Pinned protocol surface and required fields | All 9 protocol tests, exhaustively covering 33 commands/success schemas, 24 event kinds, 12 deltas, nullable and open tool payload validation | All passed in each configuration |
| Default subprocess lifecycle and process races | All 11 subprocess tests, real fake-executable pipes, descendant-held stdout, living-child EOF, cleanup and version probes | Passed in default and all-features; intentionally absent without subprocess feature |
| Optional native v3 session-file API | All 5 native session tests covering 11 entry kinds, branching, required-nullable fields, legacy rejection and bounds | Passed in all-features and session-only; intentionally absent without session_file |
| Public examples and README code | Injected example in no-default/all, native example in session-only/all, local example in default/all, three README doctests per configuration | Every invocation exited 0; both fake-backed local runs printed `streaming=false pending=0` and process exit code `Some(0)` |
| README/compatibility terminal semantics and generated public docs | README doctests, `RUSTDOCFLAGS='-D warnings' cargo doc` for all-features/no-default, terminal behavior checks above | All passed; generated `target/doc/pi_rpc/index.html`; prose claims match observed terminal checks |
| Complete feature/build/lint matrix | Four `check --all-targets`, four `test`, four `clippy --all-targets -- -D warnings`, crate and fake-fixture fmt | All passed, zero warnings/failures; 46/51/35/40 integration + 2 unit + 3 doctests per configuration |
| Preserve user constraints and local delivery | Manifest inspected, Git diff/status checks and local history | `publish=false`, no license invented; clean tree at `ff62407`, no remote action or live/paid Pi request |

This rerun includes the earlier protocol, transport, subprocess, native-session, examples and documentation outputs as well as the latest receiver repair. It verifies the requested local fake/mock acceptance workflow. It does not extend evidence to live model requests, vendor deployment, other OSes or the unexercised Rust 1.85 minimum, whose limitations remain below.

At 2026-10-02 13:42 UTC, the delivered `bash scripts/verify.sh` was executed again and returned **exit 0**. All four feature-matrix test counts remained 42/47/31/36 integration tests plus three doctests each, with zero failures. Both local-example invocations again observed `streaming=false pending=0` and `ProcessExit(Some(0))`, rather than relying on source inspection.

The six independent-review regression tests were then run individually through the public crate and each passed: descendant-held stdout resolved exit and pending command (0.41 s including fake compilation), stdout closed by a living child resolved EOF and reaping (0.42 s including fake compilation), wrong-command/same-ID delivered matching Protocol reasons over 100 multithread iterations (0.01 s), required-nullable RPC/native fields rejected absence but accepted null, missing open tool payload fields/nested malformed deltas were rejected, and native file parsing rejected absent parentId/replacement. `git diff --check` passed and the working tree was clean before recording this observation. The SDK's executable acceptance checks therefore continue to satisfy the requested mock/fake-process workflow after the review fixes. Paid live model behavior remains deliberately untested.

There are **no unresolved build/test/lint blockers on this machine**. Live paid-model acceptance was deliberately not attempted. The stop continuation discrepancy is tested as client behavior with a mock race, not as a live upstream model execution. Remote Sandbox integration is the generic real AsyncRead/AsyncWrite boundary tested with Tokio byte pipes, not a particular vendor SDK deployment.

Only macOS aarch64 with the stated toolchain was exercised. The manifest's Rust 1.85 minimum was not separately tested with an installed 1.85 toolchain, and Windows/Linux process behavior was not run here. Native timestamps are preserved/presence-checked, not calendrically parsed. Native v1/v2 migration, complete Pi model-context projection, vendor history conversion and remote session delivery are intentionally excluded. Child ownership is direct-child only, not detached descendants. Exit output after the bounded 50 ms drain is not guaranteed. Owner cleanup requires a live Tokio runtime; explicit close/terminate is preferred before runtime shutdown. All are documented in README/compatibility notes. License remains undecided and `publish = false` remains set.
