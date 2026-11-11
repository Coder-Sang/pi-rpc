# pi-rpc

[![crates.io](https://img.shields.io/crates/v/pi-rpc.svg)](https://crates.io/crates/pi-rpc)
[![API documentation](https://docs.rs/pi-rpc/badge.svg)](https://docs.rs/pi-rpc)
[![Verify](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml/badge.svg?branch=main)](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml)

A bounded, asynchronous Tokio JSONL client with versioned protocol types for Pi. It provides typed commands, ordered session events, local process ownership, injected transports and optional native v3 session files. Install Pi separately; Rust imports use `pi_rpc`.

| Version | Pi support | Minimum Rust |
|---|---|---|
| **0.3.2** | **>=1.3.0, <=1.5.2**, versioned API below | **1.88**, edition 2024 |
| **0.2.2** | **>=1.1.0, <=1.4.0**, previous API | **1.88**, edition 2024 |

Version 0.3 introduces explicit protocol types and a source-breaking migration from 0.2. The badges describe registry packages, hosted API documentation and committed main. See [migration](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/migration-0.3.md), [changelog](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/CHANGELOG.md) and [verification](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/verification.md).

## Installation and features

Use Rust **1.88+** and install a supported Pi release separately:

```toml
[dependencies]
pi-rpc = "0.3.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

Applications retaining the 0.2 API or Pi 1.1/1.2 support can use `pi-rpc = "0.2.2"`; its [release evidence](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/verification.md#022-release-2026-10-27) and immutable tag retain their original contract.

| Feature | Default | Enables |
|---|---|---|
| `subprocess` | Yes | Local process APIs and supervision |
| `pi_1_3` | No | `Pi13` and `protocol::v1_3` for Pi 1.3.x |
| `pi_1_4` | Yes | `Pi14` and `protocol::v1_4` for Pi 1.4.x |
| `pi_1_5` | Yes | `Pi15` and `protocol::v1_5`, Pi 1.5.0–1.5.2 |
| `session_file` | No | Native v3 parsing, encoding and `SessionFileBuilder` |

Protocol features are additive. All connections must also satisfy the global 1.3.0–1.5.2 range. For Transport-only Pi 1.4, use `default-features = false, features = ["pi_1_4"]`. Native sessions and JSONL codec utilities work without a protocol feature; connecting requires one. No protocol feature changes another version's event type.

## Typed local process

The following example uses the **0.3 API** and the dependency's default features. Your application does not need a local `[features]` table. Select the protocol explicitly; consume events before submitting commands:

```rust,no_run
async fn inspect_session() -> pi_rpc::Result<()> {
    use pi_rpc::{Pi15, PiProcess, ProcessOptions};
    use pi_rpc::protocol::v1_5::{Event, KnownEvent};

    let (client, mut events, mut process) = PiProcess::<Pi15>::spawn(ProcessOptions {
        executable: "/absolute/path/to/pi".into(),
        args: vec!["--no-session".into()],
        ..Default::default()
    }).await?;
    let consumer = tokio::spawn(async move {
        while let Some(record) = events.recv().await {
            match record {
                Ok(Event::Known(KnownEvent::AgentStart)) => { /* Pi15 event */ }
                Ok(_) => {}
                Err(_) => break,
            }
        }
    });
    let state = client.get_state().await;
    process.close().await;
    consumer.await.expect("event consumer");
    state?;
    Ok(())
}
```

`spawn` runs the bounded `--version` probe, then checks the actual version against `Pi15` and `expected_version` before starting RPC. The default expectation is `">=1.3.0, <=1.5.2"`. Previous ranges ending in 1.3.0, 1.4.0 or 1.5.0 retain their upper bounds and intersect the new supported range; the former range ending in 1.2.0 is rejected. A concrete expectation remains an exact lock.

Since **0.3.1**, probe-based startup also recognizes complete official Pi-Bolt output, such as `1.5.0 (Pi-Bolt 1.1.1, linux-x64, JIT off)`. Admission, protocol selection and `info().version()` use the underlying Pi version (`1.5.0`), independently of the Bolt version (`1.1.1`). Version 0.3.0 retains its original probe rules. No fork-specific message tracking is added. See [Pi-Bolt startup](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/connections.md#pi-bolt-version-output).

When the host already knows the installed release, `PiProcess::<Pi15>::spawn_with_version(options, "1.5.2")` skips the probe. The caller guarantees the declaration is accurate; all admission checks still apply. `startup_timeout` controls only probing. Neither entrypoint sends initialization RPCs or waits for application readiness.

For automatic selection, `subprocess::spawn_auto(options)` probes and returns an `AutoProcess` variant containing the corresponding typed client, event stream and process owner. `spawn_auto_with_version(options, version)` selects from a trusted declaration. Match the variant once to obtain static types; unavailable protocol features cause an explicit error, never fallback. Every handle exposes immutable `info()` with the concrete version and `Probed`/`Declared` source.

Pi inherits the process environment. The host owns installation, credentials, isolation and access policy. Pi can persist settings or refresh authentication. See [connections](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/connections.md) for lifecycle, diagnostics and Manual UI configuration.

## Injected transport

Create `Transport::new(reader, writer, concrete_version)`, then call `PiConnection::<Pi14>::connect(transport, config)` for a statically selected profile, or `client::connect_auto(transport, config)` for an `AutoConnection`. The remote host supplies the installed version and owns remote stderr, delivery and termination. There is no remote handshake.

Run `cargo run --example injected --no-default-features --features pi_1_4` for a complete in-memory example using no Pi or model.

## Events, ownership and limits

- `PiEvents<P>` returns only `P::Event`. Each supported minor has distinct `Event`, `KnownEvent` and `AssistantEvent` types. Provider, tool and extension payloads remain open JSON; the SDK does not infer provider semantics.
- All 33 commands have typed methods. Prompt acceptance means started, queued or handled; it does not mean completion. Events form one ordered session stream, generally without originating command IDs. See [event consumption](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/events.md).
- Choose `UiPolicy::Manual` before connecting to answer dialogs. The default cancels dialogs and still delivers their events. One-way UI notifications receive no reply.
- Keep the connection/process owner alive. Dropping a client clone is inert; dropping an owner stops I/O and a local process owner also signals child termination. Explicit close/termination waits for cleanup while the runtime lives.
- `clear_queue`, `abort`, orderly `close` and forced `terminate` have separate meanings. Cancelled or timed-out commands can have uncertain outcomes; the SDK does not automatically retry them.
- Defaults are 32 MiB per JSON record, 64 MiB of writer bytes, 128 MiB of queued event wire bytes, 128 pending commands, 128 writer records and 1,024 events. `Config` adjusts these limits. JSON depth is fixed at 512 containers. Wire budgets do not bound process memory.
- Malformed/oversized inbound records and event overflow terminate the connection. Buffered events drain before one terminal error, followed by `None`. Model errors remain ordinary message outcomes.

## Native sessions

With `session_file`, build native v3 JSONL with `SessionFileBuilder`, or parse caller-supplied bytes with `SessionFile::parse`. Native roles, Usage, images, custom fields and branches are preserved. Session files are format-based and do not select a Pi protocol type.

`encode()` returns a complete `Vec<u8>`; it does not stream to `Write`. `SessionLimits` adjusts record/file bytes and entry count during parsing/encoding. Builder appends do not enforce cumulative byte/count budgets. File delivery, permissions and atomic replacement belong to the host. See [native sessions](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/native-sessions.md).

## Stability and development

The 0.3 minor introduces the versioned API and defaults described above; migrate explicitly from 0.2. Rust 1.88 remains the minimum. `PI_VERSION = "1.5.2"` and `SOURCE_COMMIT` identify the latest reviewed source. In-range future patch releases pass admission but still require source review and verification; admission does not establish live acceptance. See [compatibility](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/compatibility.md) and [release policy](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/publishing.md#version-and-release-policy).

Development uses current stable Rust with rustfmt/Clippy and exact Rust 1.88.0, Bash, Git, Python 3.11+ and a native C/assembly compiler. Windows requires Git Bash, MSVC and the Windows SDK. Normal verification uses only in-memory transports and compiled fake processes.

Read [CONTRIBUTING](https://github.com/Coder-Sang/pi-rpc/blob/main/CONTRIBUTING.md), [verification](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/verification.md) and [publishing](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/publishing.md). [Real acceptance](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/acceptance.md) is separately opt-in and may persist settings or incur charges. Synthetic verification does not establish real-Pi or paid-model acceptance; candidate evidence is recorded separately.

Project code and documentation use [Apache-2.0](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/LICENSE), with [third-party notices](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/THIRD_PARTY_NOTICES.md). See [provenance](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/docs/provenance.md) and [security reporting](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.2/SECURITY.md).
