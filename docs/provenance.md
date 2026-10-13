# Release provenance and package review

Reviewed on **2026-10-14** for pi-rpc **0.1.0**, using the repository before publication preparation at commit `a17786d1525899e9b71aaa3fc49f1e1386247ef1` and the final package checks described in the [release checklist](publishing.md). No registry upload, live Pi execution or paid-model request is part of this review.

## Material inventory

| Material | Origin and retained notice |
|---|---|
| `src/` Rust implementation | Project-authored wire-compatible implementation; Apache-2.0 in [LICENSE](../LICENSE) |
| Rust tests and in-memory/fake examples | Project-authored harnesses and regression cases, with protocol field names and fixture shapes derived from the pinned Pi definitions |
| Three `tests/fixtures/rpc_*.ts` extensions | Project-authored acceptance, synthetic retry/compaction and wait-tool harnesses; they import Pi APIs and are explicitly opt-in |
| Protocol descriptions, examples and compatibility evidence | Derived from the fixed Pi 1.1.0 sources listed in [compatibility](compatibility.md#required-source-evidence); Pi MIT copyright and permission text retained in [third-party notices](../THIRD_PARTY_NOTICES.md) |
| CI workflow | Project-authored workflow with immutable third-party action references; action implementations are fetched by GitHub and are not in the source crate |
| Cargo dependencies | Separately resolved third-party crates, not vendored source; license declarations reviewed below |

The tracked-file inventory and introduction commits were inspected. No Pi source archive, executable, generated session, user auth/settings copy or vendor source directory is included. The TypeScript harnesses are not executed by ordinary builds/tests. The Pi protocol reference is fixed to commit **abe508e1b89912adde45528136c3221eb69acdd7**, rather than a moving branch. The SHA-256 of that commit's full MIT `LICENSE` is **0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48**; its copyright and permission text are retained verbatim.

## Dependency metadata

All **39** non-project packages in `cargo metadata --locked` declare a license. Direct/runtime/build and development dependency metadata were reviewed together. Expressions include MIT, MIT OR Apache-2.0, Unlicense OR MIT, alternatives including Apache-2.0 WITH LLVM-exception, and the `unicode-ident` expression `(MIT OR Apache-2.0) AND Unicode-3.0`. `r-efi` declares `MIT OR Apache-2.0 OR LGPL-2.1-or-later`; the latter is an alternative, not a requirement to select it. Individual dependency licenses remain authoritative. This records metadata and source-package provenance, not a binary redistribution license inventory for downstream applications.

## Package content and bounded privacy review

The initial **91** tracked files and **276** reachable history blobs were scanned for PEM private-key headers, GitHub PATs, AWS access-key IDs and long provider-token patterns: **zero matches**. Findings report counts and file names only, never credential contents. Historical documentation contains personal absolute paths in **13** blobs; these are already in the public history and are retained as a recorded finding. No history rewrite is included in this release preparation. Current distributed Markdown rejects personal absolute paths.

The final source crate is reviewed directly with `scripts/check-package.py`, including original/normalized metadata, exact Apache license bytes, retained notices/changelog, source-document equality, archive safety, ignore-policy agreement and clean VCS identity. The package intentionally includes docs, examples, regression tests and opt-in TypeScript harnesses. Build outputs, local `.env` variants, auth/settings files, private-key files and editor/platform debris are excluded; `.env.example` remains allowed. The fixture template regression verifies that exception even though the current checkout ships no `.env.example`.

The package inventory after the consumer selection fix is **98 files**: **96 source files** and Cargo's generated `Cargo.toml.orig`/`.cargo_vcs_info.json`. Source groups are 26 implementation files, 33 test/fixture files, 11 examples, 11 scripts, six documents under `docs/`, one CI workflow and eight root files. Consumer preparation compares every archived source file byte-for-byte, using `Cargo.toml.orig` for the original manifest, and rejects unexpected archive-only source files. The earlier reviewed 97-file archive was approximately **158 KiB compressed**; exact size/digest vary with documentation and VCS metadata. Its nine Markdown documents pass package-local link/anchor checks.

The bounded pattern scan and package checker do not claim exhaustive credential detection. Fresh consumer verification resolves the archive via Cargo metadata or an explicit `--archive`, validates its current version and clean VCS identity, and extracts the verified snapshot into a new temporary directory. It creates its own lockfile and runs all four feature configurations on stable and Rust 1.85.0. This establishes downstream compilation against fresh dependency resolution without depending on the repository lockfile or an old extracted package. Registry installation and docs.rs hosting can be confirmed only after an actual upload.
