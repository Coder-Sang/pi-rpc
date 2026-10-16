# Verification results

## Current status summary

Development **pi-rpc 0.2.0** supports **Pi >=1.1.0, <=1.2.0** and requires **Rust 1.88**. Pi **1.2.0**, commit `06a466935536f8c9bcacb2d5efc0fc021be1bdff`, is the latest reviewed source baseline. Supported version admission, source review and actual runtime acceptance are distinct; see [compatibility](compatibility.md#version-range-and-review-evidence).

Version 0.2.0 has not been uploaded to crates.io. This record separates local verification, prior real-Pi acceptance and hosted CI. Normal verification uses fake/in-memory transports and makes no paid-model requests.

## Local verification (2026-10-16)

The following checks passed at completed baseline **`c7d4f718e888f6237e9ff956c80f51f4d38d3f25`** on **macOS aarch64**, using stable **Rust 1.97.1** and exact **Rust 1.88.0**:

| Check | Stable | Rust 1.88.0 |
|---|---|---|
| Four-feature check/test matrix | 562 passing executions | 562 passing executions |
| Three-round fake transport stress | 243 passing executions | 243 passing executions |
| Debug/Release boundary regressions | 284 passing executions | 284 passing executions |
| Fresh packaged consumer, four features | PASS | PASS |
| Formatting, strict Clippy and rustdoc | PASS | Covered by stable strict checks |

The boundary suites cover stack-protected parsing, conversion, cloning, serialization, release and failure paths; floating-point bit preservation; image/capacity limits; UI replies and shutdown races. Version tests cover both endpoints, synthetic intermediate versions, malformed declarations and exact locks. Probe markers verify rejection before probing or RPC startup as appropriate. The compacted-session restoration regression runs without Pi or a model.

All **33** Python checker regressions, local Markdown links/anchors and whitespace checks passed. Package creation, source/VCS/license/metadata inspection and publication **dry-run** passed from a clean source snapshot. Each independent consumer used fresh dependency resolution, exercised all four features and ran in-memory/native-session examples without Pi. Source bytes were compared against the checked snapshot; no dirty-package bypass was used.

## Real Pi acceptance

Separately authorized real acceptance completed on **macOS aarch64** on 2026-10-16. No credentials, generated conversations, private configuration or diagnostic traces are distributed in this repository.

| Acceptance scope | Result and limits |
|---|---|
| Pi 1.1.0 and 1.2.0, ten read-only RPCs and orderly close | PASS with stable and exact Rust 1.88.0 runners |
| Minimal hosted `anthropic / claude-opus-4-6` streaming request on each endpoint | PASS with exact Rust 1.88.0: expected text, assistant completion, non-aborted settlement and unique terminal delivery |
| Pi 1.2.0 extension UI, all nine methods, manual/cancel answers | PASS with stable |
| Pi 1.2.0 retry recovery/cancellation/disabled mode and automatic compaction | PASS with a synthetic provider/summary; not evidence of hosted-provider faults or model summarization |
| Pi 1.2.0 real tool, prompt/steer/follow-up queue, active abort, model compaction, fork/switch | PASS with stable after correcting the acceptance assertion |
| Pi 1.2.0 state/settings, bash/cancellation, native sessions, HTML export/clone/fork/switch | PASS with stable |

The corrected fork/switch assertion compares saved active context and native history independently: compaction can summarize a user message out of active context while retaining its native entry. The regression rejects either lost context or lost native history. This correction changes the acceptance harness, not production behavior; see [event consumption](events.md#active-context-and-native-history).

These local observations do not establish exhaustive model/event-variant coverage, remote-vendor deployment or live Pi execution on other platforms. The [opt-in acceptance guide](acceptance.md) documents isolation requirements. Real/paid tests are separate from the default CI matrix.

## Hosted CI

The [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) runs **Linux/macOS/Windows × stable/1.88.0**, six jobs. Each executes its feature matrix, three stress rounds, native Debug/Release regressions, archive inspection and a fresh packaged consumer. The manual real-Pi job remains opt-in and disabled on normal pushes.

The [completed baseline run](https://github.com/Coder-Sang/pi-rpc/actions/runs/38857092619) passed all six jobs for **`c7d4f718e888f6237e9ff956c80f51f4d38d3f25`**, including Windows stable and exact Rust 1.88.0. This is the same historical baseline identified in the README. Subsequent candidate conclusions and evidence are published by the workflow; the baseline is not a claim that later commits have already passed.

## Candidate evidence

The current gates also check generated Rustdoc HTML on every feature configuration with stable and exact 1.88.0. Local HTML file/fragment targets and project guide URLs are validated against the generated output and source snapshot; source line ranges and Rustdoc's encoded generic anchors are supported. External sites are not fetched by this check.

Each successful matrix job uploads **`release-evidence-<os>-<toolchain>`**, containing `release-evidence.json`. It records:

- `source.commit` and SHA-256 digests of every archived source file, mapped back to original source names.
- The package version/MSRV and actual compiler version.
- The inspected archive's name, byte count, file count and SHA-256.
- The CI run URL and matrix identity. Local records have `ci: null`.

The recorder checks metadata, licenses, archive safety, clean VCS identity and every archived source byte before writing the record. It reads one archive snapshot for both inspection and hashing. No absolute paths, authentication/configuration payloads or model traces are included. Locally, run `python3 scripts/record-evidence.py` after the [complete release checks](publishing.md#final-verification-before-upload); it honors Cargo's effective target directory and supports `--archive` / `--output` overrides outside packaged source.

A record attests only to its archive/source inspection, not to tests it did not execute. For CI acceptance, confirm **all six jobs succeeded for the same `source.commit`** and inspect their preceding test steps. Keep the relevant JSON and complete run link outside the crate archive, preserving them before CI artifact retention expires. Different platforms/Cargo versions can produce different archive digests; compare the eventual upload with the selected upload platform's archive, not with an assumed universal digest.

The candidate's own commit and digest are recorded after it exists, outside the package. Documentation therefore records a completed baseline and the evidence procedure, rather than changing its own SHA after every verification run. Real-Pi observations above remain separately dated and scoped; these fake-only gates do not rerun or extend them.

## 0.1.0 release (2026-10-14)

Published [pi-rpc 0.1.0](https://crates.io/crates/pi-rpc/0.1.0) retains **Pi 1.1.0 / Rust 1.85**. Its immutable [v0.1.0 source](https://github.com/Coder-Sang/pi-rpc/tree/v0.1.0) is commit `41a7348d418ce1e1a591fa66246cc9f21e5d26b8`. The [six hosted jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/38497009320), registry consumer and [docs.rs](https://docs.rs/pi-rpc/0.1.0/pi_rpc/) verification passed for that release. Those historical results do not establish 0.2.0 acceptance.
