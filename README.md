# pi-rpc

Rust import: `pi_rpc`. A Tokio async JSONL client pinned to **Pi 1.1.0**, official commit [`abe508e1b89912adde45528136c3221eb69acdd7`](https://github.com/earendil-works/pi/tree/abe508e1b89912adde45528136c3221eb69acdd7). Version **0.1.0** is a technical preview prepared for crates.io; it has **not been uploaded**. Licensed under **[Apache-2.0](./LICENSE)** for this project's original code and documentation, with [third-party notices](THIRD_PARTY_NOTICES.md) for upstream material. See [source coverage and compatibility](docs/compatibility.md), [verification](docs/verification.md) and the [public release checklist](docs/publishing.md).

Declared minimum Rust: **1.85** (edition 2024). Native Linux, macOS and Windows CI passed all four feature configurations on stable and exact **1.85.0**, plus three stress rounds and Debug/Release boundary regressions; see the latest [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) and the [dated hosted verification evidence](docs/verification.md#hosted-six-job-cross-platform-acceptance-2026-10-11). Local macOS and Linux ARM64 checks supplement those results. Remote-vendor deployment remains unverified. Pi is a separately supplied executable, not installed by this crate.

## Installation

Until 0.1.0 is uploaded to crates.io, add a Git dependency to your application's `Cargo.toml`:

```toml
[dependencies]
pi-rpc = { git = "https://github.com/Coder-Sang/pi-rpc", branch = "main" }
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

This follows the development branch. Commit your application's `Cargo.lock` to retain the resolved Git revision, and update it deliberately when adopting newer changes. You can replace `branch` with `rev` to select a specific verified commit from the linked workflow runs. Rust imports use `pi_rpc`. For a sibling local checkout, replace the Git dependency with `pi-rpc = { path = "../pi-rpc" }`.

After the registry release is available, replace the Git dependency with `pi-rpc = "0.1.0"`. The registry dependency supports the same feature options below. The publishing allowlist is restricted to `crates-io`; enabling it does not upload the crate.

### Version and stability policy

The **0.1.x** series keeps the documented public paths, feature names and Rust **1.85** minimum. Patch releases preserve the public Rust API; source-breaking changes, including changes to exhaustively matched enums, require a new minor series with migration notes. Pi compatibility remains pinned to **1.1.0** until a separately reviewed protocol upgrade. Technical-preview status means deployment experience is still limited to the evidence in the verification document. See the [changelog](CHANGELOG.md) and [release policy](docs/publishing.md#version-and-release-policy).

### Cargo features

| Feature | Default | Enables |
|---|---|---|
| `subprocess` | Yes | Local `PiProcess` and `ProcessOptions`; enables Tokio process support |
| `session_file` | No | Native v3 `SessionFile` parsing, encoding and `SessionFileBuilder` |

For injected transport only, add `default-features = false` to the dependency. To enable native sessions, add `features = ["session_file"]`; keep defaults for local processes, or also set `default-features = false` for native sessions with injected transport. `protocol`, `codec`, `transport` and `client` are always available modules, not Cargo feature names. The four tested configurations are default, no defaults, `session_file` without defaults, and all features.

## Modules and ownership

- `protocol`: all 33 normal commands, success-data types, 24 session/RPC event kinds, streaming deltas, nine extension UI methods, native messages and entries.
- `codec`: strict LF JSONL, CRLF decoding, fragmented/coalesced byte streams, bounded records. Unicode U+2028/U+2029 inside strings do not split records. Nonempty unterminated EOF is an error.
- `transport`: inject any ordered `AsyncRead + AsyncWrite` pair without an SDK dependency. The remote host **declares** its Pi version. There is no RPC version handshake.
- `client`: cloneable command handle, ID-correlated responses, one continuous reader, one serialized writer, one single-consumer ordered event stream.
- `subprocess` (default): non-PTY local Pi with separate stdout/stderr, bounded diagnostics, exact `--version` verification and lifecycle ownership.
- `session_file` (optional): build/parse native v3 session JSONL and inspect raw branches. Not a vendor history converter or model-context replay engine.

Requires a live Tokio runtime. Owners should be explicitly closed/terminated before shutting down that runtime. Dropping a `PiClient` clone is inert. Dropping `PiConnection` aborts its I/O tasks and drops the transport, with remaining clients receiving `Closed`. Dropping `PiProcess` also signals child termination, and its supervisor reaps the child. Explicit termination waits for cleanup. This manages the direct child, not arbitrary detached descendant process trees.

Local child exit is monitored independently of stdout EOF. After observing exit the reader drains available records for at most 50 ms, then reports `ProcessExit`, even when a descendant holds stdout open. Stdout EOF allows up to 100 ms for the child-exit status to arrive. If the child remains alive, unexpected EOF reports `Eof` and supervision kills/reaps it. Orderly caller-initiated close remains distinct. Malformed buffered protocol records still report `Protocol`; output arriving after the bounded exit drain is not guaranteed to be delivered.

## Injected transport

```rust,no_run
use pi_rpc::{Config, PiConnection, Transport, PI_VERSION};
# async fn example<R, W>(reader: R, writer: W) -> pi_rpc::Result<()>
# where R: tokio::io::AsyncRead + Unpin + Send + 'static,
# W: tokio::io::AsyncWrite + Unpin + Send + 'static {
let transport = Transport::new(reader, writer, PI_VERSION)?;
let (client, mut events, mut owner) = PiConnection::connect(transport, Config::default())?;
// The event channel is already installed, and reading has started.
let consumer = tokio::spawn(async move {
    while let Some(record) = events.recv().await {
        match record {
            Ok(event) => { /* host handles session event, no invented prompt ID */ }
            Err(terminal) => break,
        }
    }
});
let messages = client.get_messages().await?;
let reason = owner.close().await;
consumer.await.expect("event task");
# Ok(()) }
```

Use `cargo run --example injected --no-default-features` for a runnable in-memory version with no Pi or paid requests. A remote Sandbox integration adapts its binary stdout/stdin streams here, drains stderr separately, and owns remote termination/delivery outside this crate.

## Local subprocess

```rust,no_run
# #[cfg(feature="subprocess")]
# async fn example() -> pi_rpc::Result<()> {
use pi_rpc::{PiProcess, ProcessOptions};
let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
    executable: "/absolute/path/to/pi".into(),
    args: vec!["--no-session".into()],
    ..Default::default()
}).await?;
let consumer = tokio::spawn(async move { while events.recv().await.is_some() {} });
let state = client.get_state().await?;
process.close().await;
consumer.await.expect("event task");
# Ok(()) }
```

The executable is probed with `--version` before `--mode rpc`; only trimmed `1.1.0` is accepted. Startup timeout and probe output are bounded. The executable inherits the environment. Library code does not directly read/write Pi configuration or credentials, but commands such as `set_auto_retry`, `set_auto_compaction`, `set_steering_mode` and `set_follow_up_mode` cause **Pi to persist settings**. Pi may also refresh authentication. The opt-in examples have their own documented scratch-file behavior. CLI options, isolation and access policy belong to the host. No PTY is ever allocated.

`examples/local.rs` accepts an optional executable path argument, enabling the same example to run against the test fake rather than a configured Pi installation.

Stderr is continuously drained to a bounded byte ring (default 64 KiB, zero disables retention). `process.diagnostics().snapshot()` is explicit opt-in and may expose sensitive Pi diagnostics. There is no default logging of diagnostics, prompts, messages, history or credentials.

## Commands, acceptance and errors

Every command has a typed convenience method, including `prompt_with`, `steer`, `follow_up`, `abort`, `clear_queue`, state/model/thinking/configuration queries, compaction, bash, and session/tree operations. `clone_session` names the wire `clone` command. `command(Command)` returns the validated envelope, and `typed::<T>(Command)` decodes a selected success payload. Provider model metadata and tool/extension payloads remain open JSON, rather than a hardcoded provider SDK schema.

`PromptDisposition::{Started, Queued, Handled}` and `QueuedInputDisposition::{Queued, Handled}` mean acceptance, **not completion**. Handled input need not start a run. New/switch/fork/clone results retain `cancelled`; success is not an unconditional session change.

Session events generally have **no command ID**. Only direct bash output can repeat its originating ID. There are no per-prompt event streams or `prompt_and_wait` convenience that pretends unrelated session events belong to a prompt. `agent_end` is a low-level run boundary, not `agent_settled`. `agent_settled.aborted` reports whether that run ended because it was aborted. Settlement is a session-run continuation boundary, **not evidence the queues are empty**. Optional `durationMs` on tool completion and native assistant/tool-result messages preserves Pi's monotonic execution timing. Observe `queue_update` and `get_state` separately.

`Error` distinguishes Pi rejection, uncertain-outcome timeout, protocol errors, transport errors, process exit, capacity, event overflow, EOF, explicit close, close timeout and forced termination. Known malformed records fail the connection, never become `Unknown`. Unknown event types retain their complete JSON. Rejection text comes from Pi and is not automatically logged. The owner exposes `terminal_reason()` and `wait_terminal()`. Events drain accepted buffered records, then deliver the terminal error once, then return `None`.

Event-channel closure can race ahead of the I/O supervisor's terminal publication. `PiEvents::recv` waits for the authoritative terminal watch after draining the channel, rather than returning premature `None`. This wait is cancellation-safe. Reader-task panic, transport EOF and owner drop all publish terminal reasons, so consumers do not need to call `wait_terminal` first. If the terminal authority itself disappears without publishing, the defensive fallback is one `Closed` error, then `None`.

Model failures are native message outcomes: inspect `message_end.message.fields["stopReason"]` and `errorMessage`. Pi's normal agent loop turns provider `done`/`error` into message boundaries; waiting only for a nested `AssistantEvent::Error` or a connection error misses model failures. Keep consuming through retries and `agent_settled`:

```rust,no_run
use pi_rpc::PiEvents;
use pi_rpc::protocol::{AssistantEvent, Event, KnownEvent};

