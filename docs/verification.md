# Verification results

## Current status summary

**pi-rpc 0.2.2** supports **Pi >=1.1.0, <=1.4.0** and requires **Rust 1.88**. Pi **1.4.0**, commit `4a080252f06caf63a525d7873c5d94d3c23887e0`, is the latest reviewed source baseline. Supported admission, source review and actual runtime acceptance are distinct; see [compatibility](compatibility.md#pi-140-source-review-2026-10-27). No live Pi 1.4.0/model acceptance is claimed. The completed baseline and final-candidate evidence procedure below are separate from publication observations, which are recorded only after confirmation.

The [0.2.1 release](#021-release-2026-10-18) was published on 2026-10-18 after complete clean-commit verification and six successful hosted jobs. Its actual registry bytes, exact Rust 1.88.0 registry consumer, docs.rs and uploaded-commit tag are confirmed below. The separate 0.2.1 bookkeeping did not replace its uploaded source. The [0.2.0 release](#020-release-2026-10-16) retains its separately recorded results. This record separates local verification, prior real-Pi acceptance and hosted CI. Normal verification uses fake/in-memory transports and makes no paid-model requests.

## Pi 1.4.0 development status (2026-10-27)

The 0.2.2 candidate retains public paths, field types, features, Rust 1.88 and dependency versions while extending default admission through 1.4.0. Both older fixed range configurations retain their own upper bounds; concrete versions remain exact locks. Fake probes cover defaults, legacy bounds, synthetic intermediate versions and rejection before probe/RPC launch. Acceptance entrypoint unit tests reject all three ranges and verify exact forwarding using only the std-only fake.

Targeted stable and exact 1.88.0 checks cover new compatibility-field preservation on model/state/cycle/list responses, fragmented CRLF delivery of summaries larger than 64 KiB and paired recovery omissions, and native-v3 round trips retaining the original messages. Existing image/float/stack/UI/close regressions remain required. These tests exercise SDK transport and data preservation, not Pi's internal recovery, provider, classifier or worker behavior.

The final clean candidate must pass the complete local feature/stress/Debug/Release/Python/document/package/dry-run/consumer checks and all six hosted jobs. Its actual commit, source hashes, archive digest, compiler and matching run are recorded outside the package following [candidate evidence](#candidate-evidence); historical results do not substitute for final-commit acceptance. Normal verification invokes no real Pi or paid model. The optional manual smoke remains pinned to Pi 1.2.0 and is skipped on ordinary pushes. The separately authorized 0.2.2 release must verify registry bytes, exact-MSRV registry consumption, docs.rs and the actual uploaded-commit tag after upload.

## 0.2.2 completed baseline (2026-10-27)

Completed baseline **`d372d64bccfa6d58efa18835192c1a978abeb0f5`** passed [all six hosted jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40465921098) on Linux/macOS/Windows with stable and exact **Rust 1.88.0**. Six matching evidence artifacts were present; the manual real-Pi job was skipped. Windows logs confirm the new exact-lock unit/subprocess and both acceptance-entrypoint regressions ran on each compiler. The baseline retains both legacy range bounds and covers Pi 1.4.0 metadata, large fragmented summaries, paired recovery edits and native history.

Matching local macOS aarch64 checks used stable **Rust 1.97.1** and exact **1.88.0**:

| Check | Stable | Rust 1.88.0 |
|---|---|---|
| Four-feature check/test matrix | 642 passing executions | 642 passing executions |
| Three-round fake transport stress | 249 passing executions | 249 passing executions |
| Debug/Release boundary regressions | 332 passing executions | 332 passing executions |
| Fresh packaged consumer, four features | PASS | PASS |
| Formatting, strict Clippy and rustdoc | PASS | Covered by stable strict checks |

All **50** Python regressions, Markdown/generated-HTML links, whitespace checks, strict archive inspection and publication dry-run passed. All **103** tracked source hashes stayed unchanged. The inspected baseline archive contained **105 files**, **176738 bytes**, SHA-256 **`2443f9f00ff04627c90f1808df4210d623feb08f856916237d9ddb37e04f7e97`**; it is not the final release archive.

Exact-lock tests reject `1.3.1` versus `1.3.10` in both directions after probing and before RPC startup; matching controls actually launch and close the fake RPC process. Both acceptance entrypoints exercise a seven-version, 49-pair exact-lock matrix. Separate local mutation controls ran 15 checks on Rust 1.88.0: seven expected passes and eight expected assertion failures detected both incorrect prefix-comparison directions. These checks used only the compiled fake, not real Pi or a paid model.

## Pi 1.3.0 development status

For the initial Pi 1.3 implementation on 2026-10-18, the local macOS checks passed stable and exact Rust 1.88.0 four-feature strict verification, both three-round stress suites, Debug/Release boundary regressions, 50 Python regressions, Markdown/generated-HTML and whitespace checks. The complete successful pass preserved all 99 tracked source hashes. Subsequent example-label and manual-CI-pin edits are checked separately and do not change SDK execution.

The 0.2.1 work extends admission and adds synthetic version, tool-argument-error and model-metadata regressions, retaining public fields/features and Rust 1.88. The completed 0.2.1 baseline below has six-job native fake-only coverage, including Windows and exact Rust 1.88.0. Real-Pi acceptance remains limited to the separately recorded Pi 1.1.0/1.2.0 versions; these checks do not establish live Pi 1.3.0 execution on any platform. No real Pi or paid model is invoked by the development checks.

An initial MSRV stress attempt failed `partial_record_at_process_exit_remains_protocol_error`; isolated recheck and a complete three-round repeat passed. The original log omitted the actual terminal value, so that individual failure cannot be conclusively attributed. Subsequent paused-clock comparisons reproduce an existing race in both 0.2.0 and the initial 0.2.1 code: a ready partial-record EOF gives `Protocol` normally but can be replaced by `ProcessExit` after delayed scheduling. The fix performs only bounded, nonwaiting terminal validation, preserving the 50 ms drain, 100 ms EOF grace and existing owner close deadline. Deterministic tests cover known/JSON/EOF errors, no dispatch or UI enqueue at cutoff, continuously ready byte bounds, 2 MiB stack boundaries, and pending/owner/event terminal order with reservation release. First-failure logs remain outside the package; the subprocess assertion now reports the actual terminal error. Expanded version-mismatch cases use distinct fake-executable paths to prevent alias collisions or executable rewrites. Field-preservation regressions include typed entries/tree responses, entry events and state model metadata. Read-budget regressions cover 4095/4096/4097 ready bytes with EOF or an open pipe, LF/CRLF records at the cutoff, and retention without validation of the single lookahead byte. The same terminal/resource checks also cover an exact 4 KiB tail. Current candidate results are tied to the source/CI identity described below.

## 0.2.1 completed baseline (2026-10-18)

Completed baseline **`d27fb2c8de7abe2dedb6aec5aced08d4096eb766`** passed [all six hosted jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/39118567500) on Linux/macOS/Windows with stable and exact **Rust 1.88.0**. Each job produced its matching `release-evidence-*` artifact; the manual real-Pi job was skipped. This baseline includes Pi 1.3.0 support, the bounded 4 KiB EOF probe, deterministic resource-release tests for blocked partial writes/flushes and queued records, and explicit shutdown-count assertions with negative controls that expose violations even after a background-task panic. Acceptance entrypoints reject both range expressions before probing; std-only fake probes verify the default `PI_VERSION`, exact forwarding and mismatch rejection before RPC startup, including model defaults with and without a provider. Three local mutation controls prove that older defaults, range substitution and latest-version substitution fail the expected assertions.

The matching local macOS aarch64 verification used stable **Rust 1.97.1** and exact **1.88.0**:

| Check | Stable | Rust 1.88.0 |
|---|---|---|
| Four-feature check/test matrix | 624 passing executions | 624 passing executions |
| Three-round fake transport stress | 243 passing executions | 243 passing executions |
| Debug/Release boundary regressions | 320 passing executions | 320 passing executions |
| Fresh packaged consumer, four features | PASS | PASS |
| Formatting, strict Clippy and rustdoc | PASS | Covered by stable strict checks |

All **50** Python checker regressions, Markdown/generated-HTML links, whitespace checks, strict packaging and publication **dry-run** passed. All **102** tracked source hashes stayed unchanged during acceptance. The inspected local archive contained **104 files**, SHA-256 **`73cb3ef1455c9ffc4fab349047a7bfd60b1849785349a830254c9c475e15f9fe`**. The independent consumers used that verified source snapshot. This is a completed development baseline, not the final release archive or live Pi 1.3.0 acceptance.

The final release documentation passed its own fresh checks and six-job run, recorded in the [0.2.1 release results](#021-release-2026-10-18). The matching [candidate evidence](#candidate-evidence) was preserved outside the package; the development baseline above was not substituted for those results.

## Local verification (2026-10-16)

The following historical 0.2.0 checks passed at completed baseline **`773ada9388d3183080f8da0036a8ce7245d97eb8`** on **macOS aarch64**, using stable **Rust 1.97.1** and exact **Rust 1.88.0**:

| Check | Stable | Rust 1.88.0 |
|---|---|---|
| Four-feature check/test matrix | 562 passing executions | 562 passing executions |
| Three-round fake transport stress | 243 passing executions | 243 passing executions |
| Debug/Release boundary regressions | 284 passing executions | 284 passing executions |
| Fresh packaged consumer, four features | PASS | PASS |
| Formatting, strict Clippy and rustdoc | PASS | Covered by stable strict checks |

The boundary suites cover stack-protected parsing, conversion, cloning, serialization, release and failure paths; floating-point bit preservation; image/capacity limits; UI replies and shutdown races. Version tests cover both endpoints, synthetic intermediate versions, malformed declarations and exact locks. Probe markers verify rejection before probing or RPC startup as appropriate. The compacted-session restoration regression runs without Pi or a model.

All **50** Python checker regressions, local Markdown links/anchors and whitespace checks passed. Package creation, source/VCS/license/metadata inspection and publication **dry-run** passed from a clean source snapshot. Each independent consumer used fresh dependency resolution, exercised all four features and ran in-memory/native-session examples without Pi. Source bytes were compared against the checked snapshot; no dirty-package bypass was used.

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

The [completed 0.2.2 baseline run](https://github.com/Coder-Sang/pi-rpc/actions/runs/40465921098) passed all six jobs for **`d372d64bccfa6d58efa18835192c1a978abeb0f5`**, including both Windows toolchains. The final release requires its own matching run and artifacts under the [candidate evidence procedure](#candidate-evidence).

The [completed 0.2.1 baseline run](https://github.com/Coder-Sang/pi-rpc/actions/runs/39118567500) passed all six jobs for **`d27fb2c8de7abe2dedb6aec5aced08d4096eb766`**, including Windows stable and exact Rust 1.88.0. The README identifies the final release separately from this historical development baseline. The final uploaded commit **`6a7004d6f845857753bd64cfdbf0762807f1c405`** separately passed its [six release jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/39120347264), including both Windows toolchains; all six matching candidate artifacts were present. Later documentation commits have their own workflow results and do not replace the release source.

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

## 0.2.1 release (2026-10-18)

Published [pi-rpc 0.2.1](https://crates.io/crates/pi-rpc/0.2.1) supports **Pi 1.1.0 through 1.3.0 / Rust 1.88**. The immutable [v0.2.1 tag](https://github.com/Coder-Sang/pi-rpc/tree/v0.2.1) points to the actual uploaded commit **`6a7004d6f845857753bd64cfdbf0762807f1c405`**. This later documentation bookkeeping does not replace that release source. The legacy `>=1.1.0, <=1.2.0` SDK configuration retains its original upper bound; public Rust paths, field types and features are unchanged.

- The [six release CI jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/39120347264) passed on Linux/macOS/Windows with stable and exact **Rust 1.88.0**, each with its matching candidate evidence artifact. The manual real-Pi job was skipped.
- The clean release commit passed complete local verification on macOS aarch64: stable **Rust 1.97.1** and exact **1.88.0** each passed **624** feature-matrix, **243** three-round stress and **320** Debug/Release boundary test executions, plus fresh packaged consumers in all four feature configurations. Formatting, strict Clippy, Rustdoc/generated-HTML, all **50** Python regressions, Markdown links, whitespace checks, strict archive inspection and publication dry-run passed. All **102** tracked source hashes stayed unchanged during acceptance.
- The downloaded registry archive matches the pre-upload inspected archive byte for byte: **104 files**, **169575 bytes**, SHA-256 **`cf4ff040cda7da561a1ca62028fa004b8cbb85c575c1b7123197b78fc3ed4c30`**. Its clean `.cargo_vcs_info.json` identifies the uploaded commit above, and source/metadata/license inspection passed again on the downloaded archive.
- A fresh **Rust 1.88.0** registry consumer resolved exactly `pi-rpc = "=0.2.1"`, verified the registry source and checksum in its new lockfile, and passed public-path tests and in-memory examples in all four feature configurations. Both session configurations passed the native-session example. No path/patch SDK override, Pi process or model was used.
- [docs.rs 0.2.1](https://docs.rs/pi-rpc/0.2.1/pi_rpc/) is available. Its crate index, `SessionFileBuilder`, `PiProcess`, `ProcessOptions`, `PI_VERSION_REQUIREMENT` and `SOURCE_COMMIT` pages were checked, including the supported range and latest reviewed upstream commit. The hosted docs.rs compiler is separate from the exact-MSRV consumer evidence.
- The remote annotated tag peels to the actual uploaded commit. All **eight** distinct README guide/license targets under `v0.2.1` are accessible and match that release source byte for byte.

The release checks did not invoke real Pi or paid models and do not establish live Pi 1.3.0 acceptance. Earlier real-Pi observations retain their separately recorded scope. Complete candidate/publication evidence is retained outside the crate archive; subsequent main/tag workflow results can be inspected in the [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml).

## 0.2.0 release (2026-10-16)

Published [pi-rpc 0.2.0](https://crates.io/crates/pi-rpc/0.2.0) supports **Pi 1.1.0 through 1.2.0 / Rust 1.88**. The immutable [v0.2.0 tag](https://github.com/Coder-Sang/pi-rpc/tree/v0.2.0) points to the actual uploaded commit **`b9a015df8a72052d49f7d422a9dd11ff6651b3d1`**. Later documentation bookkeeping commits do not replace that release source.

- The [six release CI jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/38873074039) passed on Linux/macOS/Windows with stable and exact **1.88.0**. Six candidate artifacts identify that same release commit.
- The clean release commit passed the complete local stable/MSRV feature, stress and Debug/Release boundary checks, 50 Python regressions, Markdown/generated-HTML checks, strict packaging, publication dry-run and fresh packaged consumers. All 99 tracked source hashes stayed unchanged during acceptance.
- The downloaded registry archive matches the pre-upload inspected archive byte for byte: **101 files**, SHA-256 **`41eac29dd93e69e8854aa72cb7a301b01262e7c982436536af34802d83cb5943`**. Its clean `.cargo_vcs_info.json` identifies the uploaded commit above.
- A fresh **Rust 1.88.0** registry consumer resolved exactly `pi-rpc = "=0.2.0"`, checked its registry source/checksum, and passed public-path tests and in-memory examples in all four feature configurations; both session configurations also passed the native-session example. No Pi process or model was invoked.
- [docs.rs 0.2.0](https://docs.rs/pi-rpc/0.2.0/pi_rpc/) is available. Its crate index, `SessionFileBuilder`, `PiProcess`, `ProcessOptions` and `PI_VERSION_REQUIREMENT` pages were checked. All eight distinct README guide/license targets under `v0.2.0` are accessible and match the release source. The hosted docs.rs compiler is separate from the exact-MSRV consumer evidence.

The release checks did not rerun real Pi or paid-model acceptance. Earlier real-Pi observations retain their separately recorded scope.

## 0.1.0 release (2026-10-14)

Published [pi-rpc 0.1.0](https://crates.io/crates/pi-rpc/0.1.0) retains **Pi 1.1.0 / Rust 1.85**. Its immutable [v0.1.0 source](https://github.com/Coder-Sang/pi-rpc/tree/v0.1.0) is commit `41a7348d418ce1e1a591fa66246cc9f21e5d26b8`. The [six hosted jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/38497009320), registry consumer and [docs.rs](https://docs.rs/pi-rpc/0.1.0/pi_rpc/) verification passed for that release. Those historical results do not establish 0.2.0 acceptance.
