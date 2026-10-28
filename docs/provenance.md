# Release provenance and package review

The **0.2.3 development** source retains the **Pi 1.1.0–1.4.0 / Rust 1.88** review from published 0.2.2. The caller-declared startup method and fake-process regressions are project-authored; no dependencies or upstream source baselines changed. Fixed source archives were compared; [compatibility](compatibility.md#upgrade-from-pi-110-to-120), the [Pi 1.3.0 review](compatibility.md#pi-130-source-review-2026-10-18) and [Pi 1.4.0 review](compatibility.md#pi-140-source-review-2026-10-27) record the successive comparisons. All four reviewed releases have identical Pi MIT license bytes. Published 0.2.1 retains Pi 1.1.0–1.3.0; published 0.2.0 retains Pi 1.1.0–1.2.0. Source review, [candidate verification](verification.md#candidate-evidence) and [real-Pi acceptance](verification.md#real-pi-acceptance) are separate evidence.

## Material inventory

| Material | Origin and retained notice |
|---|---|
| `src/` Rust implementation | Project-authored wire-compatible implementation; Apache-2.0 in [LICENSE](../LICENSE) |
| Rust tests and in-memory/fake examples | Project-authored harnesses and regression cases, with protocol field names and fixture shapes derived from the pinned Pi definitions |
| Three `tests/fixtures/rpc_*.ts` extensions | Project-authored acceptance, synthetic retry/compaction and wait-tool harnesses; they import Pi APIs and are explicitly opt-in |
| Protocol descriptions, examples and compatibility evidence | Derived from the fixed Pi 1.1.0/1.2.0 [source evidence](compatibility.md#required-source-evidence), [Pi 1.3.0 comparison](compatibility.md#pi-130-source-review-2026-10-18) and [Pi 1.4.0 comparison](compatibility.md#pi-140-source-review-2026-10-27); Pi MIT copyright and permission text retained in [third-party notices](../THIRD_PARTY_NOTICES.md) |
| CI workflow | Project-authored workflow with immutable third-party action references; action implementations are fetched by GitHub and are not in the source crate |
| Cargo dependencies | Separately resolved third-party crates, not vendored source; license declarations reviewed below |

The tracked-file inventory and introduction commits were inspected. No Pi source archive, executable, generated session, user auth/settings copy or vendor source directory is included. The TypeScript harnesses are not executed by ordinary builds/tests. The latest reviewed Pi protocol reference is Pi **1.4.0**, fixed to commit **4a080252f06caf63a525d7873c5d94d3c23887e0**, rather than a moving branch. The SHA-256 of that commit's full MIT `LICENSE` is **0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48**; its copyright and permission text are retained verbatim.

## Dependency metadata

The 0.2.0 upgrade's locked all-feature metadata contains **41** non-project packages, all with license declarations; the 0.2.1 through 0.2.3 work retain that dependency set. Build dependencies introduced in 0.2.0 are `ar_archive_writer 0.5.3` (**Apache-2.0 WITH LLVM-exception**) and `object 0.39.1` (**Apache-2.0 OR MIT**); `psm 0.1.32` declares **MIT OR Apache-2.0** and Rust **1.88.0**. These crates remain separately resolved and are not vendored. Their declarations do not change the license of this project's original code.

Individual dependency licenses remain authoritative. This is a source-package review, not a binary redistribution license inventory for downstream applications.

## Package content and bounded privacy review

`scripts/check-package.py` checks original/normalized Cargo metadata, license bytes, retained notices/changelog, source-document equality, archive safety, ignore-policy agreement and clean VCS identity. The package intentionally includes documentation, examples, regression tests and opt-in TypeScript harnesses. It excludes build outputs, local `.env` variants, authentication/settings files, private-key files and editor/platform debris. `.env.example` remains allowed outside excluded directories and is covered by a fixture regression.

Each candidate's [evidence record](verification.md#candidate-evidence) inventories the archived source files and their SHA-256 digests, alongside Cargo's generated `Cargo.toml.orig` and `.cargo_vcs_info.json`. Consumer preparation compares every archived source byte, using `Cargo.toml.orig` for the original manifest, and rejects unexpected archive-only source files. Distributed documentation is checked for personal absolute paths; bounded credential-pattern and filename checks supplement the package review without claiming exhaustive secret detection.

Fresh consumers resolve the current archive via Cargo metadata or an explicit `--archive`, validate version and clean VCS identity, and extract the verified snapshot into a new directory. Each creates its own lockfile and checks all four features on stable or Rust 1.88.0. Registry installation on exact Rust 1.88.0 and docs.rs hosting were confirmed after upload; see the [0.2.2 release record](verification.md#022-release-2026-10-27) and historical [0.2.1](verification.md#021-release-2026-10-18) / [0.2.0](verification.md#020-release-2026-10-16) records. The actual 0.2.2 registry archive matches the pre-upload bytes and clean main commit `d69912fc4821085d300da5ba1fe9f942c76e22ec`; `v0.2.2` remains on that uploaded source rather than this later documentation bookkeeping.

## Pi 1.3.0 development review

Version 0.2.1 extends admission through Pi 1.3.0 with no dependency upgrades. The fixed source comparison and changed-file hashes are recorded in [compatibility](compatibility.md#pi-130-source-review-2026-10-18). [Third-party notices](../THIRD_PARTY_NOTICES.md) retain the notice for these historical sources; its MIT text is also identical to the 1.4.0 baseline below. This is source evidence, not live Pi 1.3.0 acceptance.

## Pi 1.4.0 development review

Version 0.2.2 extends admission through Pi 1.4.0 with no dependency upgrades, public-type changes or MSRV increase. The fixed source comparison and changed-file hashes are recorded in [compatibility](compatibility.md#pi-140-source-review-2026-10-27). [Third-party notices](../THIRD_PARTY_NOTICES.md) identify all four reviewed releases; the MIT text remains unchanged. The new recovery fixture is project-authored synthetic native-v3 data. No Pi source archive, model payload, credential or private configuration is packaged. No live Pi 1.4.0 or paid-model acceptance is claimed. The [confirmed publication observations](verification.md#022-release-2026-10-27) remain separate from source review.
