# Release provenance and package review

Current checkout: unpublished **0.3.2-bolt.1**, `publish=false`, with the opt-in [message ID extension](bolt-message-id.md). The official Pi13/Pi14/Pi15 contract and Rust 1.88 minimum are preserved. The fork-specific review is pinned to `7193587994d2fee4b62a213c1f14d82acb832d5c`; its MIT license includes the additional 2026 OpenSec copyright, retained and checked separately in THIRD_PARTY_NOTICES.md. The historical official source/release reviews below retain their scope.

The **0.3.1** source supports **Pi 1.3.0–1.5.0 / Rust 1.88** with project-authored version profiles, typed clients and auto dispatch. Fixed source archives were compared; see the [1.5 review](compatibility.md#pi-150-source-review-2026-11-04) and retained historical comparisons. All five reviewed releases, 1.1.0 through 1.5.0, have identical MIT license bytes. Only 1.3.0–1.5.0 is admitted by 0.3. Published 0.2.2 retains its original source/API/range. Source review, [candidate verification](verification.md#candidate-evidence) and [real acceptance](verification.md#real-pi-acceptance) are separate evidence.

## Material inventory

| Material | Origin and retained notice |
|---|---|
| `src/` Rust implementation | Project-authored wire-compatible implementation; Apache-2.0 in [LICENSE](../LICENSE) |
| Rust tests and in-memory/fake examples | Project-authored harnesses and regression cases, with protocol field names and fixture shapes derived from the pinned Pi definitions |
| Three `tests/fixtures/rpc_*.ts` extensions | Project-authored acceptance, synthetic retry/compaction and wait-tool harnesses; they import Pi APIs and are explicitly opt-in |
| Protocol descriptions, examples and compatibility evidence | Derived from the fixed Pi 1.1.0/1.2.0 [source evidence](compatibility.md#required-source-evidence), [Pi 1.3.0 comparison](compatibility.md#pi-130-source-review-2026-10-18) and [Pi 1.4.0 comparison](compatibility.md#pi-140-source-review-2026-10-27); Pi MIT copyright and permission text retained in [third-party notices](../THIRD_PARTY_NOTICES.md) |
| CI workflow | Project-authored workflow with immutable third-party action references; action implementations are fetched by GitHub and are not in the source crate |
| Cargo dependencies | Separately resolved third-party crates, not vendored source; license declarations reviewed below |

The tracked-file inventory and introduction commits were inspected. No Pi source archive, executable, generated session, user auth/settings copy or vendor source directory is included. The TypeScript harnesses are not executed by ordinary builds/tests. The latest reviewed Pi protocol reference is Pi **1.5.0**, fixed to commit **f9a26365568127461c47a07a37dcb42c140a3ac5**, rather than a moving branch. The SHA-256 of that commit's full MIT `LICENSE` is **0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48**; its copyright and permission text are retained verbatim.

Official Pi-Bolt output recognition in 0.3.1 is independently implemented Rust parsing of the fixed formatter described in [compatibility](compatibility.md#pi-bolt-probe-output-031). The reference is Pi-Bolt 1.1.1 commit `0c8faeb30b28671c9aac9ed8bdc62b3dda2dd462`, built on Pi 1.5.0. No Bolt implementation, executable or additional dependency is included. This change retains the project license and notices and adds no fork-specific message tracking. Historical [0.3.0 publication evidence](verification.md#030-release-2026-11-05) remains separate from each new candidate and its later publication observations.

## Dependency metadata

The 0.2.0 upgrade's locked all-feature metadata contains **41** non-project packages, all with license declarations; 0.3 retains that exact dependency set and versions. Build dependencies introduced in 0.2.0 are `ar_archive_writer 0.5.3` (**Apache-2.0 WITH LLVM-exception**) and `object 0.39.1` (**Apache-2.0 OR MIT**); `psm 0.1.32` declares **MIT OR Apache-2.0** and Rust **1.88.0**. These crates remain separately resolved and are not vendored. Their declarations do not change the license of this project's original code.

Individual dependency licenses remain authoritative. This is a source-package review, not a binary redistribution license inventory for downstream applications.

## Package content and bounded privacy review

`scripts/check-package.py` checks original/normalized Cargo metadata, license bytes, retained notices/changelog, source-document equality, archive safety, ignore-policy agreement and clean VCS identity. The package intentionally includes documentation, contributor/security guides, examples, regression tests and opt-in TypeScript harnesses. Cargo excludes repository-only `.github` configuration; the archive checker rejects it independently. Build outputs, local `.env` variants, authentication/settings files, private-key files and editor/platform debris remain excluded by the existing ignore policy. `.env.example` remains allowed outside excluded directories and is covered by a fixture regression. Package-only exclusions do not relax the evidence recorder's output guards.

Each candidate's [evidence record](verification.md#candidate-evidence) inventories the archived source files and their SHA-256 digests, alongside Cargo's generated `Cargo.toml.orig` and `.cargo_vcs_info.json`. Consumer preparation compares every archived source byte, using `Cargo.toml.orig` for the original manifest, and rejects unexpected archive-only source files. Distributed documentation is checked for personal absolute paths; bounded credential-pattern and filename checks supplement the package review without claiming exhaustive secret detection.

Fresh consumers resolve the current archive via Cargo metadata or an explicit `--archive`, validate version and clean VCS identity, and extract the verified snapshot into a new directory. Each creates its own lockfile and checks all four features on stable or Rust 1.88.0. Registry installation on exact Rust 1.88.0 and docs.rs hosting were confirmed after upload; see the [0.2.2 release record](verification.md#022-release-2026-10-27) and historical [0.2.1](verification.md#021-release-2026-10-18) / [0.2.0](verification.md#020-release-2026-10-16) records. The actual 0.2.2 registry archive matches the pre-upload bytes and clean main commit `d69912fc4821085d300da5ba1fe9f942c76e22ec`; `v0.2.2` remains on that uploaded source rather than this later documentation bookkeeping.

## Pi 1.3.0 development review

Version 0.2.1 extends admission through Pi 1.3.0 with no dependency upgrades. The fixed source comparison and changed-file hashes are recorded in [compatibility](compatibility.md#pi-130-source-review-2026-10-18). [Third-party notices](../THIRD_PARTY_NOTICES.md) retain the notice for these historical sources; its MIT text is also identical to the 1.4.0 baseline below. This is source evidence, not live Pi 1.3.0 acceptance.

## Pi 1.4.0 development review

Version 0.2.2 extends admission through Pi 1.4.0 with no dependency upgrades, public-type changes or MSRV increase. The fixed source comparison and changed-file hashes are recorded in [compatibility](compatibility.md#pi-140-source-review-2026-10-27). [Third-party notices](../THIRD_PARTY_NOTICES.md) identify all four reviewed releases; the MIT text remains unchanged. The new recovery fixture is project-authored synthetic native-v3 data. No Pi source archive, model payload, credential or private configuration is packaged. No live Pi 1.4.0 or paid-model acceptance is claimed. The [confirmed publication observations](verification.md#022-release-2026-10-27) remain separate from source review.

## Pi 1.5.0 and 0.3 development review

The [complete source comparison](compatibility.md#pi-150-source-review-2026-11-04) pins Pi 1.5.0, its archive and unchanged license. Public per-minor Rust event types are generated from shared project-authored declarations; no upstream source is copied or vendored. Provider-origin flags, response-model metadata and native-v3 fields remain opaque. Initial local acceptance used byte-identical temporary clean snapshots while the work was uncommitted. Those snapshot identities are not main commits, release tags or hosted CI results. Committed candidates use their actual clean source commit and independently recorded [CI evidence](verification.md#candidate-evidence). No real Pi/model invocation is part of this change.
