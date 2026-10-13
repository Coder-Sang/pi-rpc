# Consuming Pi 1.1.0 events

`PiConnection::connect` returns a `PiClient`, a single `PiEvents` consumer and the connection owner. `PiEvents::recv().await` returns `Option<Result<Event>>`. Events remain ordered across the whole session and are not scoped to the latest prompt. The host adapter owns conversation/SSE correlation and run-outcome policy. See the [README](../README.md) for connection ownership and the [compatibility contract](compatibility.md) for the pinned source and limits.

## Event families

| Outer variant | Meaning |
|---|---|
| `Event::Known` | One of the 24 supported ordinary Pi 1.1.0 event types |
| `Event::Ui` | An extension UI request, including one-way notifications |
| `Event::Unknown` | A record with an unrecognized string `type`; its complete JSON is preserved |
| `Event::UncorrelatedResponse` | A validated command response with no ID |

The ordinary types are:

| Family | RPC types |
|---|---|
| Run/turn | `agent_start`, `agent_end`, `agent_settled`, `turn_start`, `turn_end` |
| Message | `message_start`, `message_update`, `message_end` |
| Tool execution | `tool_execution_start`, `tool_execution_update`, `tool_execution_end` |
| Session/state | `queue_update`, `entry_appended`, `session_info_changed`, `thinking_level_changed` |
| Compaction | `compaction_start`, `compaction_end` |
| Retry | `auto_retry_start`, `auto_retry_end`, `summarization_retry_scheduled`, `summarization_retry_attempt_start`, `summarization_retry_finished` |
| Other | `bash_execution_update`, `extension_error` |

Matching-ID command responses resolve their command futures. Valid responses with an ID but no pending waiter, including late responses after cancellation/timeout, are ignored. A wrong command for a pending ID is a terminal protocol error. Ordinary events generally carry no originating command ID; bash output may carry one.

## Text, thinking and tool calls

Pi's [wire transformation](https://github.com/earendil-works/pi/blob/abe508e1b89912adde45528136c3221eb69acdd7/packages/coding-agent/src/modes/json-event.ts) removes cumulative assistant `message` and `partial` snapshots from `message_update`. The wire retains cumulative `usage` and an `assistantMessageEvent` with `contentIndex` and the current block update.

Maintain separate content blocks by `contentIndex`, including multiple text blocks in the same message:

- `text_start` / `thinking_start` starts that block. `text_delta` / `thinking_delta` appends its text. The corresponding `*_end.content` is authoritative and replaces reconstructed content.
- `toolcall_start` provides the block's `id` and `toolName`. `toolcall_delta` appends serialized argument fragments; each fragment need not be valid standalone JSON. `toolcall_end.toolCall` replaces the reconstructed tool-call block with its complete arguments and metadata.
- `message_end.message` replaces the complete accumulated message. Preserve native content fields such as text/thinking signatures, redaction flags, tool namespace and provider metadata. The SDK delivers these records and does not assemble an application-level response.

