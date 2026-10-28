# Public release checklist

This is the reusable maintainer procedure. Current Git source is **0.2.3 development**, while **0.2.2** remains published. Publication observations, uploaded commits, checksums, tags and historical acceptance live in [verification](verification.md); source and license scope live in [provenance](provenance.md). A completed historical checklist does not authorize or validate another upload.

## Version and release policy

- The 0.2.x series preserves documented public paths, field types, feature names and Rust **1.88**. Source-breaking changes, including variants added to exhaustively matched enums, require a new minor series and migration notes.
- Current Pi admission covers canonical **1.1.0–1.4.0** releases. Both legacy fixed ranges ending in 1.2.0 and 1.3.0 retain their upper bounds; concrete expectations stay exact. See [compatibility](compatibility.md) for reviewed sources and version-specific behavior. Published 0.1.0 retains Pi 1.1.0/Rust 1.85.
- Obtain release authorization before upload or tag creation. A dry-run neither reserves a version nor proves registry/docs.rs acceptance. Published archives cannot be overwritten.
- Finalize package-facing documents before packaging the release commit. Tag the actual uploaded commit after confirmation. Later bookkeeping must not replace its tag target.

## Prepare a release

1. Select a version and review API, feature, MSRV and Pi compatibility changes. Keep the latest published installation example and Git-only API labels accurate until preparing the actual release source.
2. Finalize Cargo.toml/Cargo.lock, README installation/version text and the CHANGELOG version/date. Use immutable release-tag links for package-facing project guides. Verify their corresponding files and anchors before the tag exists.
3. Review [provenance](provenance.md), LICENSE and retained third-party notices. Confirm metadata, package contents, ignore-policy agreement, `.env.example` preservation and absence of private/generated material.
4. Commit the intended release source on main. Run every gate below against that clean commit, then confirm all six CI jobs and matching artifacts for the same SHA. Any source/document change requires fresh evidence.

## Final verification before upload

Development tools and platform requirements are in [CONTRIBUTING](../CONTRIBUTING.md#development-requirements). Run from the repository root:

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

Use offline mode only after dependencies/toolchains are cached. Fresh consumers and the publishing rehearsal may need network access for registry resolution. These gates use only in-memory transports and the compiled fake; they make no Pi or model requests.

`cargo package --locked` refreshes the standard archive; a publishing rehearsal may use a temporary subdirectory instead. Archive selection resolves the package version and effective target directory through Cargo metadata, honoring `CARGO_TARGET_DIR` and `.cargo/config.toml`. For per-command `--target-dir`, pass `--archive /absolute/path/to/pi-rpc-0.2.3.crate` explicitly to the checker/consumer rather than relying on a stale default archive.

Consumers validate clean VCS identity, metadata, notices, links and every archived source byte, then extract that exact snapshot into a fresh directory and resolve their own lockfiles. They compile/run public-path tests and in-memory examples under all four feature configurations; both session configurations run the native-session example. No Pi process is executed.

## Inspect evidence and upload

The [candidate procedure](verification.md#candidate-evidence) records actual commit, compiler and archive identity outside packaged source. Confirm all six Linux/macOS/Windows × stable/1.88.0 jobs succeeded for that same commit and inspect their corresponding `release-evidence-*` artifacts. Preserve the selected upload platform's evidence before CI artifact expiry; archive digests may differ by Cargo/platform.

After all checks pass and upload is authorized, run `cargo publish --locked --registry crates-io` from the verified clean source. Keep strict package verification enabled. Compare the downloaded registry archive with the recorded pre-upload bytes, checksum and clean `.cargo_vcs_info.json`.

## After upload

1. Confirm the exact registry version and compare actual archive/source identity with the pre-upload evidence.
2. Verify a fresh registry consumer on exact Rust 1.88.0 in all four feature configurations, using no SDK path/patch override.
3. Verify docs.rs builds with all features and exposes the optional APIs.
4. Create and push the version tag at the actual uploaded commit. Fetch every tagged README guide/license target and compare it with the release source.
5. Create the matching GitHub Release from that existing tag, using the CHANGELOG summary and confirmed publication evidence. Record observed results separately; subsequent Git-only edits do not alter the uploaded archive.

Markdown and generated-HTML checks map `main`, canonical release tags and full lowercase 40-character commit IDs to the inspected source snapshot. They validate files/anchors offline, not the existence or identity of remote refs. Confirm those refs and hosted pages separately. Development guides use main links; published archives retain their release snapshots.

## Opt-in real acceptance

Normal release gates do not execute real Pi or paid models. Such acceptance needs a separate explicit request and the isolated environments in [acceptance](acceptance.md). Results apply only to the recorded Pi version, model, platform and date; source review or synthetic transport tests do not extend them. Hosts own executable installation, credentials, isolation and process policy.
