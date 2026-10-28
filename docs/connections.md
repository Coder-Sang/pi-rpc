# Connections, capacity and lifecycle

The SDK manages transport admission and lifecycle. The host application owns conversation correlation, business policy and remote process control. Start with the [README](https://github.com/Coder-Sang/pi-rpc/blob/main/README.md) and use the [event guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/events.md) for streamed content.

## Modules and ownership

- `protocol`: all 33 normal commands, success-data types, 24 session/RPC event kinds, streaming deltas, nine extension UI methods, native messages and entries.
- `codec`: strict LF JSONL, CRLF decoding, fragmented/coalesced byte streams, bounded records. Unicode U+2028/U+2029 inside strings do not split records. Nonempty unterminated EOF is an error.
- `transport`: inject any ordered `AsyncRead + AsyncWrite` pair without an SDK dependency. The remote host **declares** its Pi version. There is no RPC version handshake.
- `client`: cloneable command handle, ID-correlated responses, one continuous reader, one serialized writer, one single-consumer ordered event stream.
- `subprocess` (default): non-PTY local Pi with separate stdout/stderr, bounded diagnostics, version-probed or caller-declared startup and lifecycle ownership.
- `session_file` (optional): build/parse native v3 session JSONL and inspect raw branches. Not a vendor history converter or model-context replay engine.

Requires a live Tokio runtime. Owners should be explicitly closed/terminated before shutting down that runtime. Dropping a `PiClient` clone is inert. Dropping `PiConnection` aborts its I/O tasks and drops the transport, with remaining clients receiving `Closed`. Dropping `PiProcess` also signals child termination, and its supervisor reaps the child. Explicit termination waits for cleanup. This manages the direct child, not arbitrary detached descendant process trees.

Local child exit is monitored independently of stdout EOF. After observing exit the reader drains available records for at most 50 ms, then reports `ProcessExit`, even when a descendant holds stdout open. Stdout EOF allows up to 100 ms for the child-exit status to arrive. If the child remains alive, unexpected EOF reports `Eof` and supervision kills/reaps it. Orderly caller-initiated close remains distinct. At the cutoff, a nonwaiting validation checks buffered records and up to 4 KiB of immediately readable input, preserving `Protocol` for malformed records and a ready unterminated EOF. An unfinished record at that byte boundary permits one additional byte of nonwaiting EOF lookahead; any lookahead data is retained without extending validation. This validation does not dispatch events/responses or enqueue UI replies. It cannot wait for EOF from a still-open pipe; output beyond the bounded drain is not guaranteed to be delivered.

## Injected transport

```rust,no_run
use pi_rpc::{Config, PiConnection, Transport};
# async fn example<R, W>(reader: R, writer: W, declared_version: &str) -> pi_rpc::Result<()>
# where R: tokio::io::AsyncRead + Unpin + Send + 'static,
# W: tokio::io::AsyncWrite + Unpin + Send + 'static {
// The host supplies the actual concrete Pi version, such as "1.1.0" or "1.4.0".
let transport = Transport::new(reader, writer, declared_version)?;
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

## Local process and diagnostics

`PiProcess::spawn` probes `--version` before RPC startup. The probe defaults to five seconds and bounds stdout and stderr separately to 4 KiB. Exact `expected_version` locks still probe. The 0.2.3 Git development API `spawn_with_version` skips that process and trusts the caller's concrete declaration while enforcing the same supported-version/expectation policy. Its `startup_timeout` is unused. Neither entrypoint sends initialization commands or waits for Pi readiness.

Both startup paths share CLI arguments, cwd, inherited environment, stderr draining, process supervision and cleanup. The host owns executable installation and configuration. There is no PTY. `examples/local.rs` accepts an optional executable path, including the compiled test fake.

Stderr is drained to a bounded ring, 64 KiB by default; zero disables retention. `process.diagnostics().snapshot()` is explicit access and can expose sensitive Pi diagnostics. The library does not log diagnostics, prompts, history or credentials automatically. Pi itself may persist settings after configuration commands or refresh authentication. The opt-in examples have their own documented scratch-file behavior.

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

See the [event consumption guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/events.md) for all event families, content-block reconstruction, custom-message display policy and the integer millisecond compatibility rules.

Cancellation of an awaiting future **does not withdraw or retry its command**. The record may already be queued or sent. Its pending-map entry is removed, and later matching-ID responses are ignored. A timeout starts after local enqueue and includes the write/flush wait, so even a stalled write has an uncertain outcome. Never automatically retry input. A local `Capacity` error occurs before enqueue and means that call was not submitted.

## Bounds and backpressure

JSON objects and arrays have an explicit maximum nesting depth of **512 containers** (`codec::MAX_JSON_DEPTH`); the root container counts as one. Strings, including escaped quotes and brackets, do not add nesting. This supports ordinary session trees of at least 200 entries along a branch, within the record byte limit; deeply nested entry metadata also counts toward the limit. Wire depth is checked iteratively before parsing, and Serde conversions use `serde_stacker` rather than the default 128-container parser limit. Over-depth inbound records fail the connection with a `Protocol` error. Native session parsing and direct `Value` decoding use the same limit. Encoding checks depth incrementally as bytes are written, stopping at the first excess container; native construction counts the complete entry/message envelope. Cloning, serialization and cleanup at the boundary are exercised on a 2 MiB thread stack in an isolated test process. This is a bounded-depth contract, not arbitrary-depth JSON support.

Default RPC limits are **32 MiB per JSON record**, **64 MiB of writer bytes**, **128 MiB of queued event wire bytes**, 128 pending commands, 128 queued writer records and 1,024 events. Explicit caller limits remain authoritative. Pi 1.2.0 normally resizes each prompt image to at most 2000×2000 and below 4.5 MiB of Base64, with model overrides and an option to disable resizing. That processing happens after RPC input is received; these transport limits cover the original input too. Four images near Pi's default processed limit occupy about 18 MiB before text and JSON overhead. The SDK does not resize images or enforce an image count. Hosts must bound original image sizes and request concurrency.

Image-bearing message_start/message_end events retain their complete payload and are charged separately. Complete get_messages/get_entries/get_tree responses must fit the same record limit; accumulated history can exceed it and then terminates the connection with Protocol. Hosts should maintain history from events rather than repeatedly fetch unbounded image history. Native sessions default to 32 MiB per record and 64 MiB per file. These are wire budgets, not RSS limits: caller-owned images, decoded objects and command responses awaiting collection use additional memory.

`Config` bounds record bytes, pending command count, queued writer record count, queued/in-flight writer bytes, event count and total event wire bytes. All limits must be positive and fit Tokio permit counts. Admission uses bounded, nonblocking queues: overload returns `Capacity`, rather than parking unlimited submitted input in a business queue. A private bounded counting pass measures stable borrowed SDK records, then reserves their complete framed output before allocating it. Encoding, queued and in-flight output share the writer byte budget, held through write/flush. Writer slots and pending-command admission are also reserved before counting; rejection never allocates a full output record. Owned command inputs are released once encoded output exists, and owned UI answers after enqueue, rather than being retained while awaiting a response or flush. Public generic codec::encode remains single-pass. Failed preparation, cancellation and close races release reservations; commands already queued still run independently of an awaiting future. Reader and writer run independently, with no mutex held while awaiting a response, so a long-running bash/compaction command does not block submitting abort.

Event byte accounting uses the original serialized UTF-8 record size, excluding LF/CR. Count limits also bound per-event representation overhead. The reader adds at most one parsed record and a bounded framing chunk beyond the configured queues. If event count or bytes overflow, or the event consumer disappears, the connection fails explicitly with `EventOverflow`, notifies the owner and pending commands, closes I/O, and local process supervision terminates/reaps the child. There are no silent drops, blocking the continuous reader on the consumer, or infinite buffers.

## Extension UI

Default `UiPolicy::Cancel` enqueues `cancelled: true` for **every dialog**, including confirm. It never approves unattended actions. `Manual` leaves select/confirm/input/editor decisions to the consumer:

```rust,no_run
use pi_rpc::{Config, Error, PiConnection, Transport, UiPolicy};
use pi_rpc::protocol::{Event, UiAnswer};
# async fn example<R, W>(reader: R, writer: W, declared_version: &str) -> pi_rpc::Result<()>
# where R: tokio::io::AsyncRead + Unpin + Send + 'static,
# W: tokio::io::AsyncWrite + Unpin + Send + 'static {
let config = Config { ui_policy: UiPolicy::Manual, ..Config::default() };
// The host supplies the actual concrete Pi version, such as "1.1.0" or "1.4.0".
let transport = Transport::new(reader, writer, declared_version)?;
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

The host application owns conversation/SSE correlation, input versus resume versus new-session policy, remote delivery and business stopping policy. To stop: exclude new submissions at the host, **await `clear_queue`, then await `abort`**, optionally terminate. This sequence is **not atomic**. In-flight inputs, extensions and Pi can enqueue between calls. Track returned texts and subsequent queue updates, and enforce admission before stopping. The pinned docs and implementation disagree about queued continuation after abort. Race tests intentionally show a nonempty queue after settlement. Do not promise automatic drain.
