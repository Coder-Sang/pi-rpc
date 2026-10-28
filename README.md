# pi-rpc

[![crates.io](https://img.shields.io/crates/v/pi-rpc.svg)](https://crates.io/crates/pi-rpc)
[![API documentation](https://docs.rs/pi-rpc/badge.svg)](https://docs.rs/pi-rpc)
[![Verify](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml/badge.svg?branch=main)](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml)

A bounded, asynchronous Tokio JSONL client for **Pi 1.1.0 through 1.4.0**. Rust imports use `pi_rpc`. The crate provides typed commands, ordered session events, local subprocess ownership, injected transports and optional native session files. Install the Pi executable separately.

| Source | Pi support | Minimum Rust |
|---|---|---|
| **crates.io 0.2.2** | **>=1.1.0, <=1.4.0** | **1.88**, edition 2024 |
| **Git 0.2.3 development** | Same range; adds `PiProcess::spawn_with_version` | **1.88** |

The badges and hosted API documentation describe the latest published release. The new startup API is currently available only from Git or a local checkout. See the [changelog](https://github.com/Coder-Sang/pi-rpc/blob/main/CHANGELOG.md) and [verification records](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/verification.md) for release history and completed checks.

## Installation

```toml
[dependencies]
pi-rpc = "0.2.2"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

This requirement accepts compatible 0.2.x updates. To pin a historical release, use `pi-rpc = "=0.2.0"` (Pi 1.1.0–1.2.0, Rust 1.88) or `pi-rpc = "=0.1.0"` (Pi 1.1.0, Rust 1.85). Commit the application's `Cargo.lock` and update deliberately.

For unpublished 0.2.3, replace the registry dependency with a Git dependency pinned by `rev` to a verified commit, or use `path = "../pi-rpc"`. The [candidate evidence procedure](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/verification.md#candidate-evidence) explains how to identify a commit whose six CI jobs passed.

### Cargo features

| Feature | Default | Enables |
|---|---|---|
| `subprocess` | Yes | Local `PiProcess` and `ProcessOptions` |
| `session_file` | No | Native v3 `SessionFile`, `SessionLimits` and `SessionFileBuilder` |

Use `default-features = false` for injected transports only. Add `features = ["session_file"]` for native sessions, with or without default features. `protocol`, `codec`, `transport` and `client` are always available modules.

## Local subprocess

Call this read-only example from a live Tokio runtime with `subprocess` enabled. It installs an event consumer, queries state and closes the owned process:

```rust,no_run
#[cfg(feature = "subprocess")]
async fn inspect_session() -> pi_rpc::Result<()> {
    use pi_rpc::{PiProcess, ProcessOptions};

    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        executable: "/absolute/path/to/pi".into(),
        args: vec!["--no-session".into()],
        ..Default::default()
    }).await?;
    let consumer = tokio::spawn(async move {
        while let Some(record) = events.recv().await {
            match record {
                Ok(event) => { /* Handle the session event. */ }
                Err(terminal) => break,
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

`spawn` checks `--version` before starting `--mode rpc`, including with an exact `expected_version`. Its default expectation is `">=1.1.0, <=1.4.0"`. The older fixed ranges ending in 1.2.0 and 1.3.0 retain their original upper bounds. A concrete expectation such as `"1.3.0"` requires an exact match. Other range syntax is rejected.

In the **0.2.3 Git development version**, a host that knows the executable's release can skip the probe:

```rust,no_run
#[cfg(feature = "subprocess")]
async fn start_with_known_version() -> pi_rpc::Result<()> {
    use pi_rpc::{PiProcess, ProcessOptions};

    let (client, mut events, mut process) =
        PiProcess::spawn_with_version(ProcessOptions::default(), "1.4.0").await?;
    let consumer = tokio::spawn(async move { while events.recv().await.is_some() {} });
    process.close().await;
    consumer.await.expect("event consumer");
    Ok(())
}
```

The caller guarantees the declaration is accurate. The method enforces the supported range and `expected_version` without checking the executable's reported version. Declarations must be canonical three-part releases without whitespace, prerelease or build metadata. `startup_timeout` only controls `spawn`'s probe; it is unused by `spawn_with_version`. Neither method sends initialization RPCs or waits for application readiness.

Pi inherits the process environment. Some configuration RPCs persist Pi settings, and Pi may refresh authentication. The host owns installation, credentials, isolation and access policy. See the [connection guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/connections.md) for diagnostics, manual UI configuration and lifecycle details.

## Injected transport

Adapt an ordered `AsyncRead + AsyncWrite` pair with `Transport::new(reader, writer, declared_version)`, then call `PiConnection::connect(transport, config)`. The host declares the actual concrete version and owns remote stderr, delivery and process termination. There is no remote version handshake.

Run `cargo run --example injected --no-default-features` for a complete in-memory example using no Pi or model. The [connection guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/connections.md#injected-transport) also shows transport setup.

## Events, ownership and limits

- All 33 commands have typed methods. Prompt acceptance means started, queued or handled; it does not mean completion.
- Consume `PiEvents` continuously: one ordered session stream, generally without originating command IDs. See the [event guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/events.md) for text, thinking, tools, custom messages and UI.
- Choose `UiPolicy::Manual` before connecting to answer dialogs. The default cancels every dialog and still delivers its event. One-way UI notifications receive no reply.
- Keep the connection/process owner alive. Dropping a client clone is inert; dropping an owner stops I/O and a local process owner also signals child termination. Explicit close/termination waits for cleanup while the runtime is alive.
- `clear_queue`, `abort`, orderly `close` and forced `terminate` have separate meanings. A timeout or cancelled command future can leave an uncertain outcome; the SDK does not automatically retry it.
- Defaults are 32 MiB per JSON record, 64 MiB of writer bytes, 128 MiB of queued event wire bytes, 128 pending commands, 128 writer records and 1,024 events. `Config` adjusts these limits. JSON depth is fixed at 512 containers. Wire budgets do not bound process memory.
- Oversized/malformed inbound records and event overflow terminate the connection. Buffered events drain before one terminal error, followed by `None`. Model errors remain ordinary message outcomes.

See [connections and capacity](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/connections.md#bounds-and-backpressure) for images, historical responses, admission and shutdown behavior.

## Native sessions

With `session_file`, build Pi-native v3 JSONL using `SessionFileBuilder`, or parse caller-supplied bytes with `SessionFile::parse`. Native roles, usage, images, custom data and branch/checkpoint records are preserved. There is no `import_history` RPC or vendor-history conversion.

`encode()` returns a complete `Vec<u8>`; it does not stream to a file or `Write`. `SessionLimits` adjusts record/file bytes and entry count at parse/encode time. Builder appends do not enforce those cumulative budgets. File delivery, `--session`, permissions and atomic replacement belong to the host.

The [native-session guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/native-sessions.md) covers construction, JSONL, Usage, images, validation and memory. Run `cargo run --example native_session --features session_file` for an offline, in-memory example.

## Version and stability policy

The 0.2.x technical preview preserves documented public Rust paths, field types, features and Rust 1.88. Source-breaking changes, including additions to exhaustively matched enums, require a new minor series with migration notes.

Pi admission covers canonical releases in the inclusive 1.1.0–1.4.0 range. `PI_VERSION` (1.4.0) and `SOURCE_COMMIT` identify the latest reviewed upstream baseline. Admission alone does not establish review or live acceptance of a future in-range release. See [compatibility](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/compatibility.md) and the [release policy](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/publishing.md#version-and-release-policy).

## Development

Development requires stable Rust with rustfmt/Clippy, exact Rust 1.88.0, Bash, Git, Python 3.11+ and a native C/assembly compiler. Windows uses Git Bash, MSVC and the Windows SDK. Normal verification uses only in-memory transports and a compiled fake Pi.

Read [CONTRIBUTING](https://github.com/Coder-Sang/pi-rpc/blob/main/CONTRIBUTING.md) for commands, [verification](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/verification.md) for results/evidence, and [publishing](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/publishing.md) for release gates. Development guides use `main`; published archives retain their tagged snapshots. [Real Pi acceptance](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/acceptance.md) is separately opt-in and may persist settings or incur model charges.

Project code and documentation use [Apache-2.0](https://github.com/Coder-Sang/pi-rpc/blob/main/LICENSE), with retained [third-party notices](https://github.com/Coder-Sang/pi-rpc/blob/main/THIRD_PARTY_NOTICES.md). See [provenance](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/provenance.md) and [security reporting](https://github.com/Coder-Sang/pi-rpc/blob/main/SECURITY.md).
