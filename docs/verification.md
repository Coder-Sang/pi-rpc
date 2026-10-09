# Actual verification results

Current target: **Pi 1.1.0**, upgraded on 2026-10-09. The sections below are dated observations, not cumulative claims that older limitations still apply. Local scratch log names identify maintainer evidence; the logs and private test environments are not distributed with this crate. See the [current status summary](#current-status-summary) and [public-library readiness](#public-library-readiness-and-rust-185-series-verification-2026-10-09). The 2026-10-02 archive applies only to Pi 0.99.2.

## Current status summary

- Four-feature offline check/test/strict-clippy matrix: PASS, **222 test executions**, not 222 distinct tests. Strict docs and examples also passed.
- Real Pi 1.1.0: **33/33 command names executed**, with real Opus 4.6 queue/tool/abort/manual-compaction evidence and session/bash/export effects. Not all variants, state transitions or events are covered.
- Retry and automatic compaction: real Pi with **synthetic provider errors/usage and an extension summary**, not production Anthropic fault or automatic Opus-summary evidence.
- Rust **1.85.1**: all-features/all-targets check and **59 integration + 2 unit + 3 doctests** PASS. Exact 1.85.0 and other OSes remain untested.
- Packaging: `cargo package --offline` created and verified the source archive. License is still undecided, `publish=false`, repository metadata/CI remain absent, and no publication was attempted.
- Safety incident: early state tests persisted four global settings without recording their previous values. The incident and subsequent isolated checks are preserved [below](#isolation-incident-and-subsequent-safeguards).

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

Scope of this read-only run: no paid prompt, real tool execution or active model abort. Those were exercised later in the broader RPC acceptance below. The historical independent external harness was not rerun during the upgrade; its in-crate stress coverage was rerun. Cross-platform and remote-vendor boundaries remain; the later public-readiness section records Rust 1.85.1 checks. License remains undecided, publish=false, no push/publication.

## Real model acceptance: Claude Opus 4.6 (2026-10-09)

After the upgrade, the user explicitly requested a live model request. The installed `/opt/homebrew/bin/pi` reported **1.1.0**, and its public `get_available_models` RPC listed the configured `anthropic/claude-opus-4-6`. The requested spelling `claude-opus-4.6` is not Pi's model ID; `claude-opus-4-6` is the matching Opus 4.6, not a substituted model.

Actual command (fresh empty scratch cwd, existing Pi-managed authentication):

```bash
cargo run --offline --example model_smoke -- /opt/homebrew/bin/pi /absolute/path/to/empty-work anthropic
```

**Exit 0**, completed at 2026-10-09 03:20:25 UTC. Exactly one fixed prompt was submitted, requesting `PI_RPC_OPUS_OK`. No tool execution, automatic retry, extensions/MCP/skills/templates/themes or persistent session were enabled. Discovery used only public RPC model IDs/providers, no credential content or diagnostics were printed. Pi used its existing provider configuration/authentication. The harness itself did not read/write those files, but its `set_auto_retry(false)` and `set_auto_compaction(false)` RPCs cause Pi to persist settings. It is therefore incorrect to infer that this run left existing settings untouched. Future runs should use a private agent directory as documented in README.

Actual observed output:

```text
Available claude-opus-4-6 providers: anthropic
Prompt accepted: started
Session tokens=936 reported_cost=0.00562
Reply=PI_RPC_OPUS_OK; updates=9; durationMs=6033; agent_settled.aborted=false; clean exit and terminal delivery PASS
```

The cost is Pi's reported estimate, not an independently observed invoice. Public `set_model` and `get_state` confirmed model/provider, `prompt` returned Started, events delivered agent_start, nine message updates including text deltas, assistant message_end (model=claude-opus-4-6, stopReason=stop, durationMs=6033), agent_end without retry and agent_settled with aborted=false. Reassembled text and get_last_assistant_text both matched the token. get_messages included user/assistant, final state was not streaming with pending=0. Close reported ProcessExit(Some(0)), and the event consumer saw the same terminal error exactly once followed by None.

`examples/model_smoke.rs` is a reproducible opt-in entry point with a 180-second deadline and forced cleanup on error/timeout. With no provider argument it only lists the requested model's available providers. It is compiled by the normal feature matrix, but never run as a paid request by verify.sh. No second paid request was made for local regression validation. After the example and its README/verification documentation were finalized, `CARGO_NET_OFFLINE=true bash scripts/verify.sh` returned exit 0 at 03:22:06 UTC: all four check/test/clippy configurations, formatting, examples and strict docs passed (222 test executions, zero failures). This compiles the exact paid-tested example through public crate interfaces but does not issue another model request.

This minimal run exercised real model generation, assistant timing and non-aborted settlement, beyond the earlier read-only release smoke. It did not itself exercise real tools, active abort or retries; later sections add that evidence and the Rust 1.85.1 checks. Cross-platform and remote-vendor deployment remain unverified.

## Broader real RPC acceptance (2026-10-09)

Executed the installed **Pi 1.1.0** on macOS aarch64 through the public SDK, following the user's request to test the remaining RPCs. Across the examples below **33/33 command names now have real-Pi execution evidence**. This is not 33 fully exercised feature state machines or all 24 events observed from Anthropic. Retry and automatic compaction use a real Pi process with synthetic provider fault/usage injection, explicitly separated from hosted-model evidence.

### Per-command acceptance map

| RPC commands | Actual effect and acceptance source |
|---|---|
| `get_state`, `get_messages`, `get_available_models`, `get_available_thinking_levels`, `get_session_stats`, `get_commands`, `get_entries`, `get_tree`, `get_last_assistant_text`, `get_fork_messages` (10) | Existing release smoke plus new state/session/live tests. Empty and populated histories, selected model identity, nonempty native trees, every-entry exclusive `since` suffix, user fork text and final assistant marker. Extension test confirms discovered explicit slash command. |
| `set_model`, `cycle_model`, `set_thinking_level`, `cycle_thinking_level`, `set_steering_mode`, `set_follow_up_mode` (6) | `rpc_state_smoke`: actual non-null model cycle followed by matching state, restore Opus, thinking cycle followed by matching state, both queue-mode values followed by matching state. Multi-input batching under each mode is not separately tested. |
| `set_auto_compaction`, `set_auto_retry`, `abort_retry` (3) | State toggles and independent extension workflow. Retry enabled: synthetic 429 then 503 then success, attempt 1/2 start and successful end. Active abort: call after real auto_retry_start while streaming, cancelled end, no second provider call, settlement before the original delay expires. Disabled: one provider error, no retry events. `get_state` omits autoRetryEnabled, so its behavior is checked rather than inventing a state field. Auto compaction threshold: synthetic usage plus extension summary triggers actual start/end and native compaction entry. |
| `bash`, `abort_bash` (2) | Real shell `printf` output/exit 0 and streaming update. `printf START; sleep 30`, then abort only after START is observed: bash result `cancelled=true` and preserved output. Only fixed scratch shell commands are used. |
| `prompt`, `steer`, `follow_up`, `clear_queue`, `abort` (5) | Real **anthropic/claude-opus-4-6** with one explicitly loaded local gate tool. Prompt Started; streaming prompt followUp Queued; queued steer/follow_up acknowledged. Clear returns exact one steering/two follow-up inputs, none enters history. Replacement steering/follow-up enters actual history, final response PI_FOLLOWUP_OK, pending=0. New tool invocation interrupted after a tool update: real agent_settled.aborted=true and idle state. Slash extension input separately returns Handled with no model inference. |
| `compact` (1) | Default **Opus 4.6** summarizer, no summary hook loaded. Nonempty summary, positive tokensBefore, persisted native compaction entry matching the result, idle state. Final isolated session test also requires successful compaction_end before clean shutdown. |
| `new_session`, `switch_session`, `fork`, `clone`, `set_session_name`, `export_html` (6) | `rpc_session_smoke`: new empty identity, clone different identity with identical projected history, fork native user entry before the selected input with matching returned text, switch restores original identity/name/history, name visible in state, real HTML file >1 KB containing `<html`. Tested both a real error conversation and a populated Opus conversation after compaction. Actual exported file sizes: 279358, 293094 and final 292346 bytes. |

### Real model observations and failures, not hidden

- Two early live tool requests returned provider **Request timed out**, assistant usage/cost zero. A minimal no-tool Opus probe then passed with **929 tokens, estimated $0.005405**. Subsequent real tool workflows passed.
- `live-run-4`: tool durationMs=23, queue consumption/removal and active abort all PASS, **7112 tokens / estimated $0.04414**. Manual compact initially rejected `Nothing to compact (session too small)`. Project-local settings were insufficient for this run. Moving keepRecentTokens=0 to the **dedicated scratch agent** allowed default Opus compaction, tokensBefore=2157, estimated summary cost **$0.048535**. The first session harness then hit an incorrect post-compaction user-message assertion. Raw user entries remain available for fork, but projected get_messages can contain only the compaction summary. Corrected the assertion and reran all session operations successfully without another model call.
- `live-run-5`: tool durationMs=22, queue consumption/removal and active abort all PASS, **7064 tokens / estimated $0.04168**. Its inline compact call hit the 180-second command deadline. It is **not recorded as an exit-0 combined live smoke**. The uncertain request was not blindly repeated on a live process: the process was explicitly terminated/reaped, and no compaction entry existed. Reopened that saved scratch session for the final standalone default Opus compact/session workflow: **exit 0**, tokensBefore=2055, summary 1544 UTF-8 bytes, estimated summary cost **$0.03787**, matching persisted entry and successful compaction_end. All subsequent session operations and exact single-terminal clean shutdown passed.
- Sum of the successful reported generation/summary costs in this round is approximately **$0.17763**. This is Pi's estimate, not an invoice or a guarantee that timed-out upstream requests incurred no charge.

### Real extension UI and controlled failure injection

`rpc_extension_smoke` loads explicit checked-in fixtures with `--offline --no-session --no-tools --no-context-files`, no hosted API requests. All six scenarios ran from fresh scratch directories **three consecutive times, each exit 0**:

```text
UI manual: all 9 method payloads and extension-received answer markers PASS
UI cancel: all 9 method payloads and extension-received answer markers PASS
Retry recover: real Pi + synthetic upstream fault injection PASS
Retry abort: real Pi + synthetic upstream fault injection PASS
Retry disabled: real Pi + synthetic upstream fault injection PASS
Auto compaction threshold: real Pi + synthetic usage, extension-provided summary (not Opus summarizer) PASS
Pi 1.1.0 real extension UI and synthetic retry acceptance: no paid request; clean exits PASS
```

Select/confirm/input/editor request IDs and payloads were checked, manual answers returned to the actual extension, and the extension acknowledged exact received values via notifications. Default Cancel returned undefined/false without manual responses. notify/status/widget/title/editor-text one-way payloads were all observed without acknowledgements. This verifies the SDK-to-Pi extension bridge, not a host's rendered graphical UI. Automatic compaction used synthetic 35000-token usage, a small-context fixture model and an extension-provided summary, with threshold reason, success flags and native fromHook=true entry checked. It does **not** demonstrate Anthropic rate limiting, production retry reliability, or Opus automatic summarization.

### Isolation incident and subsequent safeguards

**Important side effect:** the first two no-model state-test attempts inherited the normal Pi agent directory before persistent-setting behavior was recognized. They left **steeringMode=one-at-a-time, followUpMode=one-at-a-time, compaction.enabled=false and retry.enabled=false** in global settings. Source audit: agent-session.ts setSteeringMode/setFollowUpMode/setAutoCompactionEnabled/setAutoRetryEnabled call persistent SettingsManager setters. Default provider/model/thinking are **not** persisted by these RPCs because they do not pass `options.persist`; their Opus/off selection was session-only. Original values were not saved, so no speculative restoration was made. This incident was disclosed in the chat. All subsequent setting tests enforce scratch agent paths and use private opaque auth/models copies where needed, without reading/printing credential contents. A SHA-1 file-integrity check of the user's settings after the incident remained unchanged throughout subsequent isolated runs. This proves no later modification, not restoration of the original settings.

All saved sessions, HTML, tool gates and shell work are scratch-only. Examples are feature-gated and not executed by normal verification. Live/session exercises capture assertion-task failures and explicitly terminate/reap the process before returning, rather than relying on runtime-shutdown Drop. Credentials can be refreshed only in the private copies. Cleanup of retained scratch auth copies remains the caller's responsibility.

Local evidence logs: `pi-upgrade/rpc-state-smoke-20261009-1146.log` (isolated state, exit 0), `pi-full-rpc/simple.log`, `pi-full-rpc/live-run-{2,3,4,5}.log`, `pi-full-rpc/session-{compact,after-compact,compact-final}.log`, and `rpc-extension-final-{1,2,3}/acceptance.log`. Failed attempts are retained and distinguished above. Final `CARGO_NET_OFFLINE=true bash scripts/verify.sh` after process-cleanup corrections returned **exit 0** (`pi-full-rpc/verification-cleanup-final.log`): four check/test/clippy configurations, **222 test executions**, formatting, offline/fake examples and strict rustdoc all passed. The newly added examples compiled in both subprocess-enabled configurations. Final diff whitespace check passed, no acceptance-example processes remained, and the post-incident settings integrity check was still unchanged.

### Post-commit acceptance feedback

At 03:53 UTC the coordinator reran the committed state, session and extension examples against `/opt/homebrew/bin/pi` in isolated scratch environments, without additional paid requests. **All three returned exit 0.** `closure-state.log` again observed actual bash START before cancellation and exact state/entry/tree checks. `closure-session.log` checked the real Opus session after default compaction: clone/fork/new/switch identity and history checks passed, actual HTML **292518 bytes**, clean exit and unique terminal. `closure-extension.log` again passed both nine-method UI policies, retry recovery/active cancellation/disabled behavior and automatic threshold compaction. The original saved session contained **5 Opus assistant messages, 2 actual tool results and 1 nonempty native default compaction**. The user's post-incident settings checksum was unchanged and no acceptance-example processes remained. This extends the former 15-command read-only/minimal-prompt evidence to all 33 command names with observable real-Pi effects, while retaining the synthetic-source distinctions and incident limitations above.

Remaining limits: not every command variant, optional image field, cancellation hook, queue race, all-mode batching or all 24 events received from a hosted model. Tool coverage uses a real custom extension tool rather than all built-in tools. No real production provider retry failure was forced. Linux/Windows, exact Rust 1.85.0 and remote vendor deployment remain untested. Rust 1.85.1 was subsequently checked as described below. No push/publication or global Pi install was performed.

## Public-library readiness and Rust 1.85-series verification (2026-10-09)

The following commands were actually executed at 06:24–06:25 UTC on macOS aarch64, not inferred from `rust-version` or dependency metadata:

```bash
cargo package --offline --list
cargo package --offline
rustc +1.85 --version
cargo +1.85 check --offline --all-features --all-targets
cargo +1.85 test --offline --all-features
```

- Source archive: **34 files, 296.2 KiB uncompressed / 74.7 KiB compressed**, with package verification build **exit 0**. This was the archive before this documentation refresh, not a guaranteed size/count for future packages. `publish=false` does not prevent local packaging and remains set.
- The installed `+1.85` toolchain resolved to **rustc 1.85.1**. All-targets/all-features check passed; tests passed **59 integration + 2 unit + 3 doctests**, zero failures. This verifies 1.85.1, not the exact 1.85.0 release or other OSes.
- Cargo warned that license/license-file, documentation, homepage and repository metadata were absent. No crates.io upload, name-availability check or owner authentication was performed.
- Common credential-pattern and sensitive-filename scans across reachable Git history found no matching real credentials or committed auth/models/environment files. The retry fixture's API key is an explicitly synthetic constant. This is a limited local scan, not a proof that all secrets or privacy risks are absent.
- At 06:26 UTC the committed session and extension examples were rerun against real Pi, both **exit 0** with no additional hosted-model request: projected-history/identity checks and **292690-byte HTML** passed; both UI policies, retry recovery/active cancellation/disabled behavior and synthetic automatic compaction passed again.
- All ten existing commits' author and committer identities were unified at 06:36 UTC. Tree contents, messages and dates were verified unchanged. Local commit references in this document use the rewritten IDs; upstream Pi commit IDs and source hashes are unaffected. Rewriting authors does **not** remove private paths from older file contents.

See [public release checklist](publishing.md) for unresolved licensing, metadata, history-privacy and CI decisions. Local logs for package/MSRV checks are `pi-public-package.log`, `pi-public-msrv.log` and `pi-public-msrv-tests.log`; they are not included in the package.

## Historical verification: Pi 0.99.2 (2026-10-02)

Completed 2026-10-02 on macOS aarch64, from the repository root. All subsections in this archive describe that date and the former Pi 0.99.2 target, not the current coverage counts or remaining verification limits.
Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
No real Pi process or paid model request was used. The local executable tests/example use `tests/fixtures/fake_pi.rs`, compiled with rustc. No credentials/global configuration were read, printed or modified, and nothing was pushed, published or remotely created. Implementation was performed by the current agent without changing provider/model or spawning another implementation agent.

### Historical reproducible command

```bash
bash scripts/verify.sh
```

**Actual result: exit 0**, after fixing macOS Bash 3.2's empty-array/nounset incompatibility in the first script attempt. The delivered script was itself executed successfully, not just inspected. Full output was captured in the session scratch file `pi-rpc-final-verification.log`. Earlier full matrix and example runs were also captured as `pi-rpc-feature-matrix.log` and `pi-rpc-examples-docs.log`.

**Latest final run after the independent terminal-event race repair:** 2026-10-02 13:47 UTC, `bash scripts/verify.sh` again returned **exit 0**, with full output in `pi-rpc-terminal-race-final-matrix.log` in session scratch. The matrix below reflects this repaired code. The earlier successful runs were real observations but did not expose the event-consumer-before-owner race identified by the external harness.

### Historical feature matrix

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

### Historical requirement-to-test coverage

- **26 injected transport tests** (`tests/client.rs`): partial/coalesced CRLF and Unicode records, immediate reading before submission, out-of-order ID responses, abort during long execution, cancellation without withdrawal, timeout and ignored late response, no auto-retry, prompt dispositions, cancelled new/switch/fork/clone, nonterminal rejection, uncorrelated parse response, malformed known event failure, explicit reader transport error, wrong-command ID failure, independent count/byte overflow, event budget release, stalled writer count/byte bounds, owner vs clone drop, admitted-write drain/close, close deadline, record limits/unterminated EOF, default UI cancellation, manual no-ack UI and stop queue race. Four added event-first regressions cover 3000 immediately failing readers, 1000 buffered event then failure sequences, EOF/task panic, and owner drop after a cancelled receive.
- **2 deterministic terminal publication unit tests** (`src/client.rs`): event channel deliberately closed while terminal publication is delayed, cancelled recv retains the error, and disappearing terminal authority returns Closed exactly once.
- **9 protocol tests** (`tests/protocol.rs`): exhaustive **33** command round trips and all **33** success data schemas, all **24** known ordinary event kinds, **12** delta-only assistant variants, extensible native messages, UI answer checks, strict known shapes, required nullable fields and required open payload presence.
- **5 native session tests** (`tests/session_file.rs`, optional): all **11** entry kinds, native messages/system tools and sections, opaque metadata, context replacement object/null, native parentId absence vs null, branch edits/retain-none compaction/multiple roots, cycles/orphans/duplicates, legacy rejection, CRLF and file/record/entry bounds.
- **11 local process tests** (`tests/subprocess.rs`, default feature): real OS fake-executable pipes and fragmented replies, version mismatch/oversized/hung probes, continuously drained bounded stderr, process exit status, malformed final record, explicit terminate/reap, close deadline cleanup, event overflow cleanup, owner drop with surviving clients, descendant-held stdout after direct-child exit, and fd1 closed while direct child remains alive.

The wrong-command ID regression uses Tokio `multi_thread`, four workers and **100 iterations** per run. Both caller and connection are asserted to receive the same Protocol reason. The process race tests require terminal state within 700 ms: descendant-held stdout resolves `ProcessExit(Some(9))` and the pending command, while unexpected stdout EOF with a living child resolves `Eof` and confirms the direct child was reaped. Required nullable tests distinguish missing fields from explicit JSON null through both RPC decoding and native file parsing. Missing tool args/partialResult/result are rejected, while JSON null remains accepted.

### Historical fixes encountered and verified

- Corrected documentation/test counts from initial 32/25 assumptions to the exhaustively checked **33 commands / 24 events**.
- Explicitly delivered Protocol to a removed pending waiter on ID/command mismatch before returning, eliminating the sender-drop race. Multithread regression passes.
- Added independent process-exit wakeup with a bounded 50 ms ready-output drain, and a bounded 100 ms child-status race grace after stdout EOF.
- Added required-nullable custom deserialization for native parentId/context replacement and entries/tree/text, plus success-envelope presence checks for nullable cycle data.
- Added required open tool property presence and nested assistant update/role/content validation without closing arbitrary tool/provider/extension fields.
- Fixed one strict clippy guarded-match warning and a too-short test startup budget for copied macOS fake executables.
- Fixed the verification script for macOS Bash 3.2, then reran the full delivered script successfully.

### Historical boundaries and additional observations

#### Independent terminal-event race: reproduced, repaired, externally verified

The external test file `review-harness/src/lib.rs` was read without changing the external harness. On the unchanged crate, the command below failed at **iteration 2**. The path is a portable placeholder for that private harness, which is not included in this repository:

```bash
cargo test --offline --manifest-path /absolute/path/to/review-harness/Cargo.toml
```

Observed failure: `events.recv()` returned `None`, while `owner.wait_terminal()` returned `Transport("review transport error")`. The reader dropped its event sender before the supervisor published the terminal watch. The earlier transport-failure test called owner.wait_terminal first, so it accidentally hid this interleaving. Previous feedback-loop closure claims were therefore too broad for terminal delivery.

The fix makes `PiEvents::recv` drain the event channel and then **wait for the authoritative terminal reason**, marking the error delivered only when actually returning it. A cancelled wait does not consume the terminal error. Missing terminal authority defensively yields Closed once instead of hanging. No new queue, detached task or polling loop was introduced.

The unchanged external command then passed **twice**, each running all **3000 iterations** on four Tokio workers (0.12 s and 0.11 s test execution). It was rerun after the full final matrix as well. In-crate stress adds **3000 immediate reader failures** consumed before waiting on the owner and **1000 buffered event then failure** connections, asserting precisely `buffered events -> one matching terminal error -> None`. Additional multithread tests cover reader-task panic, EOF and owner drop, with bounded receive deadlines. Two deterministic unit tests force delayed terminal publication, cancel the waiting recv and check terminal-authority disappearance. These observations directly test the previously failing acceptance order, rather than only inspecting the repair.

The final script observed **46/51/35/40 integration tests**, plus **2 unit tests and 3 doctests in each feature configuration**, with zero failures. Every requested check --all-targets, test, clippy --all-targets -D warnings and fmt --check passed. Strict all-features/no-default docs and runnable injected/native_session examples passed. Local examples were only executed against `target/verification/fake-pi`, never a live Pi installation. No paid model requests were made. No existing source or harness work was removed. Git diff checks passed, license remains undecided, and publish=false is retained.

#### Post-completion feedback-loop observation

##### Whole-result rerun after finalized requirement mapping

At 2026-10-02 **13:50 UTC**, the complete delivered result at local commit `9534f9a` was checked again, not just the new race tests. `bash scripts/verify.sh` returned **exit 0**. Session scratch log: `pi-rpc-whole-result-reverification.log`. All mapped executable checks ran after the requirement-to-test mapping was finalized. The unchanged external harness was also rerun and passed its 3000 four-thread iterations in **0.11 s**. The working tree was clean before recording these observations.

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
| Preserve user constraints and local delivery | Manifest inspected, Git diff/status checks and local history | `publish=false`, no license invented; clean tree at `9534f9a`, no remote action or live/paid Pi request |

This rerun includes the earlier protocol, transport, subprocess, native-session, examples and documentation outputs as well as the latest receiver repair. It verifies the requested local fake/mock acceptance workflow. It does not extend evidence to live model requests, vendor deployment, other OSes or the unexercised Rust 1.85 minimum, whose limitations remain below.

At 2026-10-02 13:42 UTC, the delivered `bash scripts/verify.sh` was executed again and returned **exit 0**. All four feature-matrix test counts remained 42/47/31/36 integration tests plus three doctests each, with zero failures. Both local-example invocations again observed `streaming=false pending=0` and `ProcessExit(Some(0))`, rather than relying on source inspection.

The six independent-review regression tests were then run individually through the public crate and each passed: descendant-held stdout resolved exit and pending command (0.41 s including fake compilation), stdout closed by a living child resolved EOF and reaping (0.42 s including fake compilation), wrong-command/same-ID delivered matching Protocol reasons over 100 multithread iterations (0.01 s), required-nullable RPC/native fields rejected absence but accepted null, missing open tool payload fields/nested malformed deltas were rejected, and native file parsing rejected absent parentId/replacement. `git diff --check` passed and the working tree was clean before recording this observation. The SDK's executable acceptance checks therefore continue to satisfy the requested mock/fake-process workflow after the review fixes. Paid live model behavior remains deliberately untested.

There are **no unresolved build/test/lint blockers on this machine**. Live paid-model acceptance was deliberately not attempted. The stop continuation discrepancy is tested as client behavior with a mock race, not as a live upstream model execution. Remote Sandbox integration is the generic real AsyncRead/AsyncWrite boundary tested with Tokio byte pipes, not a particular vendor SDK deployment.

Only macOS aarch64 with the stated toolchain was exercised. The manifest's Rust 1.85 minimum was not separately tested with an installed 1.85 toolchain, and Windows/Linux process behavior was not run here. Native timestamps are preserved/presence-checked, not calendrically parsed. Native v1/v2 migration, complete Pi model-context projection, vendor history conversion and remote session delivery are intentionally excluded. Child ownership is direct-child only, not detached descendants. Exit output after the bounded 50 ms drain is not guaranteed. Owner cleanup requires a live Tokio runtime; explicit close/terminate is preferred before runtime shutdown. All are documented in README/compatibility notes. License remains undecided and `publish = false` remains set.
