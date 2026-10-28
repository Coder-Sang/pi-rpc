# Verification results

## Current status summary

Git **0.2.3 development** supports **Pi >=1.1.0, <=1.4.0 / Rust 1.88**. The published release remains **0.2.2**. `spawn_with_version` is Git-only; the latest reviewed upstream baseline remains Pi 1.4.0, commit `4a080252f06caf63a525d7873c5d94d3c23887e0`. Version admission, source review, synthetic tests and real execution are separate evidence.

The completed feature baseline **`b86883d05809d2e23575f654be700d72fb95c2d9`** passed [all six CI jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40656324472), including Windows stable and exact Rust 1.88.0, with six matching artifacts. Both Windows logs confirm the caller-declared startup regressions executed. The manual real-Pi job was skipped. This is evidence for that commit; subsequent source or documentation changes require their own gates.

Local macOS aarch64 verification of that baseline used stable **Rust 1.97.1** and exact **1.88.0**. Each compiler passed 670 four-feature test executions, 282 three-round stress executions and 356 Debug/Release boundary executions, plus packaged consumers. Strict stable lint/docs, 50 Python regressions, Markdown/generated-HTML links, package/source/license inspection and publication dry-run passed. The 106-file archive and complete logs/hashes are retained outside packaged source. Three snapshot mutation controls detected restored probing, ignored expectations and a bypassed legacy probe. No Pi or paid model was called.

For new results, follow [CONTRIBUTING](../CONTRIBUTING.md) and the [release gates](publishing.md#final-verification-before-upload). Store the candidate's own final SHA/archive/results outside packaged source using the procedure below; do not substitute this baseline or a historical release for a changed candidate.

## Hosted CI

The [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) runs **Linux/macOS/Windows × stable/exact 1.88.0**, six native jobs. Each runs the feature matrix, three stress rounds, Debug/Release boundaries, archive checks and a packaged independent consumer. The manual real-Pi smoke is opt-in, remains explicitly pinned to Pi 1.2.0 and is skipped on ordinary pushes.

## 0.2.2 completed baseline (2026-10-27)

The historical development baseline `d372d64bccfa6d58efa18835192c1a978abeb0f5` passed [six jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40465921098), including Windows/exact 1.88.0. It is separate from the final uploaded 0.2.2 source and archive recorded below.

## 0.2.1 completed baseline (2026-10-18)

The historical development baseline `d27fb2c8de7abe2dedb6aec5aced08d4096eb766` passed [six jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/39118567500), including Windows/exact 1.88.0. It covers bounded EOF validation, deterministic blocked-write resource release, explicit shutdown-count assertions and concrete acceptance-entrypoint tests. Its results do not replace final release acceptance.

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

## Candidate evidence

The current gates also check generated Rustdoc HTML on every feature configuration with stable and exact 1.88.0. Local HTML file/fragment targets and project guide URLs are validated against the generated output and source snapshot; source line ranges and Rustdoc's encoded generic anchors are supported. External sites are not fetched by this check.

Each successful matrix job uploads **`release-evidence-<os>-<toolchain>`**, containing `release-evidence.json`. It records:

- `source.commit` and SHA-256 digests of every archived source file, mapped back to original source names.
- The package version/MSRV and actual compiler version.
- The inspected archive's name, byte count, file count and SHA-256.
- The CI run URL and matrix identity. Local records have `ci: null`.

The recorder checks metadata, licenses, archive safety, clean VCS identity and every archived source byte before writing the record. It reads one archive snapshot for both inspection and hashing. No absolute paths, authentication/configuration payloads or model traces are included. Locally, run `python3 scripts/record-evidence.py` after the [complete release checks](publishing.md#final-verification-before-upload); it honors Cargo's effective target directory and supports `--archive` / `--output` overrides outside packaged source.

A record attests only to its archive/source inspection, not to tests it did not execute. For CI acceptance, confirm **all six jobs succeeded for the same `source.commit`** and inspect their preceding test steps. Keep the relevant JSON and complete run link outside the crate archive, preserving them before CI artifact retention expires. Different platforms/Cargo versions can produce different archive digests; compare the eventual upload with the selected upload platform's archive, not with an assumed universal digest.

The candidate's own commit and digest are recorded after it exists, outside the package. Documentation records a completed baseline and this procedure rather than changing its own SHA after each run. Real-Pi observations above remain separately dated and scoped; these fake-only gates do not rerun or extend them.

## 0.2.2 release (2026-10-27)

Published [pi-rpc 0.2.2](https://crates.io/crates/pi-rpc/0.2.2) supports **Pi 1.1.0 through 1.4.0 / Rust 1.88**. The annotated [v0.2.2 tag](https://github.com/Coder-Sang/pi-rpc/tree/v0.2.2) points to the actual uploaded main commit **`d69912fc4821085d300da5ba1fe9f942c76e22ec`**. Both legacy SDK range expressions retain their original upper bounds, and concrete expectations remain exact locks. Public Rust paths, field types, features and dependency versions are unchanged. This subsequent documentation bookkeeping records observations without replacing the uploaded source.

- The [six release CI jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40468486669) passed on Linux/macOS/Windows with stable and exact **Rust 1.88.0**, with six matching evidence artifacts. Windows logs confirm stable **1.99.1** and exact **1.88.0** executed the prefix-collision unit/subprocess and both acceptance-entrypoint regressions. The manual real-Pi job was skipped.
- The clean release commit passed complete local verification on macOS aarch64: stable **Rust 1.97.1** and exact **1.88.0** each passed **642** feature-matrix, **249** three-round stress and **332** Debug/Release boundary test executions, plus fresh packaged consumers in all four feature configurations. Formatting, strict Clippy, Rustdoc/generated-HTML, all **50** Python regressions, Markdown links, whitespace checks, strict archive inspection and publication dry-run passed. All **103** tracked source hashes stayed unchanged during acceptance.
- The downloaded registry archive matches the pre-upload inspected archive byte for byte: **105 files**, **177445 bytes**, SHA-256 **`17517acf1f8b443ef3b0ef08e9ac0ecd00fe24bff4f1b4072a40bc6efbfbd08c`**. Its clean `.cargo_vcs_info.json` identifies the uploaded commit above. Source/metadata/license inspection passed again on the actual registry archive.
- A fresh **Rust 1.88.0** registry consumer resolved exactly `pi-rpc = "=0.2.2"`, verified its registry source/checksum in a new lockfile, and passed public-path tests and in-memory examples in all four feature configurations. Both session configurations passed the native-session example. No path/patch SDK override, Pi process or model was used.
- [docs.rs 0.2.2](https://docs.rs/pi-rpc/0.2.2/pi_rpc/) is available. Its crate index, `SessionFileBuilder`, `PiProcess`, `ProcessOptions`, `PI_VERSION_REQUIREMENT`, `PI_VERSION` and `SOURCE_COMMIT` pages were checked, including the 1.1.0–1.4.0 range and fixed 1.4.0 source baseline. The hosted docs.rs compiler is separate from the exact-MSRV consumer evidence.
- The remote annotated tag peels to the actual uploaded commit. All **eight** distinct README guide/license targets under `v0.2.2` are accessible and match that release source byte for byte. The uploaded source retains its pre-upload checklist state; completed publication observations were added only after confirmation.

No real Pi or paid model was invoked by the release checks. These results do not establish live Pi 1.4.0 acceptance; earlier real-Pi observations retain their separately recorded scope. Complete candidate/publication evidence is retained outside the crate archive. Later main/tag jobs remain visible in the [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml).

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
