# Changelog

## 0.1.0 — release candidate, not uploaded

Initial technical preview, pinned to Pi **1.1.0**. Publication preparation was completed on 2026-10-14; that date is not a registry release date. See the [stability and release policy](docs/publishing.md#version-and-release-policy).

- Typed access to 33 RPC commands, 24 known events, 12 assistant stream variants and nine extension UI methods, with open custom messages and unknown events preserved.
- Tokio transport injection, default local subprocess ownership, ordered event delivery and deterministic close/terminal handling.
- Optional native v3 session parsing, encoding and indexed message construction through `session_file`.
- Bounded JSONL records, writer/event byte budgets and concurrent encoding admission, including multi-image inputs and image/history responses.
- Explicit 512-container JSON depth, stack-protected conversion and isolated 2 MiB stack regressions for accepted and rejected inputs.
- Bit-exact finite floating-point round trips, including signed zero and subnormal values. Fractional UI timeouts and retry delays decode into the existing integer public fields by truncating toward zero.
- Four-feature testing on Linux, macOS and Windows with stable and exact Rust **1.85.0**; package/source checks and fresh downstream consumer verification.
- Apache-2.0 project license, crates.io-only publishing metadata, all-feature docs.rs configuration and retained Pi MIT notices.

No public API, feature or production behavior was changed by the publication-preparation work. Real Pi and paid-model acceptance remain opt-in; normal validation uses in-memory transports and the test fake.