The exported Assistant update enum admits 12 shapes: `start`, the three text shapes, the three thinking shapes, the three tool-call shapes, `done` and `error`. The normal [agent loop](https://github.com/earendil-works/pi/blob/abe508e1b89912adde45528136c3221eb69acdd7/packages/agent/src/agent-loop.ts#L416) translates provider `start`/`done`/`error` into native message boundaries. Most normal streamed updates are the nine block shapes. A provider can fail before its first delta and still produce `message_start` followed by `message_end`.

Tool-call content and actual tool execution have separate lifecycles. Tool execution events preserve `toolCallId`, optional `parentToolCallId`, open args/results and extension fields. Execution completion order may differ from tool-call content order. Correlate by IDs rather than adjacency. `tool_execution_end.isError` reports a tool outcome; it does not end the connection. Tool result payloads can include images and opaque details.

## Custom messages

Pi's [custom-message ingestion](https://github.com/earendil-works/pi/blob/abe508e1b89912adde45528136c3221eb69acdd7/packages/coding-agent/src/core/agent-session.ts#L2293) emits native `message_start` and `message_end` records for appended custom messages. Read `Message.role == "custom"` and its open `fields`: `customType`, `content`, `display`, `details`, `timestamp` and extension metadata. Content may be a string or an array of text/image blocks, including an empty array.

The SDK delivers both `display=true` and `display=false` messages. The host chooses its presentation policy; `display=false` is not a transport filter. Additional extension-defined message roles are preserved as native messages too.

`entry_appended` carries native session entries. A `custom` entry stores extension data; a `custom_message` entry stores a custom message. Handle these entry variants separately from the message boundary events, and use entry IDs when maintaining history. The SDK forwards only records actually emitted by Pi; it does not synthesize an entry event for every message.

## UI requests

The four dialog methods are `select`, `confirm`, `input` and `editor`. Default `UiPolicy::Cancel` sends cancellation for dialogs and still delivers the requests as events. `UiPolicy::Manual` leaves responses to the host. `respond_ui` validates the answer and waits for local flush, with no RPC acknowledgement or pending-command slot.

The five one-way methods are `notify`, `setStatus`, `setWidget`, `setTitle` and `set_editor_text`; receive them without sending a reply. All nine methods preserve their request IDs and extension fields.

Closing skips automatic replies while continuing to deliver late UI events and following command responses. Public manual replies after close begins return `Closed`, or the already published terminal reason. Choose the policy at connection creation and avoid duplicate replies. Pi's own dialog timeout/signal can resolve the dialog before a host answer arrives.

## Millisecond fields

The public integer types remain `UiMethod.timeout: Option<u64>`, `KnownEvent::AutoRetryStart.delay_ms: u64` and `KnownEvent::SummarizationRetryScheduled.delay_ms: u64`. Pi can emit fractional milliseconds: dialog options use `number`, and a fractional retry delay cap can be returned by [retryDelayMs](https://github.com/earendil-works/pi/blob/abe508e1b89912adde45528136c3221eb69acdd7/packages/ai/src/utils/retry.ts#L128).

| Wire value | Public integer value |
|---|---|
| `1500.5` | `1500` |
| `0.75` | `0` |
| `0` / `-0.0` | `0` |
| Integer above 2^53, up to `u64::MAX` | Exact integer; no intermediate f64 conversion |
| Missing/null UI timeout | `None`, preserving existing behavior |
| Missing/null retry delay | Protocol error |
| Negative, nonnumeric or out-of-range number | Protocol error |

Finite nonnegative fractions are truncated toward zero, including exponent notation. Floating-point values must be strictly below 2^64; that bound prevents a rounded `u64::MAX` float from becoming a saturating cast. A numeric timeout below 1 remains `Some(0)`, distinct from `None`. These are informational integer values: Pi schedules the actual timeout/backoff using its own original number and runtime. Hosts should not replace Pi's timer with a timer based on the truncated field. `UiRequest.extra` retains original method fields, including the original wire timeout, through the existing flattening behavior.

Serializing a `UiMethod` or a retry event emits its stored integer duration. This normalization intentionally discards sub-millisecond precision in those integer fields. Other floating-point fields, such as tool `durationMs`, usage and cost, keep their existing exact finite-f64 roundtrip behavior. Record/event byte budgets continue to use the original wire record size.

## Model outcomes and connection termination

For assistant `message_end`, inspect `message.fields["stopReason"]`:

- `error` and `aborted` are model/run outcomes, with `errorMessage` or diagnostics when present. They arrive as `Ok(Event::Known(...))` and leave the connection usable.
- `deferred` is a native deferred outcome and also remains an ordinary message event.
- `toolUse`, `length` and `stop` likewise describe that assistant message rather than connection termination.

Do not wait only for `AssistantEvent::Error` or a top-level `Err` to detect a failed model attempt. `agent_end.willRetry` indicates possible recovery, and `agent_settled.aborted` describes settlement. The host decides when an API request is complete across retries, tools and queued continuations. Settlement alone does not prove queues are empty.

`extension_error`, compaction errors and retry failure fields also remain ordinary events. Command rejection is returned by the corresponding command future as `Error::Rejected` and is nonterminal.

Connection failures arrive through `recv()` as one `Err`: transport failure, EOF/process exit, malformed known records, configured capacity overflow or close termination. The consumer drains already buffered events first, receives exactly one authoritative terminal reason, then receives `None` on subsequent calls. Cancelling a pending receive does not consume that reason. The connection owner and outstanding commands observe the same published terminal error. In the README example, terminal errors are returned to the caller; the host can treat orderly `Closed` according to its lifecycle policy.
