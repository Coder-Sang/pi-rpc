# Source coverage and compatibility

**pi-rpc 0.2.3 (development)** retains **Pi >=1.1.0, <=1.4.0**, with Rust **1.88**. Published 0.2.2 supports the same range but does not include the new caller-declared local startup API. The latest reviewed release is [Pi 1.4.0](https://github.com/earendil-works/pi/releases/tag/v1.4.0), fixed commit [`4a080252f06caf63a525d7873c5d94d3c23887e0`](https://github.com/earendil-works/pi/tree/4a080252f06caf63a525d7873c5d94d3c23887e0). Published 0.2.1 retains Pi 1.1.0–1.3.0; published 0.2.0 retains Pi 1.1.0–1.2.0; published 0.1.0 retains Pi 1.1.0/Rust 1.85. No upstream executable or source is vendored. Project code remains [Apache-2.0](../LICENSE), with [retained Pi MIT notices](../THIRD_PARTY_NOTICES.md).

## Caller-declared local startup (0.2.3 development)

`PiProcess::spawn_with_version(options, declared_version)` skips the external version probe. The caller guarantees the executable's actual release; the SDK checks a concrete canonical declaration against both its supported range and `options.expected_version` before starting any child. Declared versions use the same strict format as injected Transport versions, without surrounding whitespace or range expressions. Both legacy expectation ranges retain their own upper bounds, and exact locks remain exact, including patch-prefix collisions such as 1.3.1/1.3.10.

`PiProcess::spawn` retains its original bounded probe, including when a concrete expectation is selected. `startup_timeout` applies only to that probe and is unused by the new entrypoint. Both methods share the existing RPC process, diagnostics, connection and cleanup implementation; neither sends an automatic initialization RPC or waits for application readiness. The new method changes local launch policy, not the shared wire schema or Pi-owned execution behavior. Real-Pi acceptance examples continue to probe with `spawn`.

## Version range and review evidence

At the prior **2026-10-16** review, the official [release list](https://github.com/earendil-works/pi/releases), Git tags and [npm package metadata](https://registry.npmjs.org/@earendil-works/pi-coding-agent) listed only **1.1.0** and **1.2.0** as released versions inside the supported range. Their fixed source commits were reviewed. Fixtures reporting **1.1.1** and **1.1.10** are synthetic numeric-range cases, not claims of existing upstream releases.

Future canonical patch releases inside the range will pass version admission automatically. Admission is distinct from source review and live acceptance; maintainers must review and record any newly published in-range release. The client shares one wire implementation across the range. Busy state during startup and extension hook semantics remain version-dependent, as described below; the SDK forwards the actual records rather than emulating either version.

## Pi 1.4.0 source review (2026-10-27)

Official npm metadata lists **1.1.0, 1.2.0, 1.3.0 and 1.4.0** as the released canonical versions inside the new range. Synthetic 1.3.1/1.3.10 cases verify numeric admission and legacy upper bounds; they do not claim additional released versions. Versions above 1.4.0, including 1.4.1, remain rejected.

Compared fixed 1.3.0 commit `71023aec1546d6d677a65669c5665c0927ce0dd0` with 1.4.0 commit `4a080252f06caf63a525d7873c5d94d3c23887e0`. RPC types/mode, delta transformation, RPC/command/UI/JSON documentation, native v3 messages/session manager/format, agent event types, image processing/resizing and tool-result image normalization are byte-identical. The `AgentSessionEvent` declaration is unchanged as well. Coverage remains **33 commands, 24 events, 12 assistant update shapes, nine UI methods and 11 native entry kinds**, with one shared Rust implementation. The npm Node requirement remains **>=22.19.0**; native executable installation remains host-owned.

The MIT `LICENSE` remains byte-identical (SHA-256 `0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48`). Reviewed changed-file SHA-256 values at 1.4.0:

| Path | SHA-256 |
|---|---|
| `packages/ai/src/types.ts` | `9f7346dfd52bda4f25e527d768c67a210c88fb9dc9c4a16610b693087e626319` |
| `packages/ai/src/providers/compat-schema.ts` | `b65a1a7c2981eec921c396348bf12b96ca797a8cbfb3e540eed6c4200c83c1ff` |
| `packages/ai/src/providers/model-schema.ts` | `1229ca9e0b4489d8d5bd095788608d5c86f42a6e2a66e75f504d0045d05cc671` |
| `packages/coding-agent/src/core/agent-session.ts` | `e0a6e3e7b662afff9763a6297e0bb600da75e8eff4df443912f2e215bdc6a8e2` |
| `packages/coding-agent/src/core/compaction/compaction.ts` | `32714966eb644da31cd84e8eb172965256160626b3d26ae9752c3894300f2b0b` |
| `packages/coding-agent/src/core/compaction/branch-summarization.ts` | `cdffdcb3ca37f5121383d0f5ba98295eb07ce0e2085c8d57d8cb2fb5dc5e9243` |
| `packages/ai/src/utils/retry.ts` | `4c17e881e6e3d335c12f3325dee004d4fb26bf9f08e501661ff78c5b9bc31a1f` |

The [1.4.0 release](https://github.com/earendil-works/pi/releases/tag/v1.4.0) changes Pi-owned behavior:

- Classifier image support adds System One `compat.imageInput` and `compat.sendImageOptInHeader`; the SDK keeps model/provider metadata open JSON. This is not a new classifier RPC command or content-block kind. Existing record, image and encoding budgets remain in force.
- Compaction and branch summaries use the model output limit; `reserveTokens` controls the compaction trigger. The SDK forwards larger bounded summaries without assigning a provider token budget. Synthetic tests cover summaries larger than 64 KiB through events, active messages, entries/tree and native files.
- Retry classification recognizes more transient upstream failures and avoids treating digits inside request IDs/byte counts as status codes. Recovery omits a failed assistant attempt together with its matching tool results, including resumed sessions, using the existing native context edits. The SDK forwards these edits and keeps native history; it does not implement Pi's projection/recovery algorithm.
- Restored extension tools and codemode worker waiting change Pi execution. They do not introduce another SDK input queue. Request identity/provider headers and the renamed TypeScript telemetry/tool-projection APIs affect Pi/extensions, not the Rust RPC surface.

Local and declared remote admission now accept the new inclusive range. Both older fixed expectations retain their original upper bounds, and exact expectations remain exact. Fake probes test each endpoint, synthetic intermediate versions, early rejection, unchanged probe whitespace/bounds and terminal delivery. Acceptance entrypoints reject all three fixed range expressions before probing. Source review and synthetic checks do not establish live Pi 1.4.0 execution or model acceptance. The manual CI smoke stays explicitly pinned to Pi 1.2.0 and is skipped on ordinary pushes.

## Pi 1.3.0 source review (2026-10-18)

The official npm metadata lists **1.1.0, 1.2.0 and 1.3.0** as the released canonical versions inside the new inclusive range. Synthetic 1.2.1/1.2.10 fixtures exercise numeric admission; they are not claims of released upstream versions. Published 0.2.1 rejects versions above 1.3.0, including 1.3.1.

Compared fixed 1.2.0 commit `06a466935536f8c9bcacb2d5efc0fc021be1bdff` with 1.3.0 commit `71023aec1546d6d677a65669c5665c0927ce0dd0`. RPC types/mode, all three RPC documents, native `messages.ts`/`session-manager.ts`, agent event types and image-resize core are byte-identical. They retain the corresponding baseline hashes below. Command/event/UI and native v3 schemas continue to share one implementation.

The MIT `LICENSE` is byte-identical at both fixed commits (SHA-256 `0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48`); retained notices remain unchanged. Reviewed changed-file SHA-256 values at 1.3.0:

| Path | SHA-256 |
|---|---|
| `packages/ai/src/types.ts` | `258627828297eb0165de9828cab62666265a77e932dea5dba1542ace9c47c7d5` |
| `packages/ai/src/providers/compat-schema.ts` | `2df3f174686bc945844cdee99e063eafe8ff675d236b032b06fe370754a6c11a` |
| `packages/coding-agent/src/core/agent-session.ts` | `e0fa206e8c49a98c024708068df39c0e4944f4251633425fc083b2fbb5ec0ac6` |

`packages/ai/src/types.ts` adds optional tool-call `argumentsError`; `arguments` is an empty object when complete tool arguments are malformed. The Rust `ToolCall.extra` preserves the diagnostic without adding a public field. Existing required-object validation for arguments remains intact. `compat-schema.ts` adds `supportsMidConvoEffort`; open model JSON preserves it through model selection, cycling and model lists.

The [1.3.0 changes](https://pi.dev/changelog/releases/1.3.0) restore active tools on resume, drop earlier-model thinking during model switching, change Kimi's provider protocol/base URL, handle error-tool images for Anthropic/Bedrock, and broaden retry detection. These are Pi-owned execution/provider behaviors, not SDK emulation. Hosts must not assume identical model inputs or tool activation across versions. This source review and synthetic regression coverage do not establish real Pi 1.3.0/model acceptance. The existing manual CI smoke remains explicitly pinned to Pi 1.2.0.

## Required source evidence

The following hash inventory records the **Pi 1.2.0** review baseline; the 1.3.0 and 1.4.0 comparisons above identify unchanged files and additive changes.

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

- `packages/coding-agent/package.json`: exact version is `1.2.0`.
- `packages/coding-agent/src/core/agent-session.ts`: `AgentSessionEvent`, dispositions, `SessionStats`, `abort`, `_startRun`, `_handlePostAgentRun`, and pre-settle continuation guards.
- `packages/agent/src/types.ts`: `AgentEvent`, `ThinkingLevel` including `max`, extensible `AgentMessage`.
- `packages/ai/src/types.ts`: native content blocks, usage/cost, message fields, provider assistant streaming events.
- `packages/coding-agent/src/core/messages.ts`: bashExecution, custom, branchSummary and compactionSummary roles.
- `packages/coding-agent/src/core/source-info.ts`: open command resource source metadata.
- `packages/coding-agent/src/core/bash-executor.ts`: `BashResult`.
- `packages/coding-agent/src/core/compaction/compaction.ts`: `CompactionResult`.
- `packages/coding-agent/src/modes/json-event.ts`: wire delta transformation removes cumulative snapshots (unchanged).
- `packages/coding-agent/docs/message-types.md`: shared native message documentation.

## Upgrade from Pi 1.1.0 to 1.2.0

- RPC types/mode, command/UI/JSON documentation, session v3 format/manager, native message definitions, delta transformation, compaction and bash result schemas are byte-identical. Coverage remains **33 commands, 24 events, 12 assistant stream variants, nine UI methods and 11 native entry kinds**. Existing public Rust paths, fields and features remain unchanged.
- Pi now marks a run busy before preparing `before_agent_start`. Thus `get_state.isStreaming` may be true before `agent_start` and before the prompt acceptance response. Commands and UI traffic remain multiplexed. The synthetic startup regression verifies busy state, automatic dialog cancellation and abort before prompt acceptance without a paid run.
- `before_agent_start` is an extension hook, not a new RPC event. It now discriminates `messageType: "user" | "custom"`; custom-triggered runs receive the starting custom message and pending next-turn messages. SDK consumers continue to receive the existing native custom-message events. This crate does not add a hook API or a second execution queue.
- Abort during preparation is applied when `agent_start` is emitted. Pi still owns run/queue semantics; the host must not infer idleness from the absence of a start event or promise automatic queue draining.
- Model input/cost/thinking/compatibility types moved to TypeBox schema modules. Model objects remain open JSON, preserving key-scoped OpenRouter limits, pricing tiers and provider metadata. The image-resize defaults are unchanged. Retry classification accepts additional upstream-error wording; wire retry events and fractional-delay decoding are unchanged.
- Rust **1.88** permits `psm 0.1.32` and `ar_archive_writer 0.5.3`; the direct `psm = "=0.1.27"` workaround is removed. Existing stack/depth, float, capacity and shutdown protections are retained. Local and declared remote versions must be canonical releases in **>=1.1.0, <=1.4.0** for pi-rpc 0.2.2. Local defaults accept that range; a concrete `expected_version` retains exact matching. Versions below 1.1.0 or above 1.4.0 are rejected. The previous fixed `">=1.1.0, <=1.2.0"` and `">=1.1.0, <=1.3.0"` expectations retain their original upper bounds.
- Core upgrade gates use fake/in-memory transports. The subsequent [real endpoint/UI supplement](verification.md#real-pi-acceptance) records actual local Pi 1.1.0/1.2.0 results separately; hosted Opus 4.6 minimal calls passed on both endpoints, and the 1.2.0 tool/queue/abort/compaction/session workflow passed. These are local macOS results; other native platforms and all model/event variants remain unverified.

Additional fixed 1.2.0 source hashes:

| Fixed-release source | SHA-256 |
|---|---|
| `packages/coding-agent/src/core/agent-session.ts` | `e2bbf746779fbed5a8c36f023842bf271252d1fd6f17f5a32f70a0d1c63f0b65` |
| `packages/coding-agent/src/core/extensions/types.ts` | `cb54c7a4fedb3473bb995694bbc1f2988c144fb0b802bfd1b656816bc25d0f25` |
| `packages/ai/src/types.ts` | `19f632e0918cca442d3dfa3572ebabe79453dea0b1f54eadd3287a9c6b515674` |
| `packages/ai/src/providers/model-schema.ts` | `6cfdcf0368299300af277d1952781ed701be55ab6a900454fc3eeeffbe07e178` |
| `packages/ai/src/providers/compat-schema.ts` | `7cc0d438f21b565ede6a00f41a530fd660f040f5dce2a33a71789f29b6fe9e34` |
| `packages/coding-agent/src/utils/image-resize-core.ts` | `9e9bbe879ace4e3d817f8c79f77f17252d49904f4a12f989fce53792fa890b29` |
| `packages/ai/src/utils/retry.ts` | `6afcd371d0be5b3df2f200059b9478fa94ad6346625a6fc66f1638ea314a5fb3` |

## Observed upstream boundary

An empty real Pi 1.1.0 session returns `data: {}` for `get_last_assistant_text`: undefined text is omitted by JSON serialization despite the declared `string | null` type. The client accepts omitted or null text as `None`, but rejects non-string values and missing/non-object data. This exception does not relax required nullable tree/entry fields.

## Exact compatibility boundaries

1. **33 normal commands**: prompt, steer, follow_up, abort, clear_queue, new_session, get_state, set_model, cycle_model, get_available_models, set_thinking_level, cycle_thinking_level, get_available_thinking_levels, set_steering_mode, set_follow_up_mode, compact, set_auto_compaction, set_auto_retry, abort_retry, bash, abort_bash, get_session_stats, export_html, switch_session, fork, clone, get_fork_messages, get_entries, get_tree, get_last_assistant_text, set_session_name, get_messages, get_commands. No import_history.
2. All success payloads are validated against command-specific shapes, but model/provider metadata is intentionally open JSON. Pi rejection remains separate from later run failures. Nullable cycle results and missing optional exit codes are supported.
3. **24 events**: agent_start, agent_end, agent_settled, turn_start, turn_end, message_start, message_update, message_end, tool_execution_start/update/end, queue_update, entry_appended, session_info_changed, thinking_level_changed, compaction_start/end, auto_retry_start/end, summarization_retry_scheduled/attempt_start/finished, bash_execution_update, extension_error. Known malformed events fail explicitly. Unknown event types retain complete raw JSON. Open tool and UI extension fields survive decoding.
4. Tool lifecycle includes optional `parentToolCallId`, discovered in source beyond the short documentation table. Wire message_update has `usage` and delta-only `assistantMessageEvent`, not a required cumulative message or partial snapshot. There are 12 nested update variants including admitted start/done/error.
5. **Abort discrepancy**: `docs/rpc-commands.md`, clear_queue section, says abort continues remaining queued messages. The pinned `agent-session.ts` sets `_agentRunAbortRequested` and guards automatic continuation in `_startRun` and `_handlePostAgentRun`. Therefore SDK docs/tests make no auto-drain promise. The deterministic mock race test retains queued text after agent_settled and checks clear_queue-before-abort ordering. This tests client semantics, not a paid live model run.
6. **Native context edit**: source defines `replacement: { content: ContextEditableContent } | null`, even though prose broadly calls this a content replacement. Rust follows the source object shape. Compaction system checkpoint and usage/details remain preserved as extension JSON. Native v3 only, explicit legacy rejection, no Pi context projection/migration implementation.
7. **Version verification**: local executable version probe with default inclusive range **>=1.1.0, <=1.4.0**, or a caller-selected exact version inside that range. The previous fixed `">=1.1.0, <=1.2.0"` and `">=1.1.0, <=1.3.0"` expressions each retain their original upper bound. Only these three fixed expressions or a supported concrete release are accepted as an expectation. The actual probed version is passed to the transport. Remote callers declare a concrete version; there is no remote verification or invented handshake. Versions require exactly three ASCII decimal u64 components, without leading zeros; whitespace, prereleases and build metadata are rejected. Only local probe output retains the existing surrounding-whitespace trim. `PI_VERSION` and `SOURCE_COMMIT` identify the latest reviewed release, not the whole range.
8. **Shutdown**: stdin EOF is orderly shutdown, distinct from abort/clear_queue/forced process termination. There is a bounded close deadline. Owner drop is cleanup, clone drop is not.
9. **Exit/EOF races**: process exit independently wakes the reader. A 50 ms bounded drain preserves ready protocol output where practical despite inherited stdout handles. At the exit deadline, a nonwaiting check validates buffered records plus at most 4 KiB of ready input. An unfinished record at that byte boundary permits a single nonwaiting EOF probe of at most one extra byte; lookahead data is retained without further validation. Ready protocol failures take precedence over process exit, including partial JSON with ready EOF; this check never dispatches events or replies and never waits for an open pipe. Unexpected stdout EOF grants 100 ms for direct-child status, then becomes `Eof` and triggers owned-child cleanup if the child remains alive. Detached descendants themselves are outside lifecycle ownership.
10. **Required nullable fields**: explicit null is accepted, absence is rejected for cycle success data, entries/tree leafId, native entry parentId and context-edit replacement. Open tool payloads require presence but admit JSON null. Known nested assistant events validate their required fields and toolcall_end's actual tool-call shape.
11. **Terminal-event ordering**: event-channel closure is not authoritative terminal publication. PiEvents drains queued events, then awaits the terminal watch, yields exactly one error, then None. Event-first regressions cover 3000 immediate reader failures, 1000 buffered-event/failure connections, panic, EOF, owner drop and cancelled waiting. An unchanged external 3000-iteration harness independently reproduces failure before the fix and passes after it.
12. **Configuration side effects**: Pi persists steering/follow-up modes and auto-compaction/retry toggles in the selected agent directory. The Rust transport does not directly edit user files, but these RPCs are not readonly. Use an isolated agent directory for acceptance tests. Authentication refreshes and process environment policy belong to the host/Pi, not this SDK.
13. **Toolchain/platform evidence**: the 0.2.0 CI matrix targets native Linux, macOS and Windows on stable and exact **1.88.0**, including all features, stress and Debug/Release stack/float/UI/close regressions. Local upgrade results are recorded in [verification](verification.md#current-status-summary). The verification record separates completed hosted jobs from local real-Pi endpoint acceptance; older 0.1.0 results retain their original version scope. Remote vendor deployment remains unverified. See the [release checklist](publishing.md).
14. **Integer millisecond compatibility**: the existing public `UiMethod.timeout: Option<u64>` and the two retry events' `delay_ms: u64` accept finite nonnegative wire fractions by truncating toward zero. For example, 1500.5 becomes 1500 and 0.75 becomes 0. Integer values, including values above 2^53 and `u64::MAX`, are decoded directly without an f64 conversion. Negative, nonnumeric and out-of-range values fail known-record decoding. Missing/null UI timeout remains `None`; missing/null retry delay remains invalid. Pi owns the actual dialog timeout and retry scheduling. Fractional `durationMs`, usage and cost fields retain their existing floating-point behavior. See the [consumer contract](events.md#millisecond-fields).
