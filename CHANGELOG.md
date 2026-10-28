# Changelog

## 0.2.3 — Unreleased

### Added

- `PiProcess::spawn_with_version(options, declared_version)` starts local RPC directly from a caller-declared version, retaining supported-version admission, legacy range bounds and exact `expected_version` locks. This API is Git-only until 0.2.3 is published.

### Changed

- Retain `spawn`'s bounded version probe, including exact expectations. Both startup paths share supervision and cleanup; `startup_timeout` applies only to probing. Rust 1.88, public paths/features, Pi 1.1.0–1.4.0 and dependency versions remain unchanged.
- Organize connection, native-session and contributor guides; clarify in-memory session encoding, Usage and memory budgets. Consolidate release instructions and verification records, add crate categories and omit GitHub-only configuration from the crate archive.

## 0.2.2 — 2026-10-27

- Extend inclusive Pi admission to **1.1.0 through 1.4.0**, retaining Rust **1.88**, public paths, field types and features. The fixed `>=1.1.0, <=1.2.0` and `>=1.1.0, <=1.3.0` expectations keep their original upper bounds; concrete expectations retain exact matching.
- Set the latest reviewed `PI_VERSION` to **1.4.0**, source commit `4a080252f06caf63a525d7873c5d94d3c23887e0`. Keep one shared wire implementation and unchanged dependency versions.
- Extend fake subprocess, Transport lifecycle, public-path and concrete acceptance-entrypoint regressions through 1.4.0, including rejection before probing or RPC startup as appropriate. Acceptance examples still reject every fixed range expression.
- Verify exact locks reject the `1.3.1`/`1.3.10` prefix collision in both directions before RPC startup. Matching-lock controls start RPC; local mutation controls detect either direction of an incorrect prefix comparison.
- Add open compatibility-metadata preservation, fragmented CRLF delivery of larger summaries and paired recovery omissions, and native-v3 history round trips. These synthetic tests preserve the distinction between active context and native history; they do not execute Pi's recovery algorithm.
- Document Pi-owned classifier image inputs, summary budgets, retry/recovery and request-identity changes. No real Pi 1.4.0 or paid-model acceptance is claimed. Published 0.2.1 retains Pi 1.1.0–1.3.0 support.

## 0.2.1 — 2026-10-18

- Extend inclusive Pi admission to **1.1.0 through 1.3.0**, retaining exact version locks and Rust **1.88**. The previous fixed `>=1.1.0, <=1.2.0` configuration keeps its original upper bound. The latest reviewed Pi source is `71023aec1546d6d677a65669c5665c0927ce0dd0`.
- Preserve Pi 1.3.0 tool-call `argumentsError` and model `supportsMidConvoEffort` metadata through existing extension JSON without changing public field types. Add fragmented-stream, history, native-session and three-version admission/lifecycle regressions.
- Preserve ready protocol failures when process-exit draining crosses its 50 ms deadline. Final validation is nonwaiting and byte-bounded, including a single-byte EOF probe for unfinished records at the exact 4 KiB budget; dispatch cutoff, EOF grace and owner close deadlines remain unchanged.
- Add deterministic terminal/resource regressions covering blocked partial writes, flushes, queued records, pending and budget release, and explicit shutdown-call assertions that cannot be hidden by a background panic.
- Require concrete Pi versions in opt-in acceptance entrypoints, rejecting current and legacy range expressions before probing. Verify the `PI_VERSION` default and exact version forwarding with a std-only fake probe, including rejection before RPC startup and local negative controls. Run the entrypoint regressions in normal and Debug/Release gates without real Pi or model requests.
- Keep the shared RPC implementation. Provider behavior changes remain Pi-owned; no real Pi or paid-model acceptance for 1.3.0 is claimed. Published 0.2.0 retains its original Pi 1.1.0–1.2.0 support.

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

[Unreleased changes](https://github.com/Coder-Sang/pi-rpc/compare/v0.2.2...main) · [0.2.2 changes](https://github.com/Coder-Sang/pi-rpc/compare/v0.2.1...v0.2.2) · [0.2.1 changes](https://github.com/Coder-Sang/pi-rpc/compare/v0.2.0...v0.2.1) · [0.2.0 changes](https://github.com/Coder-Sang/pi-rpc/compare/v0.1.0...v0.2.0)
