# Public release checklist

## Current decision

The implementation is suitable for a **technical preview**, subject to the remaining publication and third-party provenance checks below. The owner selected **[Apache-2.0](../LICENSE)** on 2026-10-09; Cargo metadata and the full license text now declare that license for this project's original code and documentation. Successful tests and packaging do not themselves publish a crate or replace third-party license obligations. The package is **0.1.0, unpublished on crates.io, `publish=false`**, pinned to exactly Pi 1.1.0. With user authorization, `main` was pushed to the public [Coder-Sang/pi-rpc repository](https://github.com/Coder-Sang/pi-rpc) on 2026-10-11; that repository is configured as `origin` and declared in Cargo metadata.

Cargo also declares five discoverability keywords and `package.metadata.docs.rs.all-features = true`, so a future docs.rs build includes the optional APIs. This metadata does not upload a crate or create a hosted documentation page. No separate homepage is declared because there is no dedicated project website. `.DS_Store` is ignored at every directory level, preserving local Finder files while excluding them from normal package selection.

The package checker enforces the repository, keywords, edition, Rust 1.85 minimum, README, license, `publish=false` and all-feature docs.rs configuration in both original and normalized manifests. The two versions must agree. The archived `.gitignore` must match the checker's ordered policy and, when a source root is supplied, the source file. Local `.env` variants, Finder/Windows/editor artifacts, Python caches and the existing private credential/configuration filenames are rejected even if accidentally tracked; `.env.example` remains allowed at the root and in subdirectories. An example template inside an otherwise ignored directory remains excluded. Rules are deliberately bounded, and the checker is not a general secret scanner.

See [verification](verification.md) for dated results and [compatibility](compatibility.md) for protocol/source boundaries.

## Before an open-source release

- [x] Owner selected Apache-2.0. Full `LICENSE` added, Cargo `license = "Apache-2.0"` declared, README and compatibility text updated. Dependency and upstream licenses are not replaced by this project's license.
- [ ] Review upstream provenance. The inspected Pi release is MIT licensed, copyright Mario Zechner. Rust source is an independent wire-compatible implementation, not a vendored Pi source archive. Any copied/substantial upstream material still needs the applicable notices. Confirm fixture/prose provenance as part of the license decision, not just package compilation.
- [ ] Confirm which email/name should be public. The earlier ten-commit author/committer rewrite used the configured identity and was verified. Future commits use the repository's Git configuration.
- [ ] Review **Git history**, not only HEAD, for private paths, machine names, email addresses and other sensitive information. Current documentation uses portable paths, but older blobs still contain the original personal absolute paths. The author rewrite intentionally preserved file contents. Further history editing would need an explicit decision and a new backup.
- [ ] Review package contents and credentials with appropriate tooling. The completed local pattern scan found no matching real credentials, but is not an exhaustive secret audit. Never commit test auth/model configuration copies, sessions or private diagnostics.
- [x] Add CI configuration for all four features on macOS/Linux/Windows with stable and exact Rust 1.85.0. All six jobs also run three fake-only stress rounds and focused Debug/Release boundary regressions. The separate macOS/Linux real-release smoke is opt-in and remains skipped during normal CI.
- [x] Obtain passing native Linux/Windows and hosted CI results. All six jobs passed for commit **4d9ed6f** in [Verify run 38160425452](https://github.com/Coder-Sang/pi-rpc/actions/runs/38160425452), including Windows stable and exact Rust 1.85.0. Native Linux ARM64 and local macOS checks supplement the hosted results; see the dated verification record for scope.

## Before a crates.io release

- [ ] Complete the licensing and privacy decisions above.
- [x] Configure the actual public repository URL and discoverability fields. Cargo declares the verified GitHub repository, keywords and all-feature docs.rs metadata. A hosted documentation URL can be added once an actual documentation site exists.
- [ ] Decide package name/version and API stability policy. Crates.io name availability and account ownership have not been checked.
- [ ] Review the exact packaged file list. Current packaging includes README, docs, tests and opt-in TypeScript fixtures. They do not execute automatically during a normal library build. Keep any fixture required by a shipped example available if adding an `include`/`exclude` policy.
- [ ] Deliberately change `publish=false` only when release is authorized. This setting blocks registry publishing, not local `cargo package` verification.
- [ ] Run the commands below on the final clean commit. Only after an explicit release decision should anyone perform a registry upload or push/tag a public release.

```bash
CARGO_NET_OFFLINE=true bash scripts/verify.sh
CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh
CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh
CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
RUSTUP_TOOLCHAIN=1.85.0 CARGO_NET_OFFLINE=true bash scripts/verify-regressions.sh
python3 scripts/test_checks.py
python3 scripts/check-docs.py .
cargo package --locked --offline --list
cargo package --locked --offline
python3 scripts/check-package.py target/package/pi-rpc-0.1.0.crate --source-root . --expected-commit "$(git rev-parse HEAD)"
python3 scripts/check-docs.py target/package/pi-rpc-0.1.0
```

`--offline` requires dependencies/toolchains already present locally. The MSRV script requires explicitly installed **1.85.0** and pins nested `rustc` calls too. Python checks require **3.11+** (CI pins 3.12); checker unit tests also require Git to compare the ignore policy with actual Git behavior. The archive/document checker commands themselves use only the Python standard library. Package checks inspect archive-internal targets; source/manifest/docs comparison and clean VCS matching are separate assertions, not a general secret audit.

## What the current checks establish

- Normal build/test/lint/docs workflow passed in four feature configurations, with **462 test executions** across the feature matrix.
- Exact Rust **1.85.0** passed all four feature configurations' all-targets check and tests, with **462 executions**. Each OS/compiler CI combination now runs **230** focused Debug/Release executions and **213** executions across three stress rounds; local runs passed on both compilers. Coverage includes UI reply admission, floating-point counterexamples/subnormals, isolated 2 MiB stack safety, close races, public paths, protocol negative cases and final admission refunds. Earlier compiler/test counts remain historical evidence in the verification document.
- Source packaging and verification builds passed locally and on all three stable CI runners. The earlier missing licensing/repository metadata is now supplied. Packaging is not permission or proof of registry acceptance; future docs.rs hosting has not been tested.
- Historical opt-in checks supplied real Pi execution evidence for every normal RPC command name, with Opus 4.6 queue/tool/active-abort/manual-compaction evidence. Retry/automatic-compaction used real Pi with synthetic upstream faults/usage and an extension summary, not production provider incidents. The recent cross-platform and metadata checks use only the fake and require no real Pi or paid model.
- Native Linux, macOS and Windows CI execution is verified. Source was pushed publicly with authorization; no crates.io upload, release tag, remote-vendor deployment or global Pi installation was performed.

## Safe acceptance setup

The SDK does not install Pi, manage provider credentials or isolate process environment on behalf of a host. Persistent-setting RPCs can change the selected Pi agent directory. The opt-in model acceptance example does not enforce isolation itself. Use private disposable agent/work/session directories, privately copy only necessary provider configuration, and follow the [real Pi acceptance guide](acceptance.md#broader-real-pi-acceptance). The live example overwrites its dedicated scratch settings; the session example modifies supplied scratch history and overwrites its HTML output. Neither should run against user data that needs preservation.

A prior state-test run inadvertently persisted four settings in the normal agent directory. The previous values were unknown and were not guessed back. The [incident record](verification.md#isolation-incident-and-subsequent-safeguards) remains part of verification, not silently removed for publication. Subsequent runs used isolated settings and observed no further changes to the user's post-incident settings.
