# Migrating from 0.2 to 0.3

0.3.0 introduces the versioned protocol API below. Published 0.2.2 and its tags, checksums and acceptance records retain their original contract. This migration raises no Rust requirement: the minimum remains 1.88.

## Select a protocol type

`PiClient`, `PiEvents`, `PiConnection` and `PiProcess` now require `P: PiProtocol`, with no default type. Use `Pi13`, `Pi14` or `Pi15` and events from `protocol::v1_3`, `protocol::v1_4` or `protocol::v1_5`. Root and module client/process paths remain available with the new parameter. The former unversioned `KnownEvent` and `AssistantEvent` exports are removed; `protocol::Event<P>`, `Record<P>` and `decode_record::<P>` require an explicit profile.

```rust,no_run
# #[cfg(all(feature = "subprocess", feature = "pi_1_5"))]
# async fn example() -> pi_rpc::Result<()> {
use pi_rpc::{Pi15, PiEvents, PiProcess, ProcessOptions};
use pi_rpc::protocol::v1_5::Event;
let (_client, mut events, mut process): (_, PiEvents<Pi15>, _) =
    PiProcess::<Pi15>::spawn_with_version(ProcessOptions::default(), "1.5.2").await?;
// The caller guarantees the installed version. No --version process is started.
let consumer = tokio::spawn(async move {
    while let Some(event) = events.recv().await {
        if let Ok(Event::Ui(_request)) = event { /* handle this version's UI event */ }
    }
});
process.close().await;
consumer.await.expect("consumer");
# Ok(()) }
```

Shared declaration templates cover only wire shapes audited as identical. Future protocol differences must use version-specific declarations/decoding and leave the older profile contracts intact.

Static profile mismatch returns `Error::Version` before RPC starts. Trait implementations are sealed. Shared Message/Usage/native-v3 payloads retain open fields; nominal event separation does not normalize provider behavior or guarantee identical event sequences across different Pi releases.

## Automatic selection and skipped probes

`subprocess::spawn_auto(options)` performs the bounded version probe. `spawn_auto_with_version(options, concrete_version)` trusts the caller and skips it. Both return `AutoProcess`; `client::connect_auto(transport, config)` returns `AutoConnection`. Match `V1_3`, `V1_4` or `V1_5` once; each variant owns its typed client, events and owner. These enums are non-exhaustive, so downstream matches need a wildcard arm. There is no erased common event stream.

`PiProcess::<P>::spawn` still probes, including exact expected versions. Both declaration APIs validate versions and expectations without probing. `startup_timeout` only governs the probe; it is not an RPC-ready deadline. No entrypoint adds initialization RPCs or model requests.

`info()` on Transport, client, events, connection/process owner and auto owner exposes immutable concrete version metadata. `VersionSource::Probed` means a local probe returned it; `Declared` means the caller supplied it. A source does not establish application readiness or remote verification.

## Features and admission

Defaults are `subprocess`, `pi_1_4`, `pi_1_5`. Add `pi_1_3` to enable older supported connections. Protocol features are additive and may coexist. Without any protocol feature, codec and optional session-file utilities still work; auto connection/startup reports `NoProtocolEnabled`. A globally supported but disabled version reports `VersionNotEnabled { version, feature }`, with no fallback.

As of 0.3.2, the global range is **>=1.3.0, <=1.5.2**. Pi13 matches 1.3.x, Pi14 matches 1.4.x, and Pi15 matches 1.5.0–1.5.2. Canonical three-part release syntax is required; whitespace, prereleases, metadata, leading zeros and overflowing fields are rejected. Probed output retains existing whitespace trimming.

`expected_version` remains a String and exact versions remain exact locks. The recognized previous `>=1.1.0, <=1.3.0` range now admits only 1.3.0; `>=1.1.0, <=1.4.0` admits 1.3.0–1.4.0. Their upper bounds never widen. The previous 0.3.0/0.3.1 default `>=1.3.0, <=1.5.0` remains recognized and still stops at 1.5.0. The old range ending in 1.2.0 has no intersection and is rejected before probing. Other range syntax is not accepted. Pi 1.1/1.2 callers can retain published 0.2.2.

## Lifecycle and data contracts

The SDK retains one continuous reader, one serialized writer, the existing command correlation/state machine, bounded exit draining, close deadlines, stack/depth protection, float round trips and admission budgets. Events drain before one terminal error and then None. `terminate()` returns the OS process-exit result; connection/event termination remains `Terminated`.

Native v3 session files remain independent of protocol profiles. Pi 1.5 `Usage.cost.totalFromProvider` stays in `Cost.extra`; `responseModel` stays in `Message.fields`. These values are forwarded unchanged, without calculating or inferring provider cost semantics. See [events](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/events.md), [connections](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/connections.md) and [compatibility](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/compatibility.md).
