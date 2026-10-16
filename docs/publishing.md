# Public release checklist

## Current decision

Development **pi-rpc 0.2.0** supports **Pi >=1.1.0, <=1.2.0**, with latest reviewed source commit **06a466935536f8c9bcacb2d5efc0fc021be1bdff**, and requires **Rust 1.88**. Version 0.2.0 is unreleased; registry upload and a release tag remain separate release steps. Cargo continues to permit publishing only to `crates-io`.

The published [0.1.0](https://crates.io/crates/pi-rpc/0.1.0) remains pinned to Pi 1.1.0/Rust 1.85, with the immutable [v0.1.0 tag](https://github.com/Coder-Sang/pi-rpc/tree/v0.1.0) at **41a7348d418ce1e1a591fa66246cc9f21e5d26b8**. Its registry, docs.rs and hosted results are recorded in [verification](verification.md#010-release-2026-10-14); they do not establish 0.2.0 acceptance.

The package declares the repository, nonblank description, README, Apache-2.0 license, keywords and all-feature docs.rs configuration. Original and Cargo-normalized manifests must agree. The checker enforces the current Rust 1.88 baseline, crates.io-only publishing, retained notices/changelog/provenance, source-document equality, archive safety and ignore-policy agreement. `.env.example` remains allowed outside excluded directories. These checks are bounded, not an exhaustive secret scan. See the [source review](provenance.md) and [compatibility notes](compatibility.md#upgrade-from-pi-110-to-120).

## Version and release policy

- **0.2.x** patch releases preserve documented public Rust paths, field types, feature names and the **1.88** minimum. Pi compatibility covers the inclusive **1.1.0 through 1.2.0** range. Local defaults accept the range; concrete expectations retain exact locks. Remote hosts declare their concrete Pi version. Prereleases and build metadata are rejected; range admission alone does not establish review of a future in-range patch release.
- The published **0.1.x** line retains its **Pi 1.1.0 / Rust 1.85** contract. Raising the compiler baseline and expanding the Pi compatibility policy are explicit 0.2.0 migration requirements; the existing Rust public types/features are preserved.
- Source-breaking changes, including variants added to exhaustively matched public enums, require a new minor series with migration notes. Protocol upgrades and higher MSRV require explicit review and documentation.
- A dry-run does not reserve a name/version or prove registry acceptance or docs.rs success. Published version archives cannot be overwritten.
- Finalize package-facing README/changelog/status text before packaging a release commit. After upload confirmation, tag the exact uploaded commit; later bookkeeping commits must not replace it as the tag target.

## Completed source and content review

- [x] Compare fixed Pi 1.1.0/1.2.0 archives, RPC commands/events/UI, native sessions, model schemas and image/retry semantics. Preserve the identical pinned MIT notice in [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES.md).
- [x] Retain Apache-2.0 and crate metadata; update metadata checks, negative regressions, fresh-consumer manifest and CI to Rust 1.88.
- [x] Remove the direct `psm = "=0.1.27"` workaround and resolve `psm 0.1.32` with `ar_archive_writer 0.5.3`; keep stack/depth/float/capacity/close protections.
- [x] Preserve the documented public Rust paths/features and cover version-range and exact-lock fake probes, with opt-in examples using the latest reviewed Pi 1.2.0 baseline.

The [provenance review](provenance.md) describes package contents and license declarations. [Verification](verification.md) distinguishes local checks, real-Pi acceptance and hosted CI.

## Before a separately authorized upload

- [ ] Obtain authorization for the actual **0.2.0** upload and release tag.
- [ ] Finalize the README's registry installation example and remove development/unuploaded wording before packaging. Keep version claims neutral until upload confirmation.
- [ ] Finalize the **0.2.0** changelog heading and intended release date before packaging. Recheck if the date or any source text changes.
- [ ] Commit all package-facing files and run the complete checks below against that clean release commit, then confirm all six hosted jobs. Preserve the candidate JSON and its six-job run link outside the archive, following the [evidence procedure](verification.md#candidate-evidence).

## Final verification before upload

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
RUSTUP_TOOLCHAIN=1.88.0 CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
RUSTUP_TOOLCHAIN=1.88.0 CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
python3 scripts/test_checks.py
python3 scripts/check-docs.py .
cargo package --locked --list
cargo package --locked
cargo publish --dry-run --locked --registry crates-io
python3 scripts/check-package.py --source-root . --expected-commit "$(git rev-parse HEAD)"
bash scripts/verify-consumer.sh
RUSTUP_TOOLCHAIN=1.88.0 bash scripts/verify-consumer.sh
python3 scripts/record-evidence.py
git diff --check
```

Use offline mode only after dependencies/toolchains are downloaded. The fresh consumer and publishing rehearsal should have network access to resolve the registry index. `cargo package --locked` explicitly refreshes the standard archive; Cargo can place a publishing rehearsal's archive in a temporary subdirectory instead.

Archive selection uses Cargo metadata to resolve the current package version and effective target directory, including `CARGO_TARGET_DIR` and `.cargo/config.toml`. For a per-command `--target-dir`, pass the archive explicitly to the checker and consumer, for example `bash scripts/verify-consumer.sh --archive /absolute/path/to/pi-rpc-0.2.0.crate`.

The consumer checks clean VCS identity, version, metadata, notices, links and every archived source byte before extracting that exact snapshot into a fresh directory. It ignores stale unpacked directories and creates its own lockfile. All four feature configurations compile/run the public-path tests and in-memory example; both session configurations also run the native-session example. No Pi process is executed.

The recorder writes `verification/release-evidence.json` under Cargo's effective target directory. CI uploads a separate `release-evidence-<os>-<toolchain>` artifact after each job's required checks. Confirm all six conclusions for the same commit; the JSON itself reports only archive/source inspection. Preserve the chosen upload platform's record before artifact expiration. The record includes no absolute paths or private runtime payloads. Different Cargo versions/platforms need not produce identical archive digests.

README guide links currently target repository `main` so they work before a release exists. Before packaging a formal release, decide whether to pin them to its planned version tag, then verify the corresponding source files/anchors. Confirm the tag links after pushing the exact uploaded commit's tag.

After every local check and all six hosted jobs pass for the authorized clean release commit, record its SHA/digest and upload with `cargo publish --locked --registry crates-io`. Do not use `--allow-dirty` or skip package verification. Compare the actual registry archive checksum and `.cargo_vcs_info.json` with the recorded evidence.

Development requires stable Rust with rustfmt/Clippy, separately installed exact **1.88.0**, Bash, Git, **Python 3.11+** and a native C/assembly compiler. Windows uses Git Bash, MSVC and the Windows SDK. Native stack-switching dependencies still require their platform compiler; installing a Rust target alone does not prove cross-platform execution.

The [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) is configured for Linux/macOS/Windows on stable and exact 1.88.0. Consult the [hosted CI record](verification.md#hosted-ci) for completed jobs. [Local verification](verification.md#current-status-summary) and historical release results are recorded separately.

## After a separately authorized upload

- [ ] Confirm **0.2.0** in the registry, compare its actual bytes/VCS identity and run a fresh registry consumer on exact Rust 1.88.0.
- [ ] Confirm docs.rs builds with all features and optional APIs are visible.
- [ ] Record actual results and create/push **v0.2.0** pointing to the exact uploaded commit. A later documentation commit must not be substituted.

Post-upload Git-only README/changelog edits do not replace the uploaded archive. Corrections to published source require another version.

## Opt-in live acceptance

Real Pi and paid-model acceptance require an explicit request and are separate from normal verification. The current local endpoint/UI results and completed local hosted-model checks are recorded in [verification](verification.md#real-pi-acceptance); remote deployment and vendor acceptance remain unverified. Historical results apply only to their recorded versions. Hosts own executable installation, credentials, environment isolation and process policy. Persistent-setting RPCs may modify Pi's selected agent directory; follow the [isolated acceptance guide](acceptance.md#broader-real-pi-acceptance) before a separately authorized run.
