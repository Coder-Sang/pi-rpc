# Source coverage and compatibility

Current compatibility is pinned to **Pi 1.1.0**, official release tag [`v1.1.0`](https://github.com/earendil-works/pi/releases/tag/v1.1.0), commit [`abe508e1b89912adde45528136c3221eb69acdd7`](https://github.com/earendil-works/pi/tree/abe508e1b89912adde45528136c3221eb69acdd7). The latest stable release was resolved on 2026-10-09 through GitHub's release/tag APIs (published 2026-10-07). The current `main` package version was checked only for discovery; compatibility evidence below comes from the fixed release archive, compared against the previous 0.99.2 commit `005af57d88ee23b33778f343a9595b32e67ff788`. Links below are relative to the fixed release commit, not moving `main`. No upstream source is vendored/copied into this crate. License for this implementation remains undecided.

## Required source evidence

| Path under `packages/coding-agent/` | SHA-256 of inspected file | Implementation coverage |
|---|---|---|
| `src/modes/rpc/rpc-types.ts` | `be06a1d53916e03e9274cce9e4833692b235494e4047f1f7139d7f0147daae1a` | Commands, response envelope/data, state, slash commands, nine UI methods |
| `src/modes/rpc/rpc-mode.ts` | `d534e0fa1484844a097963d5f4fb4df9397c5a1b5e29e42aed7575ab6f67989b` | Concurrent ID correlation, acceptance callback, no UI ack, shutdown on stdin EOF, separate diagnostics |
| `docs/rpc.md` | `37a03d180115460a0a275022f4aeabef1e5345458df6a9b6e8a6de18474de720` | LF framing, CRLF, continuous reading, command/event lifecycle |
| `docs/rpc-commands.md` | `4deeee367852f4633ef594bc079f9e9fb14640a4e3ba8a69b7c89b6f54740838` | Every command, queued/handled/started disposition and cancelled session changes |
| `docs/rpc-extension-ui.md` | `3c6f2c6edfc8b1ace6642343727aef97a7e9234c85509850921b52d6ff09a48e` | Dialog response shapes, timeout, one-way methods, safe unattended policy |
| `docs/json.md` | `f103191816fdcf3627dfc98b89acad8a28eee737697130ddaeb2581131213543` | All 24 event kinds, 12 delta-only assistant update variants, usage and optional bash ID |
| `docs/session-format.md` | `4f8fc42179b332bcf71fe4c85cc95d914853ff7486d68742954a45316b512a4a` | v3 header, 11 native entry kinds, tree branches, compaction/context edits |
| `src/core/session-manager.ts` | `450d82c529933214e088b8422f00061815e617f352f6c834689787a314064bff` | Native entry field definitions, context replacement object, raw tree nodes |

## Additional pinned evidence inspected

- `packages/coding-agent/package.json`: exact version is `1.1.0`.
- `packages/coding-agent/src/core/agent-session.ts`: `AgentSessionEvent`, dispositions, `SessionStats`, `abort`, `_runAgentPrompt`, `_handlePostAgentRun`, and pre-settle continuation guards.
- `packages/agent/src/types.ts`: `AgentEvent`, `ThinkingLevel` including `max`, extensible `AgentMessage`.
- `packages/ai/src/types.ts`: native content blocks, usage/cost, message fields, provider assistant streaming events.
- `packages/coding-agent/src/core/messages.ts`: bashExecution, custom, branchSummary and compactionSummary roles.
- `packages/coding-agent/src/core/source-info.ts`: open command resource source metadata.
- `packages/coding-agent/src/core/bash-executor.ts`: `BashResult`.
- `packages/coding-agent/src/core/compaction/compaction.ts`: `CompactionResult`.
- `packages/coding-agent/src/modes/json-event.ts`: wire delta transformation removes cumulative snapshots (unchanged).
- `packages/coding-agent/docs/message-types.md`: shared native message documentation.

## Upgrade from Pi 0.99.2 to 1.1.0

- The release archives were compared directly: RPC command/response definitions, RPC mode, RPC/UI command docs, session format/manager, custom message source, delta transformation and compaction result definitions are byte-identical. Coverage remains **33 commands, 24 events, nine UI methods and 11 native entry kinds**.
- `agent_settled` now requires `aborted: boolean`, copied from the session's abort flag when publishing settlement. Missing, null or non-boolean flags fail known-event decoding. This is not a queue-empty guarantee.
- `tool_execution_end.durationMs?: number` is exposed as `Option<f64>` and omitted on serialization when absent. Native assistant and tool-result messages also preserve optional numeric `durationMs` in their open fields. Explicit null/non-numbers are rejected; arbitrary tool result/extension payloads remain open.
- Provider `azure` and `samplingParamsByThinkingLevel` are retained through existing open model/message JSON. No provider SDK enum needs changing. The bash executor changed internal output storage/ANSI handling, not its RPC result schema.
- **Observed wire/type discrepancy**: on an empty real 1.1.0 session, `get_last_assistant_text` returns `data: {}`. The TypeScript response type declares `text: string | null`, but `getLastAssistantText()` returns undefined and JSON serialization omits it. The client accepts omitted or null text as `None`, still rejects non-string values and missing/non-object data. This exception does not relax required nullable tree/entry fields.
- Upgrading changes the Rust enum pattern to `KnownEvent::AgentSettled { aborted }`, and adds `duration_ms` to `ToolExecutionEnd`. This crate remains unpublished at 0.1.0, pinned to exactly 1.1.0, not a multi-version client. Old local/declared remote versions are rejected.

Additional source hashes for the new fields:

| Fixed-release source | SHA-256 |
|---|---|
| `packages/coding-agent/src/core/agent-session.ts` | `19186bf8a79876131a296fe17333c6276a671f8f48401df238c9814eeba0f5df` |
| `packages/agent/src/types.ts` | `2e116ef60fdaca4e26be5a1ace8d8b47f545d648d365de60b8c9f9d1ba629ee4` |
| `packages/ai/src/types.ts` | `568d1ba5d382d9ef4e8d98040c100545c21d1cacfbc98db6dd749014c3c3a15e` |

## Exact compatibility boundaries

1. **33 normal commands**: prompt, steer, follow_up, abort, clear_queue, new_session, get_state, set_model, cycle_model, get_available_models, set_thinking_level, cycle_thinking_level, get_available_thinking_levels, set_steering_mode, set_follow_up_mode, compact, set_auto_compaction, set_auto_retry, abort_retry, bash, abort_bash, get_session_stats, export_html, switch_session, fork, clone, get_fork_messages, get_entries, get_tree, get_last_assistant_text, set_session_name, get_messages, get_commands. No import_history.
2. All success payloads are validated against command-specific shapes, but model/provider metadata is intentionally open JSON. Pi rejection remains separate from later run failures. Nullable cycle results and missing optional exit codes are supported.
3. **24 events**: agent_start, agent_end, agent_settled, turn_start, turn_end, message_start, message_update, message_end, tool_execution_start/update/end, queue_update, entry_appended, session_info_changed, thinking_level_changed, compaction_start/end, auto_retry_start/end, summarization_retry_scheduled/attempt_start/finished, bash_execution_update, extension_error. Known malformed events fail explicitly. Unknown event types retain complete raw JSON. Open tool and UI extension fields survive decoding.
4. Tool lifecycle includes optional `parentToolCallId`, discovered in source beyond the short documentation table. Wire message_update has `usage` and delta-only `assistantMessageEvent`, not a required cumulative message or partial snapshot. There are 12 nested update variants including admitted start/done/error.
5. **Abort discrepancy**: `docs/rpc-commands.md`, clear_queue section, says abort continues remaining queued messages. The pinned `agent-session.ts` sets `_agentRunAbortRequested` and guards automatic continuation in `_runAgentPrompt` and `_handlePostAgentRun`. Therefore SDK docs/tests make no auto-drain promise. The deterministic mock race test retains queued text after agent_settled and checks clear_queue-before-abort ordering. This tests client semantics, not a paid live model run.
6. **Native context edit**: source defines `replacement: { content: ContextEditableContent } | null`, even though prose broadly calls this a content replacement. Rust follows the source object shape. Compaction system checkpoint and usage/details remain preserved as extension JSON. Native v3 only, explicit legacy rejection, no Pi context projection/migration implementation.
7. **Version verification**: local executable version probe, exact expected 1.1.0. Remote caller declaration only. No invented handshake. Upgrading requires re-inspecting another explicitly chosen commit and updating tests/types deliberately.
8. **Shutdown**: stdin EOF is orderly shutdown, distinct from abort/clear_queue/forced process termination. There is a bounded close deadline. Owner drop is cleanup, clone drop is not.
9. **Exit/EOF races**: process exit independently wakes the reader. A 50 ms bounded drain preserves ready protocol output where practical despite inherited stdout handles. Unexpected stdout EOF grants 100 ms for direct-child status, then becomes `Eof` and triggers owned-child cleanup if the child remains alive. Detached descendants themselves are outside lifecycle ownership.
10. **Required nullable fields**: explicit null is accepted, absence is rejected for cycle success data, entries/tree leafId, native entry parentId and context-edit replacement. Open tool payloads require presence but admit JSON null. Known nested assistant events validate their required fields and toolcall_end's actual tool-call shape.
11. **Terminal-event ordering**: event-channel closure is not authoritative terminal publication. PiEvents drains queued events, then awaits the terminal watch, yields exactly one error, then None. Event-first regressions cover 3000 immediate reader failures, 1000 buffered-event/failure connections, panic, EOF, owner drop and cancelled waiting. An unchanged external 3000-iteration harness independently reproduces failure before the fix and passes after it.
12. **Configuration side effects**: Pi persists steering/follow-up modes and auto-compaction/retry toggles in the selected agent directory. The Rust transport does not directly edit user files, but these RPCs are not readonly. Use an isolated agent directory for acceptance tests. Authentication refreshes and process environment policy belong to the host/Pi, not this SDK.
13. **Validated toolchain/platform**: macOS aarch64 on Rust 1.97.1 and Rust 1.85.1. The manifest declares Rust 1.85; exact 1.85.0, Linux/Windows and remote vendor deployment have not been verified. Packaging builds successfully but licensing and publishing decisions remain open; see [public release checklist](publishing.md).
