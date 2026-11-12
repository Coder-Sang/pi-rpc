# Pi-Bolt message ID extension

This **unpublished `0.3.3-bolt.2` special branch** builds on official **0.3.2** with the opt-in `bolt_message_id` feature and `BoltMessageIdV1` profile. The prerelease number identifies this special build, not a different official baseline. Cargo has `publish=false`. Official Pi13/Pi14/Pi15 types support Pi 1.3.0–1.5.2; the extension independently admits the reviewed **Pi 1.5.0 and Pi 1.5.2** fork bases. Default features and the Rust 1.88 minimum are unchanged.

## Source and capability

The latest reference is **Coder-Sang/Pi-Bolt bolt-v1.2.1**, fixed commit [`23d01fa7449729b47bde039f0095f5f6732fb55e`](https://github.com/Coder-Sang/Pi-Bolt/tree/23d01fa7449729b47bde039f0095f5f6732fb55e), based on Pi **1.5.2**. Reviewed contracts are [RPC input correlation](https://github.com/Coder-Sang/Pi-Bolt/blob/23d01fa7449729b47bde039f0095f5f6732fb55e/packages/coding-agent/docs/rpc-commands.md#prompt) and [queue tracking](https://github.com/Coder-Sang/Pi-Bolt/blob/23d01fa7449729b47bde039f0095f5f6732fb55e/packages/coding-agent/docs/json.md#input-tracking). The earlier **bolt-v1.1.1** review at [`7193587994d2fee4b62a213c1f14d82acb832d5c`](https://github.com/Coder-Sang/Pi-Bolt/tree/7193587994d2fee4b62a213c1f14d82acb832d5c), based on Pi **1.5.0**, remains supported. `SOURCE_COMMIT` identifies the latest reviewed reference. The Rust implementation and synthetic fixtures are project-authored; no executable or fork implementation is vendored.

Official Pi-Bolt's version formatter and this fork can report the same underlying Pi/Bolt releases. Recognition of a version string does not establish tracking support. Select the extension profile only for an executable or transport you trust to implement this contract. Automatic dispatch always returns official Pi13/Pi14/Pi15, including when this feature is enabled. The extension admits only the individually reviewed bases **1.5.0** and **1.5.2**; it does not widen global admission or old range limits.

The newer fork's preflight path retains input IDs through handled/queued/started responses, and queued-image normalization preserves the existing tracking identity and wire order. Its additional provider, subagent, page and mobile-view commands are outside this extension's typed API. The SDK sends no capability negotiation or subscription during startup; mobile views are not enabled. Existing official commands and UI handling remain available. See the [source review](compatibility.md#pi-bolt-121-message-id-source-review-2026-11-13).

## Build and submit

Check out `feature/pi-bolt-message-id` and use a local path dependency with version `=0.3.3-bolt.2` and `features = ["bolt_message_id"]`. This feature enables `pi_1_5`; subprocess ownership remains separately controlled by `subprocess`. For a transport-only caller, disable defaults and enable only `bolt_message_id`.

```rust,no_run
use pi_rpc::{BoltMessageIdV1, PiClient};
use pi_rpc::protocol::bolt_message_id_v1::MessageId;

async fn submit(client: &PiClient<BoltMessageIdV1>) -> pi_rpc::Result<()> {
    // The host must register this ID and start its event consumer first.
    let accepted = client.prompt_with_message_id(
        "Inspect the supplied input", None, None, MessageId::new("input-A")?,
    ).await?;
    assert_eq!(accepted.message_id.as_str(), "input-A");
    // accepted.disposition is started, queued or handled, not completion.
    Ok(())
}
```

`steer_with_message_id(message, images, id)` and `follow_up_with_message_id(message, images, id)` return `InputAcceptance<QueuedInputDisposition>`. Ordinary prompt/steer/follow_up methods remain available and omit the ID, including on the extension profile. Nonempty IDs preserve whitespace and Unicode verbatim. Clones share their string allocation. Each ID is a top-level `message_id`, independent of the SDK-generated RPC `id`; existing message, images and streaming behavior are unchanged. IDs count toward the existing escaped-JSON record and writer budgets, including framing. Wire budgets do not bound total memory.

Use `PiConnection::<BoltMessageIdV1>::connect` for a trusted injected transport. With `subprocess`, use `PiProcess::<BoltMessageIdV1>::spawn` or `spawn_with_version`. Probe-based startup recognizes the existing strict official Pi-Bolt decoration, such as `1.5.2 (Pi-Bolt 1.2.1, linux-x64, JIT off)`, and checks the underlying Pi version. Declared startup accepts `"1.5.2"` or the retained `"1.5.0"`, skips probing and trusts the caller. The shared `expected_version` defaults to `>=1.3.0, <=1.5.2`; an old range ending in 1.5.0 or an exact `"1.5.0"` lock still rejects 1.5.2. Pi 1.5.1 and unreviewed later bases fail before Transport I/O or RPC startup. Supporting another fork base requires a separate source/capability review. No capability handshake or model request is added.

Run `cargo run --example bolt_message_id --no-default-features --features bolt_message_id` for a complete in-memory submission, concurrent event consumption and orderly close, without an installed Pi/Bolt or model.

## Consume queue tracking

`PiEvents<BoltMessageIdV1>` yields `protocol::bolt_message_id_v1::Event`:

- `QueueUpdate(update)` includes legacy `steering`/`follow_up` text snapshots and optional `QueueTrackingV1`. Every queue update has this variant, including updates before tracking is enabled.
- `Pi(event)` wraps all other unmodified Pi15 events: streaming text/thinking, tools, custom messages, UI, unknown events and uncorrelated responses. Dialog policy and ordered single terminal-error delivery are unchanged.

The first queued or started identified input enables tracking for that live session instance. Tracking can remain absent before then; a handled or rejected input does not itself cause handoff. Events may precede acceptance, so register correlation and consume continuously before submitting. Acceptance echoes are required and compared exactly by the identified methods. Missing or mismatched echoes return a `Protocol` error to that call with an uncertain submission outcome; they do not establish rejection or trigger retry. Malformed echo types/empty strings and malformed tracked records terminate the connection as other malformed known records do.

`instance_id` identifies a live session object, not the persisted session. It changes on replacement/restart, and `sequence` starts over. The SDK delivers the wire order and does not infer continuity, reorder or suppress events. Hosts own gap detection and external stream binding. Schema `version` is strictly 1. Text snapshots and parallel ID arrays must align; `None` positions preserve unidentified user/system/custom inputs.

| `QueueChange` | Meaning |
|---|---|
| `Enqueue` | Input entered the steering/follow-up queue; no batch ID |
| `Consume` | An actual ordered queue drain; includes a batch ID |
| `PromptStart` | Initial prompt handoff, possibly including unidentified context; includes a batch ID |
| `Clear` | IDs actually removed from each pending queue; no batch ID |

Batch and instance IDs are opaque nonempty strings. A batch is an input scheduling/handoff operation, **not a provider receipt, output-message ID or assistant completion**. Multiple inputs can share a batch; multiple drains can precede one provider request, and retries/tool continuations can produce several responses after one handoff. Steering takes priority over follow-up, so there is no global FIFO across queues. `abort` retains pending inputs and does not requeue consumed ones; `clear_queue` removes only pending inputs. The SDK does not synthesize tracking or map assistant responses back to input IDs.

IDs are correlation values, not deduplication/idempotency keys. Cancellation or timeout can leave an uncertain execution outcome; no automatic replay occurs. Tracking is runtime-only and is not added to model messages, history, native session files or replay logs. Existing Usage and custom fields remain unchanged.

## Verification scope

Normal verification uses in-memory transports and compiled std-only fake processes. The optional feature expands the selection matrix to 64 combinations; default official features stay unchanged. Strict type-isolation controls, malformed schemas, exact wire budgets, shutdown/resource cleanup and synthetic fork events are tested. Native cross-platform acceptance requires six successful jobs and matching artifacts for the same committed candidate; actual commit/run/archive identities stay outside packaged source. Synthetic verification does not establish live fork/provider acceptance. See [verification](https://github.com/Coder-Sang/pi-rpc/blob/feature/pi-bolt-message-id/docs/verification.md) and [compatibility](https://github.com/Coder-Sang/pi-rpc/blob/feature/pi-bolt-message-id/docs/compatibility.md).
