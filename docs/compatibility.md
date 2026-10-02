# Source coverage and compatibility

All Pi evidence was fetched **only** from `earendil-works/pi` at commit `005af57d88ee23b33778f343a9595b32e67ff788`, via the pinned Git tree and pinned raw URLs. Links below are relative to [that commit](https://github.com/earendil-works/pi/tree/005af57d88ee23b33778f343a9595b32e67ff788), never `main`. Upstream docs contain `main` links, but those links were not fetched. No upstream source is vendored/copied into this crate. License for this implementation remains undecided.

## Required source evidence

| Path under `packages/coding-agent/` | SHA-256 of inspected file | Implementation coverage |
|---|---|---|
| `src/modes/rpc/rpc-types.ts` | `be06a1d53916e03e9274cce9e4833692b235494e4047f1f7139d7f0147daae1a` | Commands, response envelope/data, state, slash commands, nine UI methods |
| `src/modes/rpc/rpc-mode.ts` | `d534e0fa1484844a097963d5f4fb4df9397c5a1b5e29e42aed7575ab6f67989b` | Concurrent ID correlation, acceptance callback, no UI ack, shutdown on stdin EOF, separate diagnostics |
| `docs/rpc.md` | `37a03d180115460a0a275022f4aeabef1e5345458df6a9b6e8a6de18474de720` | LF framing, CRLF, continuous reading, command/event lifecycle |
| `docs/rpc-commands.md` | `4deeee367852f4633ef594bc079f9e9fb14640a4e3ba8a69b7c89b6f54740838` | Every command, queued/handled/started disposition and cancelled session changes |
| `docs/rpc-extension-ui.md` | `3c6f2c6edfc8b1ace6642343727aef97a7e9234c85509850921b52d6ff09a48e` | Dialog response shapes, timeout, one-way methods, safe unattended policy |
| `docs/json.md` | `839a2a2da8112298a7f8870afa71d1998db2d43b8f68c9c179a0c696b6477a5e` | All 24 event kinds, 12 delta-only assistant update variants, usage and optional bash ID |
| `docs/session-format.md` | `4f8fc42179b332bcf71fe4c85cc95d914853ff7486d68742954a45316b512a4a` | v3 header, 11 native entry kinds, tree branches, compaction/context edits |
| `src/core/session-manager.ts` | `450d82c529933214e088b8422f00061815e617f352f6c834689787a314064bff` | Native entry field definitions, context replacement object, raw tree nodes |

## Additional pinned evidence inspected

- `packages/coding-agent/package.json`: exact version is `0.99.2`.
- `packages/coding-agent/src/core/agent-session.ts`: `AgentSessionEvent`, dispositions, `SessionStats`, `abort`, `_runAgentPrompt`, `_handlePostAgentRun`, and pre-settle continuation guards.
- `packages/agent/src/types.ts`: `AgentEvent`, `ThinkingLevel` including `max`, extensible `AgentMessage`.
- `packages/ai/src/types.ts`: native content blocks, usage/cost, message fields, provider assistant streaming events.
- `packages/coding-agent/src/core/messages.ts`: bashExecution, custom, branchSummary and compactionSummary roles.
- `packages/coding-agent/src/core/source-info.ts`: open command resource source metadata.
- `packages/coding-agent/src/core/bash-executor.ts`: `BashResult`.
- `packages/coding-agent/src/core/compaction/compaction.ts`: `CompactionResult`.
- `packages/coding-agent/src/modes/json-event.ts`: wire delta transformation removes cumulative snapshots.
- `packages/coding-agent/docs/message-types.md`: shared native message documentation.

## Exact compatibility boundaries

1. **33 normal commands**: prompt, steer, follow_up, abort, clear_queue, new_session, get_state, set_model, cycle_model, get_available_models, set_thinking_level, cycle_thinking_level, get_available_thinking_levels, set_steering_mode, set_follow_up_mode, compact, set_auto_compaction, set_auto_retry, abort_retry, bash, abort_bash, get_session_stats, export_html, switch_session, fork, clone, get_fork_messages, get_entries, get_tree, get_last_assistant_text, set_session_name, get_messages, get_commands. No import_history.
2. All success payloads are validated against command-specific shapes, but model/provider metadata is intentionally open JSON. Pi rejection remains separate from later run failures. Nullable cycle results and missing optional exit codes are supported.
3. **24 events**: agent_start, agent_end, agent_settled, turn_start, turn_end, message_start, message_update, message_end, tool_execution_start/update/end, queue_update, entry_appended, session_info_changed, thinking_level_changed, compaction_start/end, auto_retry_start/end, summarization_retry_scheduled/attempt_start/finished, bash_execution_update, extension_error. Known malformed events fail explicitly. Unknown event types retain complete raw JSON. Open tool and UI extension fields survive decoding.
4. Tool lifecycle includes optional `parentToolCallId`, discovered in source beyond the short documentation table. Wire message_update has `usage` and delta-only `assistantMessageEvent`, not a required cumulative message or partial snapshot. There are 12 nested update variants including admitted start/done/error.
5. **Abort discrepancy**: `docs/rpc-commands.md`, clear_queue section, says abort continues remaining queued messages. The pinned `agent-session.ts` sets `_agentRunAbortRequested` and guards automatic continuation in `_runAgentPrompt` and `_handlePostAgentRun`. Therefore SDK docs/tests make no auto-drain promise. The deterministic mock race test retains queued text after agent_settled and checks clear_queue-before-abort ordering. This tests client semantics, not a paid live model run.
6. **Native context edit**: source defines `replacement: { content: ContextEditableContent } | null`, even though prose broadly calls this a content replacement. Rust follows the source object shape. Compaction system checkpoint and usage/details remain preserved as extension JSON. Native v3 only, explicit legacy rejection, no Pi context projection/migration implementation.
7. **Version verification**: local executable version probe, exact expected 0.99.2. Remote caller declaration only. No invented handshake. Upgrading requires re-inspecting another explicitly chosen commit and updating tests/types deliberately.
8. **Shutdown**: stdin EOF is orderly shutdown, distinct from abort/clear_queue/forced process termination. There is a bounded close deadline. Owner drop is cleanup, clone drop is not.
9. **Exit/EOF races**: process exit independently wakes the reader. A 50 ms bounded drain preserves ready protocol output where practical despite inherited stdout handles. Unexpected stdout EOF grants 100 ms for direct-child status, then becomes `Eof` and triggers owned-child cleanup if the child remains alive. Detached descendants themselves are outside lifecycle ownership.
10. **Required nullable fields**: explicit null is accepted, absence is rejected for cycle success data, last-assistant text, entries/tree leafId, native entry parentId and context-edit replacement. Open tool payloads require presence but admit JSON null. Known nested assistant events validate their required fields and toolcall_end's actual tool-call shape.
11. **Terminal-event ordering**: event-channel closure is not authoritative terminal publication. PiEvents drains queued events, then awaits the terminal watch, yields exactly one error, then None. Event-first regressions cover 3000 immediate reader failures, 1000 buffered-event/failure connections, panic, EOF, owner drop and cancelled waiting. An unchanged external 3000-iteration harness independently reproduces failure before the fix and passes after it.
