# pi-rpc

Rust import: `pi_rpc`. A Tokio async JSONL client pinned to **Pi 0.99.2**, official commit [`005af57d88ee23b33778f343a9595b32e67ff788`](https://github.com/earendil-works/pi/tree/005af57d88ee23b33778f343a9595b32e67ff788). This is a new, unpublished library. **License undecided**: no LICENSE or license grant has been invented. See [source coverage and compatibility](docs/compatibility.md) and [verification](docs/verification.md).

## Features and ownership

- `protocol`: all 32 normal commands, success-data types, 25 session/RPC event kinds, streaming deltas, nine extension UI methods, native messages and entries.
- `codec`: strict LF JSONL, CRLF decoding, fragmented/coalesced byte streams, bounded records. Unicode U+2028/U+2029 inside strings do not split records. Nonempty unterminated EOF is an error.
- `transport`: inject any ordered `AsyncRead + AsyncWrite` pair without an SDK dependency. The remote host **declares** its Pi version. There is no RPC version handshake.
- `client`: cloneable command handle, ID-correlated responses, one continuous reader, one serialized writer, one single-consumer ordered event stream.
- `subprocess` (default): non-PTY local Pi with separate stdout/stderr, bounded diagnostics, exact `--version` verification and lifecycle ownership.
- `session_file` (optional): build/parse native v3 session JSONL and inspect raw branches. Not a vendor history converter or model-context replay engine.

Requires a live Tokio runtime. Owners should be explicitly closed/terminated before shutting down that runtime. Dropping a `PiClient` clone is inert. Dropping `PiConnection` aborts its I/O tasks and drops the transport, with remaining clients receiving `Closed`. Dropping `PiProcess` also signals child termination, and its supervisor reaps the child. Explicit termination waits for cleanup. This manages the direct child, not arbitrary detached descendant process trees.

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

The executable is probed with `--version` before `--mode rpc`; only trimmed `0.99.2` is accepted. Startup timeout and probe output are bounded. The executable inherits the environment. This crate never reads or edits Pi/global configuration or credentials, but Pi itself may use its configured environment/resources. CLI options, isolation and access policy belong to the host. No PTY is ever allocated.

Stderr is continuously drained to a bounded byte ring (default 64 KiB, zero disables retention). `process.diagnostics().snapshot()` is explicit opt-in and may expose sensitive Pi diagnostics. There is no default logging of diagnostics, prompts, messages, history or credentials.

## Commands, acceptance and errors

Every command has a typed convenience method, including `prompt_with`, `steer`, `follow_up`, `abort`, `clear_queue`, state/model/thinking/configuration queries, compaction, bash, and session/tree operations. `clone_session` names the wire `clone` command. `command(Command)` returns the validated envelope, and `typed::<T>(Command)` decodes a selected success payload. Provider model metadata and tool/extension payloads remain open JSON, rather than a hardcoded provider SDK schema.

`PromptDisposition::{Started, Queued, Handled}` and `QueuedInputDisposition::{Queued, Handled}` mean acceptance, **not completion**. Handled input need not start a run. New/switch/fork/clone results retain `cancelled`; success is not an unconditional session change.

Session events generally have **no command ID**. Only direct bash output can repeat its originating ID. There are no per-prompt event streams or `prompt_and_wait` convenience that pretends unrelated session events belong to a prompt. `agent_end` is a low-level run boundary, not `agent_settled`. Settlement is a session-run continuation boundary, **not evidence the queues are empty**. Observe `queue_update` and `get_state` separately.

`Error` distinguishes Pi rejection, uncertain-outcome timeout, protocol errors, transport errors, process exit, capacity, event overflow, EOF, explicit close, close timeout and forced termination. Known malformed records fail the connection, never become `Unknown`. Unknown event types retain their complete JSON. Rejection text comes from Pi and is not automatically logged. The owner exposes `terminal_reason()` and `wait_terminal()`. Events drain accepted buffered records, then deliver the terminal error once, then return `None`.

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

## Development

See `docs/verification.md` for actual feature-matrix commands/results. Tests use Tokio duplex pipes and a fake executable. They make no paid Pi requests. This package has `publish = false`, and no remote repository is required.
