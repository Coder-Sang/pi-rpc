# Native session files

Enable `session_file` to construct and parse **Pi-native v3 JSONL**. This module operates on caller-supplied data in memory. The host owns file paths, delivery, permissions, `--session` selection and atomic replacement. There is no automatic session discovery, vendor-message conversion, `import_history` RPC or model-context replay.

## Constructing history

Use `SessionFileBuilder::new(id, timestamp, cwd)?` for a new session, or `from_session_file(file)?` to take ownership of validated history. The caller supplies entry IDs, parent IDs and timestamps. Entry/header timestamps are strings; message timestamps are Unix milliseconds. Validation checks presence rather than calendar correctness.

```rust
use pi_rpc::{
    protocol::MessageContent,
    session_file::{SessionFileBuilder, SessionLimits, user_message},
};

fn build_history() -> pi_rpc::Result<Vec<u8>> {
    let mut builder = SessionFileBuilder::new(
        "example-native", "2026-10-02T12:00:00.000Z", "/project",
    )?;
    builder.append_message(
        "entry-1", None, "2026-10-02T12:00:00.000Z",
        user_message(MessageContent::Text("Hello".into()), 1790942400000)?,
    )?;
    builder.finish()?.encode(SessionLimits::default())
}
```

Each append validates the new message, ID and already-existing parent. Rejection leaves records and the ID index unchanged. `finish` performs full validation and transfers ownership without cloning history. Pure-message bulk construction is O(n) overall. The public mutable `SessionFile::append_message` still validates the complete history on every call because callers can mutate earlier entries.

Run `cargo run --example native_session --features session_file` for a complete in-memory round trip including assistant usage. It invokes no Pi or model.

## JSONL shape and Usage

The first line is a session header. Every following line is a complete entry; encoding ends each line with LF. CRLF input is accepted, but missing final LF is rejected. The example values below are illustrative, not a model invocation or billing estimate:

```jsonl
{"type":"session","version":3,"id":"session-1","timestamp":"2026-10-02T12:00:00.000Z","cwd":"/project"}
{"id":"entry-1","parentId":null,"timestamp":"2026-10-02T12:00:00.000Z","type":"message","message":{"role":"user","content":"Hello","timestamp":1790942400000}}
{"id":"entry-2","parentId":"entry-1","timestamp":"2026-10-02T12:00:01.000Z","type":"message","message":{"role":"assistant","content":[{"type":"text","text":"Hello!"}],"api":"example-api","provider":"example-provider","model":"example-model","usage":{"input":100,"output":20,"cacheRead":0,"cacheWrite":0,"totalTokens":120,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}},"stopReason":"stop","timestamp":1790942401000}}
```

`protocol::Usage` contains `input`, `output`, `cacheRead`, `cacheWrite`, `totalTokens` and a `Cost` breakdown. These public numeric fields are f64. Supply Usage as the complete native assistant message's `usage` field; assistant decoding requires it alongside content, API/provider/model, stop reason and timestamp. For an already constructed message:

```rust
fn attach_usage(
    message: &mut pi_rpc::protocol::Message,
    usage: pi_rpc::protocol::Usage,
) -> Result<(), serde_json::Error> {
    message.fields.insert("usage".into(), serde_json::to_value(usage)?);
    Ok(())
}
```

Then pass that message to `append_message`. The SDK preserves supplied usage and checks its structure; it does not tokenize content, derive fees, verify usage totals against content, or generate a session-wide total. User messages do not require usage. Independent `EntryData::Usage` records are also supported for operation usage, with `kind`, provider, model and usage fields.

Image content uses native blocks such as `{"type":"image","data":"BASE64_DATA","mimeType":"image/png"}`. The data stays inline in the JSON; the SDK does not resize or load images from a path. Base64 and the surrounding message/envelope count toward byte limits. See [RPC image budgets](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/connections.md#bounds-and-backpressure) for transport limits, which are configured separately.

## Limits and memory

| SessionLimits field | Default | Scope |
|---|---|---|
| `max_record_bytes` | 32 MiB | One header/entry JSON record, excluding LF or CRLF |
| `max_file_bytes` | 64 MiB | Entire encoded/input file, including line endings |
| `max_entries` | 100,000 | Entries after the header |

Pass custom limits to both `SessionFile::parse(bytes, limits)` and `file.encode(limits)`. JSON nesting is fixed at **512 containers**, including the root and entry/message envelope; it is not a SessionLimits setting.

`SessionFileBuilder` has no cumulative byte/count budget. Append and finish validate semantics and depth, but can retain more data than a later encode permits. The host must bound incoming history while constructing it.

`encode()` allocates and returns a complete `Vec<u8>`. It encodes one record at a time into a temporary buffer and appends it to the complete output, then performs full semantic validation. There is currently no direct `Write` or `AsyncWrite` API. Writing the returned slice in chunks does not remove this earlier allocation.

Peak memory includes the original SessionFile, output capacity, the current record and validation temporaries. Message validation currently serializes to a temporary JSON value and decodes owned fields, which can copy text and Base64. The exact overhead depends on the payload and allocation strategy; no fixed two-times or RSS bound is promised. File/record quotas bound serialized bytes, not process memory.

Encoding checks entry count and byte budgets before full semantic validation; if both fail, a capacity error may be returned first. File bytes include each LF. Finite f64 values round-trip exactly, including signed zero and subnormals; nonfinite usage/cost/compaction numbers are rejected.

## Validation and branches

Validation checks the v3 header, nonempty IDs/timestamps/cwd, unique entry IDs, append-order parent links, reserved field collisions, known native message shapes, depth and branch-relative references. Compaction references must stay on the parent branch; context edits target editable messages on that branch; labels reference existing IDs. Multiple roots are valid.

The supported entries are message, thinking/model changes, usage, compaction, branch summaries, custom/custom-message entries, context edits, labels and session information. Unknown persisted entry types and legacy v1/v2 files are rejected. Extension JSON and custom message roles remain open. Required-but-nullable fields such as `parentId` and context-edit `replacement` distinguish absence from explicit null.

`branch` returns raw root-to-leaf entries. It does not rebuild Pi's compacted model context. Keep active context and native history separate; see [the event guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/events.md#active-context-and-native-history).

## Informational measurements

The Release examples `session_builder_bench` and `session_validation_bench` report median construction/validation times. They do not impose machine-specific unit-test thresholds or measure process RSS. Run them with `--no-default-features --features session_file`; see [CONTRIBUTING](https://github.com/Coder-Sang/pi-rpc/blob/main/CONTRIBUTING.md) for development requirements.