async fn consume(mut events: PiEvents) -> pi_rpc::Result<()> {
    while let Some(record) = events.recv().await {
        match record? {
            Event::Known(KnownEvent::MessageUpdate {
                assistant_message_event: AssistantEvent::TextDelta { content_index, delta }, ..
            }) => println!("block {content_index}: {delta}"),
            Event::Known(KnownEvent::MessageEnd { message }) => {
                let reason = message.fields.get("stopReason").and_then(|value| value.as_str());
                if message.role == "assistant" && matches!(reason, Some("error" | "aborted")) {
                    eprintln!("model outcome: {reason:?}"); // The connection remains usable.
                }
                // Replace the accumulated message with this authoritative snapshot.
            }
            _ => { /* Handle thinking, tools, custom messages and UI in the host. */ }
        }
    }
    Ok(())
}
```

See the [event consumption guide](docs/events.md) for all event families, content-block reconstruction, custom-message display policy and the integer millisecond compatibility rules.

Cancellation of an awaiting future **does not withdraw or retry its command**. The record may already be queued or sent. Its pending-map entry is removed, and later matching-ID responses are ignored. A timeout starts after local enqueue and includes the write/flush wait, so even a stalled write has an uncertain outcome. Never automatically retry input. A local `Capacity` error occurs before enqueue and means that call was not submitted.

## Bounds and backpressure

JSON objects and arrays have an explicit maximum nesting depth of **512 containers** (`codec::MAX_JSON_DEPTH`); the root container counts as one. Strings, including escaped quotes and brackets, do not add nesting. This supports ordinary session trees of at least 200 entries along a branch, within the record byte limit; deeply nested entry metadata also counts toward the limit. Wire depth is checked iteratively before parsing, and Serde conversions use `serde_stacker` rather than the default 128-container parser limit. Over-depth inbound records fail the connection with a `Protocol` error. Native session parsing and direct `Value` decoding use the same limit. Encoding checks depth incrementally as bytes are written, stopping at the first excess container; native construction counts the complete entry/message envelope. Cloning, serialization and cleanup at the boundary are exercised on a 2 MiB thread stack in an isolated test process. This is a bounded-depth contract, not arbitrary-depth JSON support.

The `psm` transitive stack-switching dependency is constrained to 0.1.27 because newer releases pull in `ar_archive_writer` code requiring Rust 1.88 despite incomplete minimum-version metadata. The lockfile and exact 1.85.0 verification preserve this crate's declared minimum compiler.

Default RPC limits are **32 MiB per JSON record**, **64 MiB of writer bytes**, **128 MiB of queued event wire bytes**, 128 pending commands, 128 queued writer records and 1,024 events. Explicit caller limits remain authoritative. Pi 1.1.0 normally resizes each prompt image to at most 2000×2000 and below 4.5 MiB of Base64, with model overrides and an option to disable resizing. That processing happens after RPC input is received; these transport limits cover the original input too. Four images near Pi's default processed limit occupy about 18 MiB before text and JSON overhead. The SDK does not resize images or enforce an image count. Hosts must bound original image sizes and request concurrency.

Image-bearing message_start/message_end events retain their complete payload and are charged separately. Complete get_messages/get_entries/get_tree responses must fit the same record limit; accumulated history can exceed it and then terminates the connection with Protocol. Hosts should maintain history from events rather than repeatedly fetch unbounded image history. Native sessions default to 32 MiB per record and 64 MiB per file. These are wire budgets, not RSS limits: caller-owned images, decoded objects and command responses awaiting collection use additional memory.

`Config` bounds record bytes, pending command count, queued writer record count, queued/in-flight writer bytes, event count and total event wire bytes. All limits must be positive and fit Tokio permit counts. Admission uses bounded, nonblocking queues: overload returns `Capacity`, rather than parking unlimited submitted input in a business queue. A private bounded counting pass measures stable borrowed SDK records, then reserves their complete framed output before allocating it. Encoding, queued and in-flight output share the writer byte budget, held through write/flush. Writer slots and pending-command admission are also reserved before counting; rejection never allocates a full output record. Owned command inputs are released once encoded output exists, and owned UI answers after enqueue, rather than being retained while awaiting a response or flush. Public generic codec::encode remains single-pass. Failed preparation, cancellation and close races release reservations; commands already queued still run independently of an awaiting future. Reader and writer run independently, with no mutex held while awaiting a response, so a long-running bash/compaction command does not block submitting abort.

Event byte accounting uses the original serialized UTF-8 record size, excluding LF/CR. Count limits also bound per-event representation overhead. The reader adds at most one parsed record and a bounded framing chunk beyond the configured queues. If event count or bytes overflow, or the event consumer disappears, the connection fails explicitly with `EventOverflow`, notifies the owner and pending commands, closes I/O, and local process supervision terminates/reaps the child. There are no silent drops, blocking the continuous reader on the consumer, or infinite buffers.

## Extension UI

Default `UiPolicy::Cancel` enqueues `cancelled: true` for **every dialog**, including confirm. It never approves unattended actions. `Manual` leaves select/confirm/input/editor decisions to the consumer:

```rust,no_run
use pi_rpc::{Config, Error, PiConnection, Transport, UiPolicy, PI_VERSION};
use pi_rpc::protocol::{Event, UiAnswer};
# async fn example<R, W>(reader: R, writer: W) -> pi_rpc::Result<()>
# where R: tokio::io::AsyncRead + Unpin + Send + 'static,
# W: tokio::io::AsyncWrite + Unpin + Send + 'static {
let config = Config { ui_policy: UiPolicy::Manual, ..Config::default() };
let transport = Transport::new(reader, writer, PI_VERSION)?;
let (client, mut events, mut owner) = PiConnection::connect(transport, config)?;
let responder = client.clone();
let consumer = tokio::spawn(async move {
    while let Some(record) = events.recv().await {
        match record {
            Ok(Event::Ui(request)) if request.method.is_dialog() => {
                // Replace Cancel with the host user's answer, once per dialog.
                match responder.respond_ui(&request, UiAnswer::Cancel).await {
                    Ok(()) | Err(Error::Closed) => {} // Drain late dialogs during close.
                    Err(error) => return Err(error),
                }
            }
            Ok(_) => { /* Handle ordinary events and one-way UI notifications. */ }
            Err(_) => break,
        }
    }
    Ok::<(), pi_rpc::Error>(())
});
let state = client.get_state().await?;
owner.close().await;
consumer.await.expect("event task")?;
# Ok(()) }
```

For local subprocesses, pass the same manual configuration in `ProcessOptions.connection` before calling `PiProcess::spawn`. Set the policy when connecting; the default Cancel policy already sends automatic replies and should not be combined with manual dialog responses.

`respond_ui` uses the received request ID, validates response kind and select choices, writes exactly one UI response, and awaits local flush only. Text answers are serialized directly within the record byte budget, without first copying the full text into JSON. Once closing begins, replies are rejected before validation or encoding; an already published terminal error takes precedence. Admission checks the state again after encoding, so a concurrent close cannot enqueue a late reply. It **does not enter the normal pending map or wait for an ack**. Host UI policy should avoid duplicate replies and respect Pi-side timeouts. notify/setStatus/setWidget/setTitle/set_editor_text are one-way and must not be replied to. They are delivered as UI events in both policies.

Once orderly close starts, automatic dialog replies are skipped atomically with submission admission. Late dialogs remain observable events, and reading continues through subsequent command responses and peer EOF. Public `respond_ui` calls after close starts still return `Closed`. Close does not enqueue replies behind the stdin shutdown marker.

## Stop, close and terminate are different

- `clear_queue`: RPC removes queued texts and returns them.
- `abort`: RPC interrupts current work and awaits Pi's handling response. Does not implicitly clear queues.
- `PiConnection::close` / `PiProcess::close`: exclude all submissions, drain locally queued writes, close stdin, keep reading until peer EOF/process exit or close deadline. Does not send abort/clear_queue. On close timeout I/O is stopped and a local child is killed by supervision. Remote close cannot terminate a remote process.
- `PiProcess::terminate`: explicitly signal forced local termination and await reaping. Not an abort RPC.

An outer gateway **PiAdapter** owns Conversation/SSE correlation, input vs resume vs new-session policy, remote delivery, and business stopping policy. To stop: exclude new submissions at the adapter, **await `clear_queue`, then await `abort`**, optionally terminate. This sequence is **not atomic**. In-flight inputs, extensions and Pi can enqueue between calls. Track returned texts and subsequent queue updates, and enforce admission before stopping. The pinned docs and implementation disagree about queued continuation after abort. Race tests intentionally show a nonempty queue after settlement. Do not promise automatic drain.

## Native sessions

There is **no `import_history` RPC**. With `session_file`, `SessionFile` builds/parses native Pi v3 records, keeps native roles, system sections/tools, opaque provider metadata, usage, compaction and branch checkpoints, custom entries and context edits. v1/v2 are explicitly rejected, rather than pretending to implement Pi's migration. Input file bytes, record bytes and entry count are bounded. Export checks entry count and streams within record/file budgets before full semantic validation; when both budgets and semantics fail, a capacity error may be returned first. File budgets include every LF, while record budgets exclude LF. Non-finite compaction/usage/cost numbers are rejected rather than exported as null. Finite f64 values retain their IEEE-754 bits across JSON encode/parse, including signed zero and subnormal values. Flattened extension fields may not shadow reserved header, entry, message, content-block or usage fields; such caller-created records are rejected before export. Tree IDs/parent links and branch-relative compaction/context-edit references are validated. `branch` returns raw entries, not reconstructed model context. Unknown persisted entry types are rejected in this version; arbitrary custom entry data and custom message roles remain open.

Only caller-supplied bytes are parsed; there is no automatic session discovery, OpenAI/Anthropic conversion or history-as-prompt. Delivering bytes to a remote sandbox, selecting `--session`, and filesystem permissions/atomic writes are the host's job. See `examples/native_session.rs`.

For bulk message construction, use `session_file::SessionFileBuilder::new(id, timestamp, cwd)?`, or `from_session_file(file)?` to take ownership of validated existing history. Its `append_message` has the same arguments as the existing file method, validates only the new message and ID/parent, and leaves history/index unchanged on rejection. `finish()?` validates once more and returns the native `SessionFile` without cloning it. Pure message bulk construction is O(n) overall; mixed-history reference validation also uses a single forest index with constant-time ancestor checks. The builder keeps history private so its ID index cannot become stale. The original public, mutable `SessionFile` and `append_message` remain compatible and still fully validate on each append, including caller-modified historical entries. The native-session example uses the builder. Run `cargo run --release --no-default-features --features session_file --example session_builder_bench` for three-sample median comparisons at 1,000/2,000/4,000 messages; timing is informational, not a machine-specific test threshold. Use `cargo run --release --no-default-features --features session_file --example session_validation_bench` for seven-sample median mixed-history parse measurements.

Required-but-nullable fields distinguish absence from explicit null: cycle response `data`, entries/tree `leafId`, native `parentId` and context-edit `replacement`. Tool `args`, `partialResult` and `result` accept arbitrary JSON including null, but missing properties are protocol errors. Native timestamps are carried verbatim and checked for presence, not calendrical correctness. `get_last_assistant_text` accepts omitted `text` as well as null because real Pi omits it for an empty session, despite its TypeScript response declaration. The caller owns timestamp generation and valid Pi-native identifiers.

## Development

See [verification](docs/verification.md#current-status-summary) for actual commands/results. Tests use Tokio duplex pipes and a fake executable. They make no paid Pi requests. Registry publishing is allowed only to crates.io; local verification requires no registry upload or remote repository.

Development requires stable Rust with rustfmt/Clippy, separately installed exact Rust **1.85.0**, **Bash**, **Git**, **Python 3.11+**, and a native C/assembly toolchain. On Windows use Git Bash plus MSVC with the Windows SDK. Python checker unit tests invoke Git; standalone document/archive checks use only the Python standard library. Commands below run from a clean Git checkout at the repository root; Windows users can use `python` if `python3` is unavailable.

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh       # Four features, strict lint/docs, examples
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh  # Requires installed exact Rust 1.85.0
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh # Three rounds, no failure retries
CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh # Fake-only Debug and Release boundaries
RUSTUP_TOOLCHAIN=1.85.0 CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
python3 scripts/test_checks.py                     # Python 3.11+ and Git required
python3 scripts/check-docs.py .                    # Explicit root, no Git dependency
cargo package --locked --offline
python3 scripts/check-package.py target/package/pi-rpc-0.1.0.crate --source-root . --expected-commit "$(git rev-parse HEAD)"
python3 scripts/check-docs.py target/package/pi-rpc-0.1.0
```

