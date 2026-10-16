# Changelog

## 0.2.0 — 2026-10-16

- Raise the minimum Rust version from **1.85** to **1.88** and support the inclusive **Pi 1.1.0 through 1.2.0** range, with latest reviewed source commit `06a466935536f8c9bcacb2d5efc0fc021be1bdff`.
- Preserve the existing public Rust paths, fields and features. The reviewed RPC/native v3 session schemas are unchanged; protocol coverage remains 33 commands, 24 events, 12 assistant stream variants and nine UI methods.
- Remove the direct `psm = "=0.1.27"` compatibility workaround; resolve `psm 0.1.32` with `ar_archive_writer 0.5.3`. Retain the existing stack, float, capacity and close protections.
- Add the public `PI_VERSION_REQUIREMENT` constant and default subprocess range admission; retain concrete `expected_version` exact locks. Reject out-of-range, malformed, prerelease and build-metadata versions. Add version admission, model-metadata preservation and startup state/UI/abort multiplexing regressions. Update CI, fresh-consumer and package metadata checks to Rust 1.88.
- Use immutable hard-linked fake executables in Linux subprocess version tests to avoid executable-copy races; retain executable copies on other platforms.
- Correct opt-in fork/switch acceptance to compare compaction-aware context and native history separately, with a no-model Transport regression that also rejects lost context or history.
- Publish per-job candidate evidence outside the crate archive, binding clean source, exact archive bytes, compiler and CI run without private payloads.
- Fix guide/license navigation in generated Rustdoc and add generated-HTML link checks for every feature configuration on stable and Rust 1.88.0.
- Pi 1.2.0 may report a busy run during extension startup before prompt acceptance or `agent_start`. Extensions must discriminate user/custom `before_agent_start` hooks. See the [upgrade notes](docs/compatibility.md#upgrade-from-pi-110-to-120).

Upgrading requires Rust 1.88 and a separately installed Pi release in the supported range; the published 0.1.x line retains its Pi 1.1.0/Rust 1.85 contract. Normal verification remains fake-only. Separately authorized real-Pi endpoint/UI results and hosted Opus 4.6 results are recorded in the [verification supplement](docs/verification.md#real-pi-acceptance).

## 0.1.0 — 2026-10-14

Initial technical preview, pinned to Pi **1.1.0**. See the [stability and release policy](docs/publishing.md#version-and-release-policy).

- Typed access to 33 RPC commands, 24 known events, 12 assistant stream variants and nine extension UI methods, with open custom messages and unknown events preserved.
- Tokio transport injection, default local subprocess ownership, ordered event delivery and deterministic close/terminal handling.
- Optional native v3 session parsing, encoding and indexed message construction through `session_file`.
- Bounded JSONL records, writer/event byte budgets and concurrent encoding admission, including multi-image inputs and image/history responses.
- Explicit 512-container JSON depth, stack-protected conversion and isolated 2 MiB stack regressions for accepted and rejected inputs.
- Bit-exact finite floating-point round trips, including signed zero and subnormal values. Fractional UI timeouts and retry delays decode into the existing integer public fields by truncating toward zero.
- Four-feature testing on Linux, macOS and Windows with stable and exact Rust **1.85.0**; package/source checks and fresh downstream consumer verification.
- Apache-2.0 project license, crates.io-only publishing metadata, all-feature docs.rs configuration and retained Pi MIT notices.

No public API, feature or production behavior was changed by the publication-preparation work. Real Pi and paid-model acceptance remain opt-in; normal validation uses in-memory transports and the test fake.
