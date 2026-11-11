# Verification results

## Unpublished message ID branch

The 0.3.3-bolt.1 synchronization inherits official 0.3.2/Pi 1.5.2 checks while independently pinning the extension to base Pi 1.5.0. Synthetic tests reject 1.5.1/1.5.2 before Transport I/O and declared/plain/decorated RPC startup, including exact expectations. An independent mutation control proves that substituting ordinary minor-version admission makes the extension regression fail. Existing message-ID schema, exact-echo, tracking and lifecycle controls remain intact.

The `0.3.3-bolt.1` special build uses `publish=false` and adds the opt-in `bolt_message_id` profile. Its local selection matrix contains **64** combinations; official default features and Rust 1.88 remain unchanged. Stable strict validation, exact 1.88.0 checks, three-round stress and Debug/Release boundaries exercise the new regression target and shared lifecycle. Independent consumption runs the in-memory extension example with only `bolt_message_id`, without subprocess or a local protocol feature table requirement. Package checks retain strict clean VCS and source-byte equality using a temporary snapshot when source is uncommitted. No publishing dry-run is run through the guard.

Local logs, source hashes and candidate evidence are kept under ignored `target/verification/`, excluded from the crate. Every committed special-branch candidate requires its own local gates and six Linux/macOS/Windows CI jobs with matching source/archive artifacts before claiming cross-platform acceptance. Record the actual commit, run and archive identities outside packaged source using the [candidate procedure](#candidate-evidence). Synthetic verification does not establish real fork/provider acceptance. Published release evidence below remains historical. See the [extension guide](bolt-message-id.md) for the fixed reference and tested wire semantics.

## Current status summary

**pi-rpc 0.3.2** supports **Pi >=1.3.0, <=1.5.2 / Rust 1.88** with versioned protocol types. Published 0.2.2 retains its historical API and compatibility range. Latest reviewed source is Pi 1.5.2 commit `b4afdddf44e059c67ccf5097e5317d913521b470`. Admission, source review, synthetic verification and live execution are distinct evidence.

Each release candidate requires local gates and six successful native CI jobs for the same commit before claiming cross-platform acceptance; the actual commit, run, logs and archive identities remain outside packaged source. Registry/docs.rs publication is checked after upload, separately from real-Pi/model acceptance. Local gates use installed stable Rust (recording the actual compiler in evidence) and exact 1.88.0, all four main configurations, the complete protocol/feature matrix (32 combinations for the official release baseline, 64 on this special branch), type-isolation positive/negative controls, pressure/boundary tests and fresh packaged consumers. Stable applies strict Clippy to every combination and explicitly checks formatting of all Rust sources, including macro include! files and standalone fixtures; temporary-crate negative controls cover the Cargo formatting blind spot. Preserve actual results, first failures and before/after hashes outside the archive. Strict candidate packaging uses the clean commit being verified. Earlier local uncommitted checks used byte-identical temporary clean snapshots; those snapshot identities must never be presented as main or as candidates verified by hosted CI.

Verification entrypoints now pass their absolute native Bash path through the verification-only PI_RPC_TEST_BASH selector, avoiding Git Bash/WSL name collisions. Shell-selection, Windows path-conversion and later Pi13-row failure controls are included on both toolchains; MSRV still needs no Clippy or rustfmt component. The Windows observer probe checks cfg selection, and the WSL fixture checks executable selection; neither establishes native Windows runtime acceptance.

Temporary Shell fixtures explicitly preserve LF under Windows text-write translation. Byte-level controls verify that default writes become CRLF while the actual fixtures stay LF; deliberately corrupted scripts fail before execution. These controls run with both entrypoint observers on stable and exact 1.88.0 and remain separate from native Windows CI results.

README caller checks run on both compilers without local feature forwarding. They compile the actual dependency/example blocks and variants constructed from TOML dependency entries: path, version strings, version inline tables, equivalent spacing/quoting/order, dependency subtables and CRLF. Every form retains the positive compile control and rejects the original cfg regression, an invalid example body and incompatible dependency features/version constraints. String-version constraints remain strings in direct match/mismatch checks; an in-memory version override must produce the specific mismatch failure, not an unrelated compiler error. Manifest generation preserves documented dependency options and other dependencies while redirecting the SDK source. Renderer and complete-manifest regressions cover DEL and non-BMP Unicode in keys, values and dependency paths. The example function is referenced without invoking Pi. A second independent caller repeats the compile check from the inspected archive. Rustdoc inclusion is gated inside the library; generated HTML verifies both available and unavailable feature configurations.

The historical documentation baseline `cbb7f6ea3ccb8e329d339b7534c55324d1e8a170` passed [six jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40729102276), with matching artifacts. This historical 0.2.3 development evidence includes Windows/exact 1.88.0 but does not validate the new generic API or Pi 1.5.0. Earlier declared-startup baseline `b86883d05809d2e23575f654be700d72fb95c2d9` passed [its six jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/40656324472). No 0.2.3 was published.

Follow [CONTRIBUTING](../CONTRIBUTING.md) and [release gates](publishing.md#final-verification-before-upload) for new results; a changed candidate requires its own evidence.

The optional `protocol_bench` Release example measures fixed synthetic event parsing/decoding, command encoding and 1,000/2,000/4,000-message builder runs with seven-sample medians. Compare identical features/dependencies on 1.88.0 and 1.99.1; timings are observations, not portable performance thresholds or proof that raising MSRV improves throughput.

## Hosted CI

The [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) runs **Linux/macOS/Windows × stable/exact 1.88.0**, six native jobs. Each runs the main/protocol feature matrices and type-isolation checks, three stress rounds, Debug/Release boundaries, archive checks and a packaged independent consumer. The manual real-Pi smoke is opt-in, is explicitly pinned to Pi 1.5.2 and is skipped on ordinary pushes.

## 0.3.2 candidate scope

Version 0.3.2 admits Pi 1.5.1 and 1.5.2 through the existing Pi15 profile. Exact expectations and the prior default range retain their caps. The complete upstream Git-tree comparison and changed implementation review are recorded in [compatibility](compatibility.md#pi-151-and-152-source-review-2026-11-11). Fake tests cover plain/decorated probes, declarations, feature selection, acceptance entrypoint locks, idle exit, handled-prompt response draining and cleanup output after stdin EOF. Synthetic protocol tests preserve open provider compatibility fields and cache-usage events after settlement; existing protocol/native-v3 regressions cover the unchanged wire schemas. These checks do not establish live Pi 1.5.2, model, provider-thread, image-resize or cache-warming acceptance.

The final clean candidate still requires the full local and six-job CI gates below. Actual source, archive, run and publication observations are retained separately from the immutable release source.

## 0.3.1 candidate scope

Version 0.3.1 adds strict official Pi-Bolt probe-output recognition while retaining the 0.3 public API, features, dependencies, Pi range and Rust minimum. Synthetic tests cover the complete format, underlying-version admission, legacy caps, exact locks, canonical-only declarations, byte limits and typed/automatic startup. The fixed formatter review is documented in [compatibility](compatibility.md#pi-bolt-probe-output-031). Source-format recognition and fake-process tests do not establish real Pi/Bolt or model acceptance.

Development checks used byte-identical temporary snapshots; their identities and local results do not validate the final release commit. The actual clean candidate must pass all local gates and all six native jobs, with corresponding source/archive evidence recorded outside the crate. Publication, registry-consumer, docs.rs and immutable-tag observations are recorded only after confirmation, separately from the source being uploaded.

## 0.3.0 release (2026-11-05)

The historical [pi-rpc 0.3.0](https://crates.io/crates/pi-rpc/0.3.0) release introduced Pi 1.3.0–1.5.0 versioned protocols with Rust 1.88. Its [v0.3.0 tag and release](https://github.com/Coder-Sang/pi-rpc/releases/tag/v0.3.0) identify actual uploaded commit `e8532fd225eb47977982c0026c61e48cbbda7bf3`. Registry archive SHA-256 is `d8d3c1e26ed77aae67ab41cd01b50a4ecddcca4ef5681ac0e6bb4508897d9576`.

The [six release-source jobs](https://github.com/Coder-Sang/pi-rpc/actions/runs/41990554189) passed with corresponding artifacts, alongside full local gates, packaged consumption and exact Rust 1.88.0 registry consumers. [docs.rs 0.3.0](https://docs.rs/pi-rpc/0.3.0/pi_rpc/) exposes all protocol and optional APIs. These confirmed observations remain specific to 0.3.0 and do not establish Pi-Bolt output recognition or validate the 0.3.1 candidate. No real Pi or paid model was invoked for that release.

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