Offline mode requires previously downloaded dependencies and toolchains. The stack-protection dependencies also require a native C/assembly toolchain (Clang/GCC, or MSVC with the Windows SDK); Rust target installation alone is insufficient for cross-compiling their native code. `verify.sh` accepts Windows Git Bash and accounts for `.exe`/native paths. All six CI combinations (Linux/macOS/Windows, stable/exact 1.85.0) run the feature matrix, three stress rounds including floating-point regressions, focused Debug/Release UI, float, stack and close regressions, and a fresh independent consumer of the packaged crate. After `cargo package --locked`, run `bash scripts/verify-consumer.sh` with stable and `RUSTUP_TOOLCHAIN=1.85.0`; it checks all four features without copying the SDK lockfile. The separate opt-in macOS/Linux real-release smoke remains disabled by default; these normal checks use only the test fake. Configuring CI is not evidence that hosted jobs passed. Package/document checks cover this repository's local Markdown link conventions and known unwanted file names, not general Markdown or an exhaustive credential audit.

The fake-only image tests cover four near-limit images, a 15 MiB original represented as 20 MiB Base64, exact record/envelope limits, image event replay and complete history overflow. For an informational seven-sample Release comparison of single-pass versus reserved two-pass encoding, run `cargo test --locked --release --all-features --lib image_encoding_measurement -- --ignored --nocapture`. It prints elapsed medians and reserved output bytes, not process RSS, and has no timing threshold.

## Real Pi acceptance

Normal verification uses only the test fake. Separate opt-in checks cover read-only Pi RPCs, synthetic extension behavior and paid model acceptance; see the [real Pi acceptance guide](docs/acceptance.md). Those examples can persist settings or modify scratch files, and require the disposable environments described there.
