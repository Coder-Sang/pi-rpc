# Public release checklist

## Current decision

The implementation is suitable for a **technical preview**, subject to the remaining publication and third-party provenance checks below. The owner selected **[Apache-2.0](../LICENSE)** on 2026-10-09; Cargo metadata and the full license text now declare that license for this project's original code and documentation. Successful tests and packaging do not themselves publish a crate or replace third-party license obligations. The package is **0.1.0, unpublished, `publish=false`**, pinned to exactly Pi 1.1.0. No remote repository is configured in the audited local repository.

See [verification](verification.md) for dated results and [compatibility](compatibility.md) for protocol/source boundaries.

## Before an open-source release

- [x] Owner selected Apache-2.0. Full `LICENSE` added, Cargo `license = "Apache-2.0"` declared, README and compatibility text updated. Dependency and upstream licenses are not replaced by this project's license.
- [ ] Review upstream provenance. The inspected Pi release is MIT licensed, copyright Mario Zechner. Rust source is an independent wire-compatible implementation, not a vendored Pi source archive. Any copied/substantial upstream material still needs the applicable notices. Confirm fixture/prose provenance as part of the license decision, not just package compilation.
- [ ] Confirm which email/name should be public. Existing ten commits' author and committer were unified to the configured identity and verified. Future commits use the repository's Git configuration.
- [ ] Review **Git history**, not only HEAD, for private paths, machine names, email addresses and other sensitive information. Current documentation uses portable paths, but older blobs still contain the original personal absolute paths. The author rewrite intentionally preserved file contents. Further history editing would need an explicit decision and a new backup.
- [ ] Review package contents and credentials with appropriate tooling. The completed local pattern scan found no matching real credentials, but is not an exhaustive secret audit. Never commit test auth/model configuration copies, sessions or private diagnostics.
- [x] Add CI configuration for all four features on macOS/Linux/Windows with stable and exact Rust 1.85.0, plus default-off macOS/Linux real-release smoke. Configuration is committed and statically validated; actual hosted runs still need a configured repository and authorization.
- [ ] Obtain passing native Linux/Windows and hosted CI results before broadening platform claims. Exact Rust 1.85.0 four-feature checks/tests now passed locally on macOS aarch64.

## Before a crates.io release

- [ ] Complete the licensing and privacy decisions above.
- [ ] Choose and configure the actual public repository URL, appropriate documentation/homepage metadata and discoverability fields. No guessed owner URL is inserted by this audit.
- [ ] Decide package name/version and API stability policy. Crates.io name availability and account ownership have not been checked.
- [ ] Review the exact packaged file list. Current packaging includes README, docs, tests and opt-in TypeScript fixtures. They do not execute automatically during a normal library build. Keep any fixture required by a shipped example available if adding an `include`/`exclude` policy.
- [ ] Deliberately change `publish=false` only when release is authorized. This setting blocks registry publishing, not local `cargo package` verification.
- [ ] Run the commands below on the final clean commit. Only after an explicit release decision should anyone perform a registry upload or push/tag a public release.

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
python3 scripts/test_checks.py
python3 scripts/check-docs.py .
cargo package --locked --offline --list
cargo package --locked --offline
python3 scripts/check-package.py target/package/pi-rpc-0.1.0.crate --source-root . --expected-commit "$(git rev-parse HEAD)"
python3 scripts/check-docs.py target/package/pi-rpc-0.1.0
```

`--offline` requires dependencies/toolchains already present locally. The MSRV script requires explicitly installed **1.85.0** and pins nested `rustc` calls too. Python checks require **3.11+** (CI pins 3.12). Package checks inspect archive-internal targets; source/manifest/docs comparison and clean VCS matching are separate assertions, not a general secret audit.

## What the current checks establish

- Normal build/test/lint/docs workflow passed in four feature configurations, with 222 test executions, not 222 independent tests.
- Exact Rust 1.85.0 passed all four feature configurations' all-targets check and tests, with 222 executions. Earlier Rust 1.85.1 all-features checks remain historical evidence.
- Local source packaging and its verification build passed. The earlier pre-license package check warned about missing licensing/repository metadata. Apache-2.0 now resolves the licensing metadata warning; public repository/documentation metadata remains to be configured. Packaging is not permission or proof of registry acceptance.
- Every normal RPC command name has real Pi execution evidence. Opus 4.6 supplied real queue/tool/active-abort/manual-compaction evidence. Retry/automatic-compaction evidence uses real Pi with synthetic upstream faults/usage and an extension summary, not production provider incidents.
- No Linux/Windows or remote-vendor deployment was verified. No publication, remote push or global Pi installation was performed.

## Safe acceptance setup

The SDK does not install Pi, manage provider credentials or isolate process environment on behalf of a host. Persistent-setting RPCs can change the selected Pi agent directory. The opt-in model acceptance example does not enforce isolation itself. Use private disposable agent/work/session directories, privately copy only necessary provider configuration, and follow the [README acceptance instructions](../README.md#broader-real-pi-acceptance). The live example overwrites its dedicated scratch settings; the session example modifies supplied scratch history and overwrites its HTML output. Neither should run against user data that needs preservation.

A prior state-test run inadvertently persisted four settings in the normal agent directory. The previous values were unknown and were not guessed back. The [incident record](verification.md#isolation-incident-and-subsequent-safeguards) remains part of verification, not silently removed for publication. Subsequent runs used isolated settings and observed no further changes to the user's post-incident settings.
