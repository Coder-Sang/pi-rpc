# Public release checklist

## Current decision

Release **0.2.1** supports **Pi >=1.1.0, <=1.3.0**, retaining Rust **1.88**, public fields and features. Its latest reviewed source is `71023aec1546d6d677a65669c5665c0927ce0dd0`; live Pi 1.3.0 acceptance is not claimed. Version 0.2.1 was published on 2026-10-18 from **`6a7004d6f845857753bd64cfdbf0762807f1c405`**, after complete local verification and six successful hosted jobs for that commit. The registry bytes, exact Rust 1.88.0 registry consumer, docs.rs, immutable `v0.2.1` tag and frozen guide links are confirmed in the [release record](verification.md#021-release-2026-10-18).

The [completed 0.2.1 development baseline](verification.md#021-completed-baseline-2026-10-18) is commit `d27fb2c8de7abe2dedb6aec5aced08d4096eb766`, with six successful native fake-only jobs and local stable/exact 1.88.0 package/consumer verification. The final release commit passed its own [six-job run](https://github.com/Coder-Sang/pi-rpc/actions/runs/39120347264) and [candidate evidence procedure](verification.md#candidate-evidence); the development baseline was not substituted for those checks. This later documentation bookkeeping records observed results without moving the release tag away from the actual uploaded commit.

**pi-rpc 0.2.0** supports **Pi >=1.1.0, <=1.2.0**, with latest reviewed source commit **06a466935536f8c9bcacb2d5efc0fc021be1bdff**, and requires **Rust 1.88**. Version 0.2.0 was published on 2026-10-16; registry bytes, Rust 1.88.0 registry consumption, docs.rs and the exact uploaded commit tag are confirmed in the [release record](verification.md#020-release-2026-10-16). Cargo continues to permit publishing only to `crates-io`.

The published [0.1.0](https://crates.io/crates/pi-rpc/0.1.0) remains pinned to Pi 1.1.0/Rust 1.85, with the immutable [v0.1.0 tag](https://github.com/Coder-Sang/pi-rpc/tree/v0.1.0) at **41a7348d418ce1e1a591fa66246cc9f21e5d26b8**. Its registry, docs.rs and hosted results are recorded in [verification](verification.md#010-release-2026-10-14); they do not establish 0.2.0 acceptance.

The package declares the repository, nonblank description, README, Apache-2.0 license, keywords and all-feature docs.rs configuration. Original and Cargo-normalized manifests must agree. The checker enforces the current Rust 1.88 baseline, crates.io-only publishing, retained notices/changelog/provenance, source-document equality, archive safety and ignore-policy agreement. `.env.example` remains allowed outside excluded directories. These checks are bounded, not an exhaustive secret scan. See the [source review](provenance.md) and [compatibility notes](compatibility.md#upgrade-from-pi-110-to-120).

## Version and release policy

- **0.2.x** patch releases preserve documented public Rust paths, field types, feature names and the **1.88** minimum. Pi compatibility for 0.2.1 covers the inclusive **1.1.0 through 1.3.0** range; published 0.2.0 retains **1.1.0 through 1.2.0**. Local defaults accept the range; concrete expectations retain exact locks. The previous fixed `>=1.1.0, <=1.2.0` configuration retains its original upper bound. Remote hosts declare their concrete Pi version. Prereleases and build metadata are rejected; range admission alone does not establish review of a future in-range patch release.
- The published **0.1.x** line retains its **Pi 1.1.0 / Rust 1.85** contract. Raising the compiler baseline and expanding the Pi compatibility policy are explicit 0.2.0 migration requirements; the existing Rust public types/features are preserved.
- Source-breaking changes, including variants added to exhaustively matched public enums, require a new minor series with migration notes. Protocol upgrades and higher MSRV require explicit review and documentation.
- A dry-run does not reserve a name/version or prove registry acceptance or docs.rs success. Published version archives cannot be overwritten.
- Finalize package-facing README/changelog/status text before packaging a release commit. After upload confirmation, tag the exact uploaded commit; later bookkeeping commits must not replace it as the tag target.

## 0.2.1 release checklist

- [x] Authorize the actual upload and exact uploaded-commit tag (2026-10-18).
- [x] Finalize registry installation, the 0.2.1 changelog date and immutable `v0.2.1` guide links before packaging.
- [x] Review Pi 1.1.0/1.2.0/1.3.0 source and identical MIT notices; retain SDK legacy configuration, public paths, features and Rust 1.88.
- [x] Commit the release source; pass every local gate, strict package/dry-run/consumer check and all six CI jobs for that same commit.
- [x] Upload the verified commit and compare the downloaded registry archive, checksum and clean VCS identity with the pre-upload evidence.
- [x] Verify an exact Rust 1.88.0 registry consumer in all four feature configurations and verify docs.rs with the optional APIs.
- [x] Create/push `v0.2.1` at the actual uploaded commit, verify frozen guide links, and record completed observations separately.

The uploaded archive captures the pre-upload checklist state. The completed observations above were added after confirmation; this documentation bookkeeping does not change the uploaded archive or the `v0.2.1` target.

## Completed source and content review

The checked items here and in the before/after-upload lists record the published **0.2.0** release. Current 0.2.1 source and license coverage are documented in [provenance](provenance.md).

- [x] Compare fixed Pi 1.1.0/1.2.0 archives, RPC commands/events/UI, native sessions, model schemas and image/retry semantics. Preserve the identical pinned MIT notice in [THIRD_PARTY_NOTICES](../THIRD_PARTY_NOTICES.md).
- [x] Retain Apache-2.0 and crate metadata; update metadata checks, negative regressions, fresh-consumer manifest and CI to Rust 1.88.
- [x] Remove the direct `psm = "=0.1.27"` workaround and resolve `psm 0.1.32` with `ar_archive_writer 0.5.3`; keep stack/depth/float/capacity/close protections.
- [x] Preserve the documented public Rust paths/features and cover version-range and exact-lock fake probes, with opt-in examples using the latest reviewed Pi 1.2.0 baseline.

The [provenance review](provenance.md) describes package contents and license declarations. [Verification](verification.md) distinguishes local checks, real-Pi acceptance and hosted CI.

## Before a separately authorized upload

- [x] Obtain authorization for the actual **0.2.0** upload and release tag (2026-10-16).
- [x] Finalize the README's registry installation example and remove development/unuploaded wording before packaging. Keep version claims neutral until upload confirmation.
- [x] Finalize the **0.2.0** changelog heading and intended release date before packaging. Recheck if the date or any source text changes.
- [x] Commit all package-facing files and run the complete checks below against that clean release commit, then confirm all six hosted jobs. Preserve the candidate JSON and its six-job run link outside the archive, following the [evidence procedure](verification.md#candidate-evidence).

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

Archive selection uses Cargo metadata to resolve the current package version and effective target directory, including `CARGO_TARGET_DIR` and `.cargo/config.toml`. For a per-command `--target-dir`, pass the archive explicitly to the checker and consumer, for example `bash scripts/verify-consumer.sh --archive /absolute/path/to/pi-rpc-0.2.1.crate`.

The consumer checks clean VCS identity, version, metadata, notices, links and every archived source byte before extracting that exact snapshot into a fresh directory. It ignores stale unpacked directories and creates its own lockfile. All four feature configurations compile/run the public-path tests and in-memory example; both session configurations also run the native-session example. No Pi process is executed.

The recorder writes `verification/release-evidence.json` under Cargo's effective target directory. CI uploads a separate `release-evidence-<os>-<toolchain>` artifact after each job's required checks. Confirm all six conclusions for the same commit; the JSON itself reports only archive/source inspection. Preserve the chosen upload platform's record before artifact expiration. The record includes no absolute paths or private runtime payloads. Different Cargo versions/platforms need not produce identical archive digests.

The 0.2.1 README guide links target the immutable `v0.2.1` tag at the actual uploaded commit, following the published 0.2.0 links under `v0.2.0`. All eight distinct guide/license targets were fetched and matched against that release source. For future releases, verify the corresponding source files/anchors before packaging and confirm remote targets after tagging. Both Markdown and generated-HTML checks map `main`, canonical three-part release tags (with optional `v`), and full 40-character lowercase commit IDs to the inspected source snapshot. Other repository `blob` references are rejected; use full commit IDs instead of abbreviated hashes. These offline checks validate files and anchors, not the existence or identity of a remote ref. Confirm the tag links after pushing the exact uploaded commit's tag.

After every local check and all six hosted jobs pass for the authorized clean release commit, record its SHA/digest and upload with `cargo publish --locked --registry crates-io`. Do not use `--allow-dirty` or skip package verification. Compare the actual registry archive checksum and `.cargo_vcs_info.json` with the recorded evidence.

Development requires stable Rust with rustfmt/Clippy, separately installed exact **1.88.0**, Bash, Git, **Python 3.11+** and a native C/assembly compiler. Windows uses Git Bash, MSVC and the Windows SDK. Native stack-switching dependencies still require their platform compiler; installing a Rust target alone does not prove cross-platform execution.

The [Verify workflow](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml) is configured for Linux/macOS/Windows on stable and exact 1.88.0. Consult the [hosted CI record](verification.md#hosted-ci) for completed jobs. [Local verification](verification.md#current-status-summary) and historical release results are recorded separately.

## After a separately authorized upload

- [x] Confirm **0.2.0** in the registry, compare its actual bytes/VCS identity and run a fresh registry consumer on exact Rust 1.88.0.
- [x] Confirm docs.rs builds with all features and optional APIs are visible.
- [x] Record actual results and create/push **v0.2.0** pointing to the exact uploaded commit. A later documentation commit must not be substituted.

Post-upload Git-only README/changelog edits do not replace the uploaded archive. Corrections to published source require another version.

## Opt-in live acceptance

Real Pi and paid-model acceptance require an explicit request and are separate from normal verification. The current local endpoint/UI results and completed local hosted-model checks are recorded in [verification](verification.md#real-pi-acceptance); remote deployment and vendor acceptance remain unverified. Historical results apply only to their recorded versions. Hosts own executable installation, credentials, environment isolation and process policy. Persistent-setting RPCs may modify Pi's selected agent directory; follow the [isolated acceptance guide](acceptance.md#broader-real-pi-acceptance) before a separately authorized run.
