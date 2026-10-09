# pi-rpc

Rust import: `pi_rpc`. A Tokio async JSONL client pinned to **Pi 1.1.0**, official commit [`abe508e1b89912adde45528136c3221eb69acdd7`](https://github.com/earendil-works/pi/tree/abe508e1b89912adde45528136c3221eb69acdd7). This is a new, unpublished library. **License undecided**: no LICENSE or license grant has been invented. See [source coverage and compatibility](docs/compatibility.md) and [verification](docs/verification.md).

## Features and ownership

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

The executable is probed with `--version` before `--mode rpc`; only trimmed `1.1.0` is accepted. Startup timeout and probe output are bounded. The executable inherits the environment. This crate never reads or edits Pi/global configuration or credentials, but Pi itself may use its configured environment/resources. CLI options, isolation and access policy belong to the host. No PTY is ever allocated.

`examples/local.rs` accepts an optional executable path argument, enabling the same example to run against the test fake rather than a configured Pi installation.

Stderr is continuously drained to a bounded byte ring (default 64 KiB, zero disables retention). `process.diagnostics().snapshot()` is explicit opt-in and may expose sensitive Pi diagnostics. There is no default logging of diagnostics, prompts, messages, history or credentials.

## Commands, acceptance and errors

Every command has a typed convenience method, including `prompt_with`, `steer`, `follow_up`, `abort`, `clear_queue`, state/model/thinking/configuration queries, compaction, bash, and session/tree operations. `clone_session` names the wire `clone` command. `command(Command)` returns the validated envelope, and `typed::<T>(Command)` decodes a selected success payload. Provider model metadata and tool/extension payloads remain open JSON, rather than a hardcoded provider SDK schema.

`PromptDisposition::{Started, Queued, Handled}` and `QueuedInputDisposition::{Queued, Handled}` mean acceptance, **not completion**. Handled input need not start a run. New/switch/fork/clone results retain `cancelled`; success is not an unconditional session change.

Session events generally have **no command ID**. Only direct bash output can repeat its originating ID. There are no per-prompt event streams or `prompt_and_wait` convenience that pretends unrelated session events belong to a prompt. `agent_end` is a low-level run boundary, not `agent_settled`. `agent_settled.aborted` reports whether that run ended because it was aborted. Settlement is a session-run continuation boundary, **not evidence the queues are empty**. Optional `durationMs` on tool completion and native assistant/tool-result messages preserves Pi's monotonic execution timing. Observe `queue_update` and `get_state` separately.

`Error` distinguishes Pi rejection, uncertain-outcome timeout, protocol errors, transport errors, process exit, capacity, event overflow, EOF, explicit close, close timeout and forced termination. Known malformed records fail the connection, never become `Unknown`. Unknown event types retain their complete JSON. Rejection text comes from Pi and is not automatically logged. The owner exposes `terminal_reason()` and `wait_terminal()`. Events drain accepted buffered records, then deliver the terminal error once, then return `None`.

Event-channel closure can race ahead of the I/O supervisor's terminal publication. `PiEvents::recv` waits for the authoritative terminal watch after draining the channel, rather than returning premature `None`. This wait is cancellation-safe. Reader-task panic, transport EOF and owner drop all publish terminal reasons, so consumers do not need to call `wait_terminal` first. If the terminal authority itself disappears without publishing, the defensive fallback is one `Closed` error, then `None`.

Cancellation of an awaiting future **does not withdraw or retry its command**. The record may already be queued or sent. Its pending-map entry is removed, and later matching-ID responses are ignored. A timeout starts after local enqueue and includes the write/flush wait, so even a stalled write has an uncertain outcome. Never automatically retry input. A local `Capacity` error occurs before enqueue and means that call was not submitted.

## Bounds and backpressure

`Config` bounds record bytes, pending command count, queued writer record count, queued/in-flight writer bytes, event count and total event wire bytes. All limits must be positive and fit Tokio permit counts. Admission uses bounded, nonblocking queues: overload returns `Capacity`, rather than parking unlimited submitted input in a business queue. Byte budget remains reserved through write/flush. Reader and writer run independently, with no mutex held while awaiting a response, so a long-running bash/compaction command does not block submitting abort.

Event byte accounting uses the original serialized UTF-8 record size, excluding LF/CR. Count limits also bound per-event representation overhead. The reader adds at most one parsed record and a bounded framing chunk beyond the configured queues. If event count or bytes overflow, or the event consumer disappears, the connection fails explicitly with `EventOverflow`, notifies the owner and pending commands, closes I/O, and local process supervision terminates/reaps the child. There are no silent drops, blocking the continuous reader on the consumer, or infinite buffers.

## Extension UI

Default `UiPolicy::Cancel` enqueues `cancelled: true` for **every dialog**, including confirm. It never approves unattended actions. `Manual` leaves select/confirm/input/editor decisions to the consumer:

```rust,no_run
# async fn example(client: pi_rpc::PiClient, request: pi_rpc::protocol::UiRequest) -> pi_rpc::Result<()> {
use pi_rpc::protocol::UiAnswer;
client.respond_ui(&request, UiAnswer::Cancel).await?;
# Ok(()) }
```

`respond_ui` uses the received request ID, validates response kind and select choices, writes exactly one UI response, and awaits local flush only. It **does not enter the normal pending map or wait for an ack**. Host UI policy should avoid duplicate replies and respect Pi-side timeouts. notify/setStatus/setWidget/setTitle/set_editor_text are one-way and must not be replied to. They are delivered as UI events in both policies.

## Stop, close and terminate are different

- `clear_queue`: RPC removes queued texts and returns them.
- `abort`: RPC interrupts current work and awaits Pi's handling response. Does not implicitly clear queues.
- `PiConnection::close` / `PiProcess::close`: exclude all submissions, drain locally queued writes, close stdin, keep reading until peer EOF/process exit or close deadline. Does not send abort/clear_queue. On close timeout I/O is stopped and a local child is killed by supervision. Remote close cannot terminate a remote process.
- `PiProcess::terminate`: explicitly signal forced local termination and await reaping. Not an abort RPC.

An outer gateway **PiAdapter** owns Conversation/SSE correlation, input vs resume vs new-session policy, remote delivery, and business stopping policy. To stop: exclude new submissions at the adapter, **await `clear_queue`, then await `abort`**, optionally terminate. This sequence is **not atomic**. In-flight inputs, extensions and Pi can enqueue between calls. Track returned texts and subsequent queue updates, and enforce admission before stopping. The pinned docs and implementation disagree about queued continuation after abort. Race tests intentionally show a nonempty queue after settlement. Do not promise automatic drain.

## Native sessions

There is **no `import_history` RPC**. With `session_file`, `SessionFile` builds/parses native Pi v3 records, keeps native roles, system sections/tools, opaque provider metadata, usage, compaction and branch checkpoints, custom entries and context edits. v1/v2 are explicitly rejected, rather than pretending to implement Pi's migration. Input file bytes, record bytes and entry count are bounded. Tree IDs/parent links and branch-relative compaction/context-edit references are validated. `branch` returns raw entries, not reconstructed model context. Unknown persisted entry types are rejected in this version; arbitrary custom entry data and custom message roles remain open.

Only caller-supplied bytes are parsed; there is no automatic session discovery, OpenAI/Anthropic conversion or history-as-prompt. Delivering bytes to a remote sandbox, selecting `--session`, and filesystem permissions/atomic writes are the host's job. See `examples/native_session.rs`.

Required-but-nullable fields distinguish absence from explicit null: cycle response `data`, entries/tree `leafId`, native `parentId` and context-edit `replacement`. Tool `args`, `partialResult` and `result` accept arbitrary JSON including null, but missing properties are protocol errors. Native timestamps are carried verbatim and checked for presence, not calendrical correctness. `get_last_assistant_text` accepts omitted `text` as well as null because real Pi omits it for an empty session, despite its TypeScript response declaration. The caller owns timestamp generation and valid Pi-native identifiers.

## Development

See `docs/verification.md` for actual feature-matrix commands/results. Tests use Tokio duplex pipes and a fake executable. They make no paid Pi requests. This package has `publish = false`, and no remote repository is required.

For opt-in, non-paid acceptance against a real **1.1.0** executable, run `bash scripts/verify-release.sh /absolute/path/to/pi`. It executes ten read-only RPCs through this crate and verifies orderly exit plus terminal-event delivery. The script clears inherited environment variables, uses temporary HOME/agent/work directories, and disables sessions, tools, MCP, extensions, skills, templates and themes. It does not install Pi or submit a prompt. The normal `verify.sh` remains offline/fake-only.

For **opt-in paid model acceptance** with your existing Pi credentials, create an empty work directory and run `cargo run --example model_smoke -- /absolute/path/to/pi /absolute/path/to/empty-work anthropic`. This selects Claude Opus 4.6, whose Pi ID is **`claude-opus-4-6`** (not `claude-opus-4.6`), disables tools/extensions/session saving/automatic retry, and sends one fixed short prompt. Omit the provider argument to list available matching providers without a model request. It verifies acceptance, streaming text, assistant completion/timing, non-aborted settlement, state/messages and clean terminal delivery. Unlike `verify-release.sh`, this deliberately lets Pi use its configured environment and authentication. It never prints model metadata, credentials or raw diagnostics. The default verification script never runs this paid example.

### Broader real-Pi acceptance

All four examples below are opt-in and subprocess-feature gated. They are compiled, but **not run**, by `verify.sh`. Pi setting commands can persist configuration. Never point them at your normal agent directory. Use an absolute scratch `ROOT`, set `PI_CODING_AGENT_DIR=ROOT/agent`, and prepare private copies of `auth.json` and, if required, `models.json` there without printing their contents. Give the directory mode 700 and files mode 600. Each run needs fresh work/session directories. No private files belong in this repository.

| Example | Arguments after `--` | Actual checks |
|---|---|---|
| `rpc_state_smoke` | `/path/pi ROOT` | Queries, model/thinking/queue modes, setting toggles, actual bash output and active bash cancellation, native entries/tree/since, new session. No model request. Unsaved bash-only export/clone restrictions are explicitly reported. |
| `rpc_live_smoke` | `/path/pi ROOT` | **Paid Opus 4.6** tool execution, prompt queue acceptance, steer/follow-up consumption, exact queue removal, active abort, default manual compaction, fork and switch. Only its explicit local gate tool is enabled. It writes dedicated scratch settings with `keepRecentTokens=0` so the small test conversation can compact. |
| `rpc_session_smoke` | `/path/pi ROOT ROOT/session.jsonl [--compact]` | Name, HTML file, clone, fork, new session and switch with exact history/identity checks. Existing session must be inside ROOT. No additional generation unless **paid** `--compact` is supplied; that option requires an appropriate small keepRecentTokens setting in the dedicated agent. |
| `rpc_extension_smoke` | `/path/pi /fresh/scratch/cwd` | Nine UI methods under manual and default-cancel policies, actual extension-received answers, retry recovery/active cancellation/disabled behavior and automatic compaction. Requires `PI_CODING_AGENT_DIR` and `PI_CODING_AGENT_SESSION_DIR` inside its scratch cwd. Uses an in-memory synthetic provider and extension-provided automatic summary, **not hosted-model fault evidence or Opus summarization**. |

Use `cargo run --offline --example NAME -- ...`. Existing provider authentication is handled only by Pi. Settings isolation does not guarantee a readonly authentication copy: Pi may refresh tokens in that private copy. Scratch authentication/configuration cleanup remains the caller's responsibility. See the dated [per-RPC evidence and limitations](docs/verification.md#broader-real-rpc-acceptance-2026-10-09).
