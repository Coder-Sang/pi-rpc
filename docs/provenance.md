# Release provenance and package review

The 0.2.0 review covers **Pi 1.1.0–1.2.0 / Rust 1.88**. Fixed upstream source archives were compared; [compatibility](compatibility.md#upgrade-from-pi-110-to-120) records the relevant schemas, changes and hashes. The Pi MIT license is unchanged between those endpoints. Source review and [runtime acceptance](verification.md#real-pi-acceptance) are separate evidence.

## Material inventory

| Material | Origin and retained notice |
|---|---|
| `src/` Rust implementation | Project-authored wire-compatible implementation; Apache-2.0 in [LICENSE](../LICENSE) |
| Rust tests and in-memory/fake examples | Project-authored harnesses and regression cases, with protocol field names and fixture shapes derived from the pinned Pi definitions |
| Three `tests/fixtures/rpc_*.ts` extensions | Project-authored acceptance, synthetic retry/compaction and wait-tool harnesses; they import Pi APIs and are explicitly opt-in |
| Protocol descriptions, examples and compatibility evidence | Derived from the fixed Pi 1.1.0 and 1.2.0 sources listed in [compatibility](compatibility.md#required-source-evidence); Pi MIT copyright and permission text retained in [third-party notices](../THIRD_PARTY_NOTICES.md) |
| CI workflow | Project-authored workflow with immutable third-party action references; action implementations are fetched by GitHub and are not in the source crate |
| Cargo dependencies | Separately resolved third-party crates, not vendored source; license declarations reviewed below |

The tracked-file inventory and introduction commits were inspected. No Pi source archive, executable, generated session, user auth/settings copy or vendor source directory is included. The TypeScript harnesses are not executed by ordinary builds/tests. The latest reviewed Pi protocol reference is fixed to commit **06a466935536f8c9bcacb2d5efc0fc021be1bdff**, rather than a moving branch. The SHA-256 of that commit's full MIT `LICENSE` is **0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48**; its copyright and permission text are retained verbatim.

## Dependency metadata

The 0.2.0 upgrade's locked all-feature metadata contains **41** non-project packages, all with license declarations. New build dependencies are `ar_archive_writer 0.5.3` (**Apache-2.0 WITH LLVM-exception**) and `object 0.39.1` (**Apache-2.0 OR MIT**); `psm 0.1.32` declares **MIT OR Apache-2.0** and Rust **1.88.0**. These crates remain separately resolved and are not vendored. Their declarations do not change the license of this project's original code.

Individual dependency licenses remain authoritative. This is a source-package review, not a binary redistribution license inventory for downstream applications.

## Package content and bounded privacy review

`scripts/check-package.py` checks original/normalized Cargo metadata, license bytes, retained notices/changelog, source-document equality, archive safety, ignore-policy agreement and clean VCS identity. The package intentionally includes documentation, examples, regression tests and opt-in TypeScript harnesses. It excludes build outputs, local `.env` variants, authentication/settings files, private-key files and editor/platform debris. `.env.example` remains allowed outside excluded directories and is covered by a fixture regression.

Each candidate's [evidence record](verification.md#candidate-evidence) inventories the archived source files and their SHA-256 digests, alongside Cargo's generated `Cargo.toml.orig` and `.cargo_vcs_info.json`. Consumer preparation compares every archived source byte, using `Cargo.toml.orig` for the original manifest, and rejects unexpected archive-only source files. Distributed documentation is checked for personal absolute paths; bounded credential-pattern and filename checks supplement the package review without claiming exhaustive secret detection.

Fresh consumers resolve the current archive via Cargo metadata or an explicit `--archive`, validate version and clean VCS identity, and extract the verified snapshot into a new directory. Each creates its own lockfile and checks all four features on stable or Rust 1.88.0. Registry installation and docs.rs hosting for 0.2.0 can be confirmed only after an actual upload; see the [release checklist](publishing.md).
