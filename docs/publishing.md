# Public release checklist

## Current decision

pi-rpc **0.1.0** is a **technical-preview release candidate**, pinned to exactly Pi **1.1.0**, with Rust **1.85** as the minimum. The owner selected [Apache-2.0](../LICENSE) on 2026-10-09. Publication preparation is authorized; an actual registry upload is **not**. Cargo now permits publishing only to `crates-io`. No crate version or release tag has been uploaded/pushed by this preparation.

The package declares its public [repository](https://github.com/Coder-Sang/pi-rpc), description, README, license, keywords and all-feature docs.rs configuration. A separate homepage/documentation URL is not declared because no dedicated site exists and docs.rs has not built an uploaded release. The [README](../README.md#installation) distinguishes the currently usable Git dependency from the registry dependency available after upload.

The package checker enforces `publish = ["crates-io"]` in both original and normalized manifests, along with the repository, keywords, edition, Rust 1.85 minimum, README, license and docs.rs configuration. Each manifest must declare a nonblank string `description`, and the two values must agree. Negative regressions reject missing/blank/mistyped/mismatched descriptions and a disabled, unrestricted, missing, mistyped or different-registry publishing setting. It also requires the changelog, retained notices and provenance record, compares those files with the source checkout, and rejects ignore-policy drift. `.env.example` remains allowed at the root and in subdirectories, except inside excluded directories. The checker is bounded, not a general secret scanner.

## Version and release policy

- **0.1.x** patch releases preserve documented public Rust paths, field types, feature names and the **1.85** minimum. Bug fixes and internal refactoring must keep the documented contract.
- Source-breaking changes, including variants added to exhaustively matched public enums, require a new minor series such as **0.2.0** and migration notes in the [changelog](../CHANGELOG.md). A protocol upgrade or higher MSRV must be reviewed explicitly and documented before release.
- Pi compatibility remains **1.1.0**, verified against its fixed source commit; this is not a multi-version SDK. Remote transports still rely on the host's declared Pi version.
- Version **0.1.0** is not reserved by a dry-run. The public crates.io API returned no existing `pi-rpc` crate on 2026-10-14; availability and registry-side authorization remain subject to the eventual upload.
- Finalize package-facing release text on the clean release commit **before** a separately authorized upload, following the checklist below. After the upload is confirmed, create/push **v0.1.0** pointing to that exact uploaded commit and verify registry installation and docs.rs. Updating Git afterwards cannot replace text already inside the uploaded 0.1.0 source archive.

## Completed source and content review

- [x] Project license, repository and discoverability metadata are present; publishing is restricted to crates.io.
- [x] Review project source, protocol fixture shapes, TypeScript harnesses and documentation origins. The pinned Pi MIT notice is retained in [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES.md); scope and the license digest are recorded in [provenance](provenance.md).
- [x] Review dependency license declarations and final source-package contents. Dependencies/Pi sources and binaries are not vendored. Tests and opt-in harnesses are intentionally shipped; ordinary builds do not execute them.
- [x] Review current files and reachable Git history using bounded credential patterns. Zero matches were found; historical personal-path findings are recorded without rewriting history. Existing public commit identity is retained.
- [x] Add the initial [CHANGELOG](../CHANGELOG.md), preview/API policy and post-upload tag plan.
- [x] Maintain four-feature Linux/macOS/Windows CI on stable and exact Rust 1.85.0, fake-only stress rounds, focused Debug/Release regressions, package checks and fresh independent consumers.

## Before a separately authorized upload

This preparation authorizes fixes, tests and source pushes only. The current README and changelog intentionally retain candidate status; no publication date or successful upload is claimed.

- [ ] Obtain separate authorization for the actual crates.io upload.
- [ ] On the final release commit, replace candidate/unpublished wording in the README, put the registry installation example first and retain Git as an alternative. Use neutral version/technical-preview wording rather than claiming that an unperformed upload succeeded.
- [ ] Finalize the **0.1.0** changelog heading and intended release date before packaging. If the intended date changes before upload, update the release commit and recheck it. Record actual upload confirmation separately afterwards.
- [ ] Update this preparation status or make its historical date explicit. Preserve dated verification/provenance records without presenting historical candidate status as the current release status.
- [ ] Commit all package-facing documentation, then run the complete verification, packaging, dry-run and independent-consumer checks below against that clean commit. Record its SHA and archive digest outside the archive itself.

The package must contain the final release text when uploaded. A post-upload README/changelog correction in Git does not rewrite the already published version; source corrections require another crate version.

## Final verification before upload

Run the following from a clean commit. Offline mode is appropriate for the locked SDK checks after dependencies/toolchains are downloaded. The fresh consumer and publishing rehearsal should have network access to resolve the current registry index.

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
RUSTUP_TOOLCHAIN=1.85.0 CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
RUSTUP_TOOLCHAIN=1.85.0 CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
python3 scripts/test_checks.py
python3 scripts/check-docs.py .
cargo package --locked --list
cargo package --locked
cargo publish --dry-run --locked --registry crates-io
python3 scripts/check-package.py --source-root . --expected-commit "$(git rev-parse HEAD)"
bash scripts/verify-consumer.sh
RUSTUP_TOOLCHAIN=1.85.0 bash scripts/verify-consumer.sh
git diff --check
```

`cargo package --locked` explicitly refreshes the standard archive used by the checker; Cargo versions can put the publishing rehearsal's archive in a temporary subdirectory instead. Without an explicit archive argument, archive checking and consumer preparation use Cargo metadata to resolve the package version and effective target directory, including `CARGO_TARGET_DIR` and `.cargo/config.toml`. For a per-command `--target-dir`, pass the archive explicitly to the checker and use `bash scripts/verify-consumer.sh --archive /absolute/path/to/pi-rpc-0.1.0.crate`.

The consumer verifies the archive's current clean VCS identity, version, metadata, notices, links and every archived source file before unpacking that exact snapshot into a new temporary directory. Existing unpacked package directories are ignored. It creates its own lockfile, checks public paths in all four feature configurations, runs the in-memory example four times and the native-session example twice. Neither example executes Pi. Its build output uses the effective target directory's `independent-consumer` subdirectory. The checker already covers archive-local Markdown links, so no default-target extraction path is assumed by the release commands.

`--dry-run` packages and verifies without uploading; it does not establish registry acceptance, reserve a name, or prove future docs.rs success. No bare `cargo publish` is part of this preparation.

Development requires stable Rust with rustfmt/Clippy, separately installed exact **1.85.0**, Bash, Git, **Python 3.11+** and a native C/assembly compiler. Windows uses Git Bash, MSVC and the Windows SDK. Python checker tests invoke Git; document/archive checking otherwise uses standard-library modules. The archive checks do not extract arbitrary member paths or follow archive symlinks.

The latest hosted outcomes are available in [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml). Before these preparation changes, all six jobs passed for commit **a17786d** in [run 38380352820](https://github.com/Coder-Sang/pi-rpc/actions/runs/38380352820). Dated local/final results belong in [verification](verification.md#current-status-summary); historical counts apply only to their recorded commits. Normal verification uses the fake executable and in-memory transports. This preparation does not call real Pi or paid models.

## After a separately authorized upload

- [ ] Confirm `pi-rpc` **0.1.0** appears in the registry and a fresh registry consumer builds on Rust 1.85.0.
- [ ] Confirm docs.rs builds with all features and optional APIs are visible.
- [ ] Record the actual upload confirmation and validation results in a release record, and push **v0.1.0** pointing to the exact uploaded commit. A follow-up bookkeeping commit must not be substituted as the tag target.

Do not defer package-facing README/changelog changes to this stage. If verification fails after upload, correct the package in a new version and describe the outcome; the source archive for the uploaded version cannot be overwritten.

## Opt-in live acceptance

Historical live-Pi/model evidence and the earlier settings-isolation incident remain in [verification](verification.md). Live examples are outside normal release preparation. Hosts own executable installation, credentials, environment isolation and process access policy. Persistent-setting RPCs can modify the selected Pi agent directory; follow the [isolated acceptance guide](acceptance.md#broader-real-pi-acceptance) before any separately authorized live run.
