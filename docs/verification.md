# Actual verification results

Current target: **Pi 1.1.0**, upgraded on 2026-10-09. The sections below are dated observations, not cumulative claims that older limitations still apply. Local scratch log names identify maintainer evidence; the logs and private test environments are not distributed with this crate. See the [current status summary](#current-status-summary) and [public-library readiness](#public-library-readiness-and-rust-185-series-verification-2026-10-09). The 2026-10-02 archive applies only to Pi 0.99.2.

## Current status summary

- Four-feature offline check/test/strict-clippy matrix: latest completed baseline PASS, **530 test executions** across four feature configurations. Strict docs and examples also passed; preparation results are recorded below.
- Real Pi 1.1.0: **33/33 command names executed**, with real Opus 4.6 queue/tool/abort/manual-compaction evidence and session/bash/export effects. Not all variants, state transitions or events are covered.
- Retry and automatic compaction: real Pi with **synthetic provider errors/usage and an extension summary**, not production Anthropic fault or automatic Opus-summary evidence.
- Rust **1.85.0**: exact minimum compiler passed the latest completed four-feature all-targets check/test (**530 executions**); earlier checks remain recorded below. Native Linux ARM64 execution supplements the earlier Linux x64 cross-check. Hosted Linux, macOS and Windows jobs also passed on both stable and exact 1.85.0, including Debug/Release boundary regressions and three stress rounds.
- Packaging: the project is licensed under **[Apache-2.0](../LICENSE)**, with retained [Pi MIT notices](../THIRD_PARTY_NOTICES.md). The manifest permits only crates.io publishing and declares the public repository, keywords and all-feature docs.rs metadata. Version **0.1.0** is an unuploaded technical-preview candidate; the [release checklist](publishing.md) and [provenance review](provenance.md) distinguish preparation from upload. All six jobs passed on the preceding preparation commit **06ecc07** in [run 38441521427](https://github.com/Coder-Sang/pi-rpc/actions/runs/38441521427); current hosted results are in [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml).
- Safety incident: early state tests persisted four global settings without recording their previous values. The incident and subsequent isolated checks are preserved [below](#isolation-incident-and-subsequent-safeguards).

## Release preparation gap fixes (2026-10-14)

Consumer preparation now resolves the effective Cargo target directory and package version via metadata, or accepts an explicit `--archive`. It checks the current clean commit and every archived source byte, then extracts the validated in-memory snapshot into a fresh directory. Default/stale extracted directories are not used. Archive inspection also supports metadata-based selection. No SDK lockfile is copied into the consumer.

Both manifests require a nonblank string `description` with matching values. The [pre-upload checklist](publishing.md#before-a-separately-authorized-upload) now puts final README/changelog/status changes and verification before the actual upload. Post-upload work confirms registry/docs.rs and tags the exact uploaded commit; it does not claim that a Git-only correction replaces the uploaded source archive. Candidate wording remains intentional until a separately authorized release.

All **32** Python regressions passed, including custom metadata target directories with stale default data, explicit archives, changed source with unchanged VCS claims, stale/dirty/wrong-version packages, Unicode/space paths, Windows drive/alternate-stream rejection and nonempty-destination protection. Description cases cover both manifests' absence, empty/whitespace strings, wrong types, mismatches and valid matching edits.

Local stable and exact **1.85.0** each passed **530** four-feature test executions, **228** stress executions and **262** focused Debug/Release executions, plus the strict stable formatting/lint/docs/examples gate. Source document checks cover nine Markdown files; checker whitespace and link validation passed. The first selection-fix commit was packaged as a clean **98-file** archive and passed the real four-feature independent consumer (16 tests, four in-memory runs and two native-session runs). Final clean package/dry-run/consumer commands are specified in the [release procedure](publishing.md#final-verification-before-upload); their scratch logs use `pi-rpc-release-gaps-*`.

Production Rust, public interfaces, features, dependencies and `Cargo.lock` are unchanged. No crate upload, release tag, real Pi or paid-model request is part of these fixes. Current six-job hosted outcomes remain available in [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml).

## Crates.io publication preparation (2026-10-14)

Version **0.1.0** now allows only `crates-io`, with matching original/normalized manifest checks and negative regressions. The [changelog](../CHANGELOG.md), [stability policy](publishing.md#version-and-release-policy), [retained MIT notice](../THIRD_PARTY_NOTICES.md) and [source/package review](provenance.md) are shipped. Production Rust files, public paths, Cargo feature definitions and `Cargo.lock` are unchanged.

Local macOS fake-only verification completed successfully:

| Check | Stable | Exact Rust 1.85.0 |
|---|---|---|
| Four-feature matrix | 530 passing test executions, strict lint/docs/examples | 530 passing test executions and all-targets checks |
| Three stress rounds | 228 passing executions | 228 passing executions |
| Focused Debug/Release regressions | 262 passing executions | 262 passing executions |
| Independent packaged consumer | 16 passing public-path tests, four in-memory runs, two native-session runs | Same, with a newly generated consumer lockfile |

All **24** Python checker tests passed, including registry restriction, required release files, retained-license changes, new root-document links and source mismatch regressions. Source/archive document checks cover **nine Markdown documents**. The reviewed `.crate` contains **97 files**, with every one of the **95 source files** byte-compared and only the two expected Cargo-generated metadata files added.

`cargo publish --dry-run --locked --registry crates-io` completed package creation and the verification build and reported **“aborting upload due to dry run”**. The initial pre-commit rehearsal included `--allow-dirty`; the final release procedure repeats it from a clean commit and checks archive VCS identity with `--expected-commit`. The first clean archive check caught the earlier package because this Cargo writes dry-run archives under temporary package subdirectories; the procedure now explicitly refreshes the standard archive with `cargo package --locked`. This does not prove registry-side acceptance or docs.rs hosting.

The independent consumer uses the extracted crate, not the working source or SDK lockfile. It forwards the existing features, compiles all original public paths, and runs packaged in-memory/native examples. Fresh registry resolution passed on both compilers. All six hosted jobs now build/check the package and run this consumer using their selected compiler; final hosted outcomes are available in [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml). The preceding six-job success is [run 38380352820](https://github.com/Coder-Sang/pi-rpc/actions/runs/38380352820).

Local logs use `pi-rpc-release-prep-{verify,msrv,stress,msrv-stress,regressions,msrv-regressions,python,docs,archive,consumer,msrv-consumer,dry-run}.log` under the temporary directory. No registry upload, release tag, history rewrite, real Pi or paid-model request was performed.

## Client/protocol follow-up invariants (2026-10-13)

The client/protocol split preserves the original public paths, features and Pi 1.1.0 wire behavior. The external public-path fixture (`tests/public_paths.rs`) now references all 52 original protocol exports, their existing wire/comparison traits, root aliases and optional subprocess/session-file entry points. Every feature configuration compiles and runs the applicable fixture tests; the focused Debug/Release suite includes them too.

The stalled-writer capacity tests explicitly poll the second command to its pending write wait before checking the third command's rejection. This replaces the scheduler assumption from a single `yield_now`; both current-thread and multi-thread runtimes are covered. Response negative cases (`tests/protocol/response_negative.rs`) check both record decoding and direct `Response::validate` for all 24 commands with constrained success data, including missing data, wrong outer types, required fields, nullable cases, nested data and enum values. The 33 command and response fixture names must be unique and match. Unconstrained success data, opaque model objects, empty-session text and future response commands retain their existing acceptance behavior.

Client state (`src/client/state.rs`) owns private pending, closing and terminal storage. Close marking, final command enqueue/registration, manual/automatic UI admission and response correlation use its internal methods. Counting and encoding remain outside the mutex; final admission and close share the same lock. Terminal errors take precedence over close, while close skips late automatic replies even when preparation failed. Deterministic tests check command and automatic-reply admission after preparation and refund writer slots, bytes and command admission permits. Existing late-dialog, ordered terminal delivery, response mismatch, cancellation, stack and floating-point regressions remain in the verification suite.

Local fake-only verification passed with stable and exact Rust **1.85.0**: **462 executions** per four-feature matrix, **213 executions** per three-round stress run, and **230 executions** per focused Debug/Release run. All **21** Python checker tests passed. Cargo dependencies, lockfile, root exports and production protocol definitions are unchanged. Normal validation makes no live Pi or paid model request. The latest hosted outcomes are available in the [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml); historical counts below apply to their dated versions.

## Image capacity and encoding budgets (2026-10-12)

Local fake-only verification passed after raising default RPC records to **32 MiB**, writer bytes to **64 MiB**, and event wire bytes to **128 MiB**. Native session records now default to 32 MiB; the independent file limit remains 64 MiB. Explicit caller limits remain unchanged. Pi 1.1.0's default prompt-image processing targets 2000×2000 and below **4.5 MiB of Base64 per image**, subject to model overrides and the resize setting; see the [pinned resize source](https://github.com/earendil-works/pi/blob/abe508e1b89912adde45528136c3221eb69acdd7/packages/coding-agent/src/utils/image-resize-core.ts#L31). That processing occurs after RPC input, so raw image requests still need transport and host admission limits.

Stable `verify.sh` and exact **1.85.0** `verify-msrv.sh` each passed **418 executions** across four feature configurations. Three stress rounds passed **210 executions**. Each compiler's focused Debug/Release run passed **206 executions**. The 21 Python checker tests, five Markdown documents and 25 local links passed. The manual encoding measurement is deliberately ignored by normal tests and was run separately in Release. No real Pi or model request was made.

Coverage includes four near-4.5-MiB Base64 images, one 20-MiB Base64 image representing a 15-MiB original, rejected two-image 40-MiB input, exact envelope/escaping/LF budgets, all three image input methods and a successful command after rejection. Image start/end and all three complete-history response shapes are exercised above the former 4-MiB limit, including event budget reuse and exactly one terminal error on overflow. Owned command inputs are released after encoding and UI answers after enqueue. Writer/pending admission, concurrent second-pass encoding, failure/panic/size-change/close rollback, dropped futures and write/flush failures have deterministic reservation assertions. The JSONL reader scans each byte once and preserves its scan cursor across cancelled reads.

Seven-sample local Release medians for single-pass versus reserved two-pass encoding were **6.886/12.491 ms** for one image and **25.577/48.571 ms** for four. Reserved output bytes were **4,714,597/18,858,235**. These numbers describe output reservations, not RSS; no machine-specific timing threshold is enforced. Caller-owned images, expanded parsed values and uncollected command responses remain separate memory responsibilities. The latest hosted outcomes are available in the [Verify workflow runs](https://github.com/Coder-Sang/pi-rpc/actions/workflows/verify.yml); earlier dated runs below apply to their recorded commits.

## Latest hosted verification (2026-10-12)

Commit **4d9ed6f** passed all six jobs in [Verify run 38160425452](https://github.com/Coder-Sang/pi-rpc/actions/runs/38160425452), including stable and exact **1.85.0** on Linux, macOS and Windows. Stable jobs passed **21** Python checker tests, including leading-space/Tab ignore-policy regressions, plus document and actual crate checks. Every job passed its feature matrix, three stress rounds and Debug/Release boundary regressions. The manual real-Pi smoke was skipped; no real Pi or paid model request was made. This dated run is the latest completed evidence before the README reorganization; earlier observations below remain historical.

## Hosted six-job cross-platform acceptance (2026-10-11)

Commit **9567193** passed [Verify run 38137879472](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472), triggered by the authorized push of `main`. All six jobs completed successfully:

| Native runner | Stable | Exact Rust 1.85.0 |
|---|---|---|
| Linux (`ubuntu-latest`) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398662) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398706) |
| macOS (`macos-latest`) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398736) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398707) |
| Windows (`windows-latest`) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398536) | [PASS](https://github.com/Coder-Sang/pi-rpc/actions/runs/38137879472/job/114465398687) |

Every job passed its complete feature matrix, three fake-only stress rounds (including native floating-point cases) and focused Debug/Release regressions. The exact MSRV script checks the compiler is **1.85.0**, rather than accepting a newer 1.85 patch release. Windows therefore has native execution evidence for bounded UI admission/close races, bit-exact floats and subnormals, isolated **2 MiB** stack tests, deep-input rejection/destruction and subprocess termination. Stable jobs also passed strict lint/docs/examples, Python checker tests, document checks and actual crate archive verification.

No CI job failed, so no production fix or test relaxation was needed. The real-Pi smoke job was **skipped**; no real Pi or paid model request was made. This replaces the earlier Windows/hosted-CI limitation below; local results and initial environment failures remain as dated evidence.

## Native cross-platform verification supplement (2026-10-11)

The normal CI matrix now runs all four feature configurations, three fake-only stress rounds and `scripts/verify-regressions.sh` on **Linux/macOS/Windows**, each with **stable and exact Rust 1.85.0**. The focused script runs both Debug and Release, including client admission unit tests, UI replies, close ordering, subprocess lifecycle, protocol conversion, JSON depth, streaming encoding limits, native depth/budgets and floating-point numbers. Each local focused run executes **87 tests per profile, 174 total**. Stress includes native floating-point regressions on both compilers. Native stacker/psm C/assembly code is built by each host's compiler; a Rust cross-check alone is not runtime evidence. CI has a 45-minute per-job deadline and fails on the first failing command; it does not retry failed tests.

Native Ubuntu ARM64 (OrbStack, GCC, stable **1.99.0**) passed `verify.sh`, the Debug/Release focused suite and three stress rounds. Exact **1.85.0** passed the complete feature matrix, Debug/Release focused suite and three stress rounds as well. Both Linux compilers executed **356** tests in their feature matrix, **174** focused tests and **189** stress tests. macOS stable **1.97.1** passed the strict matrix, focused suite and stress; macOS exact **1.85.0** also passed the focused suite. Linux's Python 3.13 checker tests (**11**) and Markdown links (**4 documents, 22 links**) passed. The guest uses its own Linux Cargo/rustup binaries and only shares the dependency registry cache. Build outputs are on the guest's disk; incremental compilation and debug symbols were disabled to conserve space, while Debug assertions and unoptimized execution remain enabled. No crate source, dependencies, public interface or lockfile changed.

Initial environment setup incorrectly shared the whole macOS Cargo home, causing fake-fixture compiler invocations to use host paths, and then exhausted the guest's 3.2 GiB `/tmp` memory disk. Those failures were resolved by separating tool binaries and moving build output to disk. One initial MSRV version-probe assertion also failed without printing the unexpected error. The test now reports that error explicitly; the full matrix, focused suite, three stress rounds and **20 additional probe repetitions** passed afterward. The original transient error could not be identified or reproduced, so these passing runs do not establish its cause. This diagnostic change does not alter timeouts or accept additional errors.

At the start of this supplement the SSH-accessible `Coder-Sang/pi-rpc` repository was empty, and hosted Windows execution was pending the first push. The subsequent hosted acceptance above completes that check. The separate real-Pi smoke remains opt-in and is not invoked by these checks. No real Pi or paid model request was made.

Guest logs are `/tmp/pi-rpc-linux-{verify,msrv,regressions,msrv-regressions,stress,msrv-stress,probe-repeat,python}.log`. Local macOS logs use `/tmp/pi-rpc-platform-mac-*.log`. These logs are maintainer evidence, not shipped files.

## Bounded UI replies and close admission (2026-10-11)

Manual UI replies now serialize a borrowed ID/text record into the bounded writer. The public `UiAnswer::record` API still returns an owned Value and retains its existing behavior. Automatic cancellation uses the same borrowed wire representation. Public interfaces, dependency versions, the lockfile, Rust 1.85 and Pi 1.1.0 are unchanged.

- Manual admission checks terminal/closing state before validation or record construction, releases the lock for bounded encoding, then rechecks state under the enqueue lock. Existing terminal errors take precedence; closing returns Closed. Reply kind and select-choice validation remains unchanged on open connections.
- Wire tests cover confirm true/false, cancel for every dialog, select, empty input, editor text and escaped UTF-8 IDs/text. Exact record budgets exclude LF, and one-byte-short limits reject without submitting any record. Encoded fields match the existing public record API.
- Oversized input/editor answers return Capacity without terminating the connection or sending a rejected reply. A subsequent command and small UI reply both succeed without an acknowledgement for the UI reply.
- A stalled-writer close test supplies a valid **5 MiB** late input reply, checks Closed, drains the previously admitted command and verifies no late UI bytes, successful response processing and exactly one terminal event. Synchronization polls the close future directly; no sleep establishes the race.
- Deterministic unit tests change closing/terminal state during serialization, assert encoding holds no state lock, and verify no writer records/byte permits or pending slots are consumed. A record-construction callback is not invoked after closing or termination. Public tests also cover large/invalid answers after owner drop or EOF, with the original terminal reason delivered exactly once.

An external allocation probe repeated the review reproducer with **100,000,000 bytes** of caller-owned answer text and a **1,024-byte** record limit. The largest allocation inside the reply call fell from **100,000,000 bytes** to **64 bytes** while open; closing/closed calls return Closed with **zero** allocations. These are observed allocation sizes, not machine-dependent unit-test thresholds; the probe is not shipped.

`verify.sh`, exact Rust **1.85.0** `verify-msrv.sh`, three-round `verify-stress.sh` (now including `ui_reply`), Release UI/client/protocol/depth/float regressions, all **11** Python checker tests, document links and whitespace checks passed. The four configurations execute **72/102/61/91 integration tests**, **4/5/4/5 unit tests**, and 3 doctests each: **356 executions** per matrix. Strict formatting, Clippy, rustdoc and injected/native/fake-local examples passed. Linux x64 all-feature/all-target cross-check passed; native Linux/Windows runtime and hosted CI were not run. No live Pi or paid model request was made.

Local logs are `/tmp/pi-rpc-ui-reply-{verify,msrv,stress,release,linux,python}.log`; they are not shipped.

## Exact floating-point roundtrips (2026-10-11)

Enabled the existing `serde_json/float_roundtrip` feature without changing dependency versions, the lockfile, public interfaces or Pi 1.1.0. The new regression failed before the feature was enabled: native `tokensBefore` changed from `1.5801779765215413` to `1.5801779765215411` after encoding and parsing (1 ULP).

- Native tests exercise both signs of that counterexample and `8.971135692166839e-150` across all **11** compaction/usage/cost fields, using builder admission, finish, encode and parse.
- Subnormal tests exercise the smallest value, its neighbor, a middle value and the two values immediately below the normal boundary, with both signs in every typed field.
- All finite roundtrip assertions compare IEEE-754 bits, including signed zero. Existing non-finite rejection tests remain unchanged and pass.
- A JSONL tool-duration test verifies the precision counterexamples, smallest/largest subnormal values and negative zero through wire parsing, typed event decoding, encoding and reparsing. It runs in all four feature configurations.

`verify.sh`, exact Rust **1.85.0** `verify-msrv.sh`, three-round `verify-stress.sh`, Release regressions for encoding/depth/native budgets/numbers, all **11** Python checker tests, document links and whitespace checks passed. The four configurations execute **69/99/58/88 integration tests**, **2/3/2/3 unit tests**, and 3 doctests each: **336 executions** per matrix. Strict formatting, Clippy, rustdoc and injected/native/fake-local examples passed. Linux x64 all-feature/all-target cross-check also passed; native Linux/Windows runtime and hosted CI were not run. No live Pi or paid model request was made.

Local logs are `/tmp/pi-rpc-float-{verify,msrv,stress,release,linux}.log`; they are not shipped.

## Five boundary follow-up fixes (2026-10-11)

Fake-only verification on macOS aarch64, Rust 1.97.1 and exact 1.85.0. Public method arguments, return types, Pi 1.1.0 and dependency versions remain unchanged. No live Pi or paid model request was made.

- The output writer checks JSON depth incrementally, with string/escape state retained across writes. It stops on container 513 and preserves the first depth/byte error even if later writes are attempted. Isolated **2 MiB** thread-stack tests accept 512 and reject 513/50,000 levels for custom `SerializeSeq` and `SerializeStruct` implementations in Debug and Release; serializer counters prove rejection after 513 visits. Escaped strings and existing wire/typed-tree regressions passed.
- `prompt_with`, `steer` and `follow_up` guard image ownership before `Into<String>`. An isolated **2 MiB** test supplies 100,000-level image metadata and a panicking conversion to each method. `catch_unwind` recovers, the connection remains open and the process exits normally. Existing rejection, rollback and unpolled-future tests passed.
- Native export checks entry count and performs bounded serialization before full semantic validation. Per-record budgets exclude LF; remaining file budgets include LF, and bound the next record before allocating it. Header/nonmessage validation scans serialized depth into a sink without a Value copy. Regression cases cover capacity-before-semantics, zero/exact/one-byte-short budgets, remaining-file exhaustion and rejection of admitted reserved-field collisions. If an input violates both budgets and semantics, export may now return Capacity first; this is documented in the README.
- Native message validation includes both enclosing entry/message objects. A 510-array opaque field gives a complete 512-container record and roundtrips; 511 arrays are rejected on append, mutable-file validation and existing-file builder admission. Failed appends neither retain records nor reserve IDs. Header/custom-entry roots continue to accept 511 arrays (512 containers total).
- All **11 typed floating-point fields** across compaction, usage and cost reject NaN and both infinities before export or builder admission. Finite zero, negative zero, positive/negative ordinary values, large values, maximum finite magnitudes and the minimum positive normal value roundtrip.

`CARGO_NET_OFFLINE=true bash scripts/verify.sh`, exact `verify-msrv.sh`, three-round `verify-stress.sh`, the Release encoding/depth/numeric suites, all **11** Python checker tests, explicit-root Markdown checks and Git whitespace checks passed. The four feature configurations run **68/96/57/85 integration tests**, **2/3/2/3 unit tests** and 3 doctests each: **328 executions** per matrix. Strict formatting, all-targets check, Clippy, rustdoc and injected/native/fake-local examples passed. Stress now includes `encoding_limits` along with client, subprocess, close/UI and JSON depth suites.

The original isolated generated-sequence and conversion-panic reproductions now exit normally with a Protocol depth error and a recovered panic respectively. For a caller-created **100,000,000-byte** header string with a **32-byte** record limit, the macOS peak resident size was **101.7 MB**, versus **101.4 MB** for merely holding the input. Before the fix, the same review case reached **201.7 MB**. These are process measurements, not machine-dependent unit-test thresholds.

After compiler/test workloads finished, seven-sample Release mixed-history parse medians were **1.43/2.84/5.67 ms** for 1,000/2,000/4,000 entries. Three-sample message-builder medians were **5.18/9.83/19.93 ms** for the same message counts; the compatible mutable append API took **917/3,593/14,160 ms**. Linux x64 all-feature/all-target cross-check passed using clang/ar. Native Linux/Windows runtime and hosted CI were not run; the existing Windows SDK limitation still applies. No dependency or native build configuration changed.

Local evidence is in `/tmp/pi-rpc-five-fixes-*.log` and is not shipped. Older results below predate these boundary fixes.

## Four follow-up review fixes (2026-10-11)

Fake-only verification on macOS aarch64 with Rust 1.97.1 and exact 1.85.0. No live Pi, model requests, protocol upgrade or new dependencies were used.

- Owned command/message/session admission now uses private iterative cleanup guards. Isolated child tests with a **2 MiB** thread stack reject **100,000-level** caller-created JSON without aborting, including rejected messages, invalid-ID/parent rollback, existing-file admission and unpolled command/typed/image convenience futures. Both Debug and Release depth suites passed. Calls retain their existing arguments and `.await` usage; guards are constructed before returning futures so cancellation before the first poll is safe.
- Output serialization stops at the first write exceeding the record-byte budget, rather than allocating the complete output first. Commands serialize directly with their request ID, avoiding a full temporary Value copy. Tests cover early stopping of a 500,000-element generated sequence with a 32-byte limit, exact boundaries excluding LF, escaped strings and serializer errors. Admission is checked before serialization and checked again atomically before enqueue.
- Mixed history validation constructs one forest index and uses DFS intervals for ancestor membership. Exhaustive small-forest comparisons against parent walks, existing branch/compaction/context-edit cases and a **10,000-entry** mixed-history roundtrip passed. Public `branch` behavior is unchanged.
- Native flattened metadata cannot shadow reserved header/entry/message/content/usage fields. Tests cover all header keys, entry envelope and variant keys, role collisions with rollback, nested content/replacements, usage/cost and preserved nonreserved metadata.

`verify.sh`, exact `verify-msrv.sh`, three-round `verify-stress.sh`, all 11 Python checker tests, explicit-root document checks and Git whitespace checks passed. The four feature configurations execute **65/85/54/74 integration tests**, **2/3/2/3 unit tests**, and 3 doctests each: **300 executions** per matrix. The strict matrix also checks formatting, all targets, Clippy, rustdoc and injected/native/fake-local examples. One initial strict run exposed a non-session dead-code warning after direct command encoding; the conversion helper was feature-gated and the final matrix rerun passed.

Linux x64 all-feature/all-target cross-check passed using clang/ar. Native Linux/Windows runtime and hosted CI were not run; the existing Windows SDK limitation recorded below still applies. No dependencies or native build configuration changed.

Release medians for mixed-history parsing from **seven samples**, measured again after other compiler/test workloads finished:

| Entries | Parse median |
|---|---|
| 1,000 | 1.94 ms |
| 2,000 | 3.94 ms |
| 4,000 | 8.00 ms |

Run `cargo run --release --no-default-features --features session_file --example session_validation_bench` to repeat. Final message-builder medians from three samples were **7.70/9.42/19.35 ms** at 1,000/2,000/4,000 messages; these were measured alongside other verification workloads. The unchanged mutable append API took **824/3,428/12,809 ms**. Timings are informational and have no machine-specific unit-test thresholds.

Local logs are `/tmp/pi-rpc-four-fixes-{verify,msrv,stress,release-stack,linux,mixed-bench,builder-bench}.log`; they are not shipped. The older measurements and real-Pi observations below are historical evidence.

## Bounded JSON, close/UI and bulk construction fixes (2026-10-11)

Fake-only verification on macOS aarch64, Rust 1.97.1 and exact 1.85.0. No live Pi or paid model requests were made for these changes; the real-Pi observations below predate these fixes.

- `CARGO_NET_OFFLINE=true bash scripts/verify.sh`: exit 0; four feature configurations passed all-targets check, tests, strict Clippy, strict rustdoc and formatting, plus injected/native/fake-local examples. Integration counts were **61/73/50/62**, with **2 unit tests and 3 doctests** in each configuration, totaling **266 test executions**.
- `CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh`: exit 0 on exact **1.85.0**, all four configurations and the same test counts. A first dependency resolution selected `ar_archive_writer` with unsupported let-chain syntax despite incomplete MSRV metadata. Constraining `psm` to 0.1.27 removed that dependency and preserved compilation on 1.85.0.
- `CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh`: exit 0, all three rounds; stress now includes the new JSON depth and close/UI suites alongside client/subprocess tests.
- `python3 scripts/test_checks.py`: all **11** tests passed; `python3 scripts/check-docs.py .` and Git whitespace checks passed. The subprocess suites required execution outside the local sandbox because fake-child process-state inspection is blocked inside it.
- Linux x64: `CC_x86_64_unknown_linux_gnu=clang AR_x86_64_unknown_linux_gnu=ar cargo check --offline --locked --all-features --all-targets --target x86_64-unknown-linux-gnu` passed. This is compile evidence, not linking or runtime evidence. The equivalent Windows MSVC check stopped in `stacker`'s native build because this Mac lacks Windows SDK `windows.h`; native Windows execution remains for the existing hosted CI matrix. No hosted CI run is claimed.

New regressions cover real client calls with **70/100/200-node-deep session trees**, accepted **512** and rejected **513** JSON container levels, fragmented unknown events, CRLF/escaping, malformed/trailing JSON, direct Value entrypoints and native sessions. An isolated child process checks a **2 MiB** thread stack for deep object/array clones, typed tree conversion, serialization, normal destruction and partial-conversion error cleanup. Direct rejection of a caller-built 10,000-level Value uses iterative destruction.

Close/UI tests deterministically establish closing state before late dialogs arrive, both while an admitted write is stalled and after stdin EOF. They verify successful pending responses, ordered UI/ordinary events, no late reply behind shutdown, exactly one terminal error and then None. Open-connection auto-cancel capacity errors still fail the connection.

The builder preserves the existing mutable SessionFile API and moves indexed bulk message admission into privately owned history. Tests cover invalid headers/history/messages, duplicate IDs, missing/self parents, failure rollback, existing native checkpoints/header metadata, exact equality/encoded bytes against the old API and a 10,000-message roundtrip.

Release timing command: `cargo run --offline --locked --release --no-default-features --features session_file --example session_builder_bench`. Final medians from **three samples** per count/path, after the compiler workload finished:

| Messages | Existing append_message | SessionFileBuilder including finish |
|---|---|---|
| 1,000 | 741.3 ms | 4.56 ms |
| 2,000 | 3,401.8 ms | 8.92 ms |
| 4,000 | 11,731.0 ms | 18.35 ms |

Builder time roughly doubled as message count doubled, versus repeated full-history validation in the original mutable API. This measures pure message construction, not arbitrary mixed-history reference validation or file encoding. Timing is informational and has no machine-specific pass/fail threshold. Local scratch logs: `pi-rpc-final-verify.log`, `pi-rpc-final-msrv.log`, `pi-rpc-final-stress.log`, `pi-rpc-builder-bench-final.log` and the two platform cross-check logs.

## Current release verification: Pi 1.1.0 (2026-10-09)

Verified on macOS aarch64 with rustc/cargo 1.97.1. The target is the official `v1.1.0` release commit `abe508e1b89912adde45528136c3221eb69acdd7`, resolved via GitHub's latest stable release API and tag ref. Fixed source archives were compared to the former 0.99.2 commit, with current hashes in [compatibility.md](compatibility.md).

### Actual complete matrix

`CARGO_NET_OFFLINE=true bash scripts/verify.sh` returned **exit 0**, both before and after final response-shape tightening. The final combined matrix and real-release command completed at 2026-10-09 03:07 UTC, exit 0. Full local scratch logs: `pi-upgrade/final-verification.log` and `pi-upgrade/final-release.log`.

| Features | check --all-targets | Integration tests | Unit / doctests | clippy --all-targets -D warnings |
|---|---|---|---|---|
| default | PASS | 53 passed | 2 / 3 passed | PASS |
| --all-features | PASS | 59 passed | 2 / 3 passed | PASS |
| --no-default-features | PASS | 42 passed | 2 / 3 passed | PASS |
| --no-default-features --features session_file | PASS | 48 passed | 2 / 3 passed | PASS |

**222 passing test executions across four configurations**, not 222 distinct tests. No failures or lint warnings. Crate/fake-fixture formatting, injected/native/fake-local examples and strict all-features/no-default rustdoc all passed. The added release_smoke example compiled in both subprocess-enabled all-targets checks.

### Upgrade-specific coverage and observed improvement

- Required boolean `agent_settled.aborted`: false/true round trips, missing/null/wrong types fail. Client tests observe both settlement values without inventing per-prompt correlation.
- Optional numeric tool `durationMs`: omitted/zero/fractional values, no null/wrong types, no accidental swallowing into extension JSON. Client receives the typed timing field, and malformed values terminate explicitly with Protocol.
- Native assistant/tool-result `durationMs`: preserves fields through RPC/native entries and complete SessionFile encode/parse, rejects malformed numeric fields but does not constrain unknown-role or nested open payloads.
- Former Pi 0.99.2 local executable is rejected by the real fake-process version probe; current 1.1.0 fake is accepted. Existing declared-version and configuration rejection checks remain passing.
- Real empty-session `get_last_assistant_text` initially failed the new release smoke with `Protocol("malformed known record: response")`. Isolated raw RPC observation showed `data:{}`. After accepting omitted text as None, the exact same real-release smoke passed. Protocol/client regression accepts omitted/null/string text, rejects wrong text types, missing data and non-object data. Adding array/null/scalar boundaries first reproduced Serde's permissive array-to-struct decoding, then passed after the explicit response object guard. Required tree/native nullable fields stay strict.
- Existing bounded queues, transport errors, process-exit/EOF races and 3000/1000-iteration terminal delivery regressions still pass in the complete matrix.

### Real official release acceptance, no model request

Downloaded only to scratch, not installed globally: official `pi-darwin-arm64.tar.gz`, SHA-256 **3455b13de35c15a5893cdebc922678199e90a7ce99b06cbc23f37860e90d63c7**, matched the release API digest. Executed:

```bash
CARGO_NET_OFFLINE=true bash scripts/verify-release.sh /absolute/path/to/release/pi/pi
```

**Actual output:** `Pi 1.1.0: 10 read-only RPCs passed; clean exit and terminal delivery verified`.

Through the public PiProcess/PiClient API, the executable's exact --version probe succeeded, then get_state, get_messages, get_available_models, get_available_thinking_levels, get_session_stats, get_commands, get_entries, get_tree, get_last_assistant_text and get_fork_messages all decoded. The fresh session had streaming=false, pending=0, no messages/assistant text. Close returned ProcessExit(Some(0)); the event consumer received the same terminal exactly once then None.

The script uses env -i, fresh temporary HOME/PI_CODING_AGENT_DIR/cwd/TMPDIR, disables session persistence/tools/extensions/MCP/skills/templates/themes, and submits no prompt/bash/model request. No existing credentials/global settings were read or modified. It pins Cargo build and runner output to the same explicit target directory. For npm shebang compatibility it exposes only the resolved Node executable via a temporary symlink, not the host PATH. Independent review identified both target-dir drift and missing non-system Node before this correction. After the fix, direct binary and temporary `/usr/bin/env node` wrapper smoke runs passed with CARGO_TARGET_DIR set to an alternate scratch path; that alternate path was not created. The wrapper test is synthetic Node-launch coverage, not acceptance of an installed npm package.

Scope of this read-only run: no paid prompt, real tool execution or active model abort. Those were exercised later in the broader RPC acceptance below. The historical independent external harness was not rerun during the upgrade; its in-crate stress coverage was rerun. Cross-platform and remote-vendor boundaries remain; the later public-readiness section records Rust 1.85.1 checks. Licensing was undecided at the time of this read-only observation; Apache-2.0 was selected later. `publish=false` remains set, no push/publication.

## Real model acceptance: Claude Opus 4.6 (2026-10-09)

After the upgrade, the user explicitly requested a live model request. The installed `/opt/homebrew/bin/pi` reported **1.1.0**, and its public `get_available_models` RPC listed the configured `anthropic/claude-opus-4-6`. The requested spelling `claude-opus-4.6` is not Pi's model ID; `claude-opus-4-6` is the matching Opus 4.6, not a substituted model.

Actual command (fresh empty scratch cwd, existing Pi-managed authentication):

```bash
cargo run --offline --example model_smoke -- /opt/homebrew/bin/pi /absolute/path/to/empty-work anthropic
```

**Exit 0**, completed at 2026-10-09 03:20:25 UTC. Exactly one fixed prompt was submitted, requesting `PI_RPC_OPUS_OK`. No tool execution, automatic retry, extensions/MCP/skills/templates/themes or persistent session were enabled. Discovery used only public RPC model IDs/providers, no credential content or diagnostics were printed. Pi used its existing provider configuration/authentication. The harness itself did not read/write those files, but its `set_auto_retry(false)` and `set_auto_compaction(false)` RPCs cause Pi to persist settings. It is therefore incorrect to infer that this run left existing settings untouched. Future runs should use a private agent directory as documented in README.

Actual observed output:

```text
Available claude-opus-4-6 providers: anthropic
Prompt accepted: started
Session tokens=936 reported_cost=0.00562
Reply=PI_RPC_OPUS_OK; updates=9; durationMs=6033; agent_settled.aborted=false; clean exit and terminal delivery PASS
```

The cost is Pi's reported estimate, not an independently observed invoice. Public `set_model` and `get_state` confirmed model/provider, `prompt` returned Started, events delivered agent_start, nine message updates including text deltas, assistant message_end (model=claude-opus-4-6, stopReason=stop, durationMs=6033), agent_end without retry and agent_settled with aborted=false. Reassembled text and get_last_assistant_text both matched the token. get_messages included user/assistant, final state was not streaming with pending=0. Close reported ProcessExit(Some(0)), and the event consumer saw the same terminal error exactly once followed by None.

`examples/model_smoke.rs` is a reproducible opt-in entry point with a 180-second deadline and forced cleanup on error/timeout. With no provider argument it only lists the requested model's available providers. It is compiled by the normal feature matrix, but never run as a paid request by verify.sh. No second paid request was made for local regression validation. After the example and its README/verification documentation were finalized, `CARGO_NET_OFFLINE=true bash scripts/verify.sh` returned exit 0 at 03:22:06 UTC: all four check/test/clippy configurations, formatting, examples and strict docs passed (222 test executions, zero failures). This compiles the exact paid-tested example through public crate interfaces but does not issue another model request.

This minimal run exercised real model generation, assistant timing and non-aborted settlement, beyond the earlier read-only release smoke. It did not itself exercise real tools, active abort or retries; later sections add that evidence and the Rust 1.85.1 checks. Cross-platform and remote-vendor deployment remain unverified.

## Broader real RPC acceptance (2026-10-09)

Executed the installed **Pi 1.1.0** on macOS aarch64 through the public SDK, following the user's request to test the remaining RPCs. Across the examples below **33/33 command names now have real-Pi execution evidence**. This is not 33 fully exercised feature state machines or all 24 events observed from Anthropic. Retry and automatic compaction use a real Pi process with synthetic provider fault/usage injection, explicitly separated from hosted-model evidence.

### Per-command acceptance map

| RPC commands | Actual effect and acceptance source |
|---|---|
| `get_state`, `get_messages`, `get_available_models`, `get_available_thinking_levels`, `get_session_stats`, `get_commands`, `get_entries`, `get_tree`, `get_last_assistant_text`, `get_fork_messages` (10) | Existing release smoke plus new state/session/live tests. Empty and populated histories, selected model identity, nonempty native trees, every-entry exclusive `since` suffix, user fork text and final assistant marker. Extension test confirms discovered explicit slash command. |
| `set_model`, `cycle_model`, `set_thinking_level`, `cycle_thinking_level`, `set_steering_mode`, `set_follow_up_mode` (6) | `rpc_state_smoke`: actual non-null model cycle followed by matching state, restore Opus, thinking cycle followed by matching state, both queue-mode values followed by matching state. Multi-input batching under each mode is not separately tested. |
| `set_auto_compaction`, `set_auto_retry`, `abort_retry` (3) | State toggles and independent extension workflow. Retry enabled: synthetic 429 then 503 then success, attempt 1/2 start and successful end. Active abort: call after real auto_retry_start while streaming, cancelled end, no second provider call, settlement before the original delay expires. Disabled: one provider error, no retry events. `get_state` omits autoRetryEnabled, so its behavior is checked rather than inventing a state field. Auto compaction threshold: synthetic usage plus extension summary triggers actual start/end and native compaction entry. |
| `bash`, `abort_bash` (2) | Real shell `printf` output/exit 0 and streaming update. `printf START; sleep 30`, then abort only after START is observed: bash result `cancelled=true` and preserved output. Only fixed scratch shell commands are used. |
| `prompt`, `steer`, `follow_up`, `clear_queue`, `abort` (5) | Real **anthropic/claude-opus-4-6** with one explicitly loaded local gate tool. Prompt Started; streaming prompt followUp Queued; queued steer/follow_up acknowledged. Clear returns exact one steering/two follow-up inputs, none enters history. Replacement steering/follow-up enters actual history, final response PI_FOLLOWUP_OK, pending=0. New tool invocation interrupted after a tool update: real agent_settled.aborted=true and idle state. Slash extension input separately returns Handled with no model inference. |
| `compact` (1) | Default **Opus 4.6** summarizer, no summary hook loaded. Nonempty summary, positive tokensBefore, persisted native compaction entry matching the result, idle state. Final isolated session test also requires successful compaction_end before clean shutdown. |
| `new_session`, `switch_session`, `fork`, `clone`, `set_session_name`, `export_html` (6) | `rpc_session_smoke`: new empty identity, clone different identity with identical projected history, fork native user entry before the selected input with matching returned text, switch restores original identity/name/history, name visible in state, real HTML file >1 KB containing `<html`. Tested both a real error conversation and a populated Opus conversation after compaction. Actual exported file sizes: 279358, 293094 and final 292346 bytes. |

### Real model observations and failures, not hidden

- Two early live tool requests returned provider **Request timed out**, assistant usage/cost zero. A minimal no-tool Opus probe then passed with **929 tokens, estimated $0.005405**. Subsequent real tool workflows passed.
- `live-run-4`: tool durationMs=23, queue consumption/removal and active abort all PASS, **7112 tokens / estimated $0.04414**. Manual compact initially rejected `Nothing to compact (session too small)`. Project-local settings were insufficient for this run. Moving keepRecentTokens=0 to the **dedicated scratch agent** allowed default Opus compaction, tokensBefore=2157, estimated summary cost **$0.048535**. The first session harness then hit an incorrect post-compaction user-message assertion. Raw user entries remain available for fork, but projected get_messages can contain only the compaction summary. Corrected the assertion and reran all session operations successfully without another model call.
- `live-run-5`: tool durationMs=22, queue consumption/removal and active abort all PASS, **7064 tokens / estimated $0.04168**. Its inline compact call hit the 180-second command deadline. It is **not recorded as an exit-0 combined live smoke**. The uncertain request was not blindly repeated on a live process: the process was explicitly terminated/reaped, and no compaction entry existed. Reopened that saved scratch session for the final standalone default Opus compact/session workflow: **exit 0**, tokensBefore=2055, summary 1544 UTF-8 bytes, estimated summary cost **$0.03787**, matching persisted entry and successful compaction_end. All subsequent session operations and exact single-terminal clean shutdown passed.
- Sum of the successful reported generation/summary costs in this round is approximately **$0.17763**. This is Pi's estimate, not an invoice or a guarantee that timed-out upstream requests incurred no charge.

### Real extension UI and controlled failure injection

`rpc_extension_smoke` loads explicit checked-in fixtures with `--offline --no-session --no-tools --no-context-files`, no hosted API requests. All six scenarios ran from fresh scratch directories **three consecutive times, each exit 0**:

```text
UI manual: all 9 method payloads and extension-received answer markers PASS
UI cancel: all 9 method payloads and extension-received answer markers PASS
Retry recover: real Pi + synthetic upstream fault injection PASS
Retry abort: real Pi + synthetic upstream fault injection PASS
Retry disabled: real Pi + synthetic upstream fault injection PASS
Auto compaction threshold: real Pi + synthetic usage, extension-provided summary (not Opus summarizer) PASS
Pi 1.1.0 real extension UI and synthetic retry acceptance: no paid request; clean exits PASS
```

Select/confirm/input/editor request IDs and payloads were checked, manual answers returned to the actual extension, and the extension acknowledged exact received values via notifications. Default Cancel returned undefined/false without manual responses. notify/status/widget/title/editor-text one-way payloads were all observed without acknowledgements. This verifies the SDK-to-Pi extension bridge, not a host's rendered graphical UI. Automatic compaction used synthetic 35000-token usage, a small-context fixture model and an extension-provided summary, with threshold reason, success flags and native fromHook=true entry checked. It does **not** demonstrate Anthropic rate limiting, production retry reliability, or Opus automatic summarization.

### Isolation incident and subsequent safeguards

**Important side effect:** the first two no-model state-test attempts inherited the normal Pi agent directory before persistent-setting behavior was recognized. They left **steeringMode=one-at-a-time, followUpMode=one-at-a-time, compaction.enabled=false and retry.enabled=false** in global settings. Source audit: agent-session.ts setSteeringMode/setFollowUpMode/setAutoCompactionEnabled/setAutoRetryEnabled call persistent SettingsManager setters. Default provider/model/thinking are **not** persisted by these RPCs because they do not pass `options.persist`; their Opus/off selection was session-only. Original values were not saved, so no speculative restoration was made. This incident was disclosed in the chat. All subsequent setting tests enforce scratch agent paths and use private opaque auth/models copies where needed, without reading/printing credential contents. A SHA-1 file-integrity check of the user's settings after the incident remained unchanged throughout subsequent isolated runs. This proves no later modification, not restoration of the original settings.

All saved sessions, HTML, tool gates and shell work are scratch-only. Examples are feature-gated and not executed by normal verification. Live/session exercises capture assertion-task failures and explicitly terminate/reap the process before returning, rather than relying on runtime-shutdown Drop. Credentials can be refreshed only in the private copies. Cleanup of retained scratch auth copies remains the caller's responsibility.

Local evidence logs: `pi-upgrade/rpc-state-smoke-20261009-1146.log` (isolated state, exit 0), `pi-full-rpc/simple.log`, `pi-full-rpc/live-run-{2,3,4,5}.log`, `pi-full-rpc/session-{compact,after-compact,compact-final}.log`, and `rpc-extension-final-{1,2,3}/acceptance.log`. Failed attempts are retained and distinguished above. Final `CARGO_NET_OFFLINE=true bash scripts/verify.sh` after process-cleanup corrections returned **exit 0** (`pi-full-rpc/verification-cleanup-final.log`): four check/test/clippy configurations, **222 test executions**, formatting, offline/fake examples and strict rustdoc all passed. The newly added examples compiled in both subprocess-enabled configurations. Final diff whitespace check passed, no acceptance-example processes remained, and the post-incident settings integrity check was still unchanged.

### Post-commit acceptance feedback

At 03:53 UTC the coordinator reran the committed state, session and extension examples against `/opt/homebrew/bin/pi` in isolated scratch environments, without additional paid requests. **All three returned exit 0.** `closure-state.log` again observed actual bash START before cancellation and exact state/entry/tree checks. `closure-session.log` checked the real Opus session after default compaction: clone/fork/new/switch identity and history checks passed, actual HTML **292518 bytes**, clean exit and unique terminal. `closure-extension.log` again passed both nine-method UI policies, retry recovery/active cancellation/disabled behavior and automatic threshold compaction. The original saved session contained **5 Opus assistant messages, 2 actual tool results and 1 nonempty native default compaction**. The user's post-incident settings checksum was unchanged and no acceptance-example processes remained. This extends the former 15-command read-only/minimal-prompt evidence to all 33 command names with observable real-Pi effects, while retaining the synthetic-source distinctions and incident limitations above.

Remaining limits: not every command variant, optional image field, cancellation hook, queue race, all-mode batching or all 24 events received from a hosted model. Tool coverage uses a real custom extension tool rather than all built-in tools. No real production provider retry failure was forced. Linux/Windows, exact Rust 1.85.0 and remote vendor deployment remain untested. Rust 1.85.1 was subsequently checked as described below. No push/publication or global Pi install was performed.

## Public-library readiness and Rust 1.85-series verification (2026-10-09)

The following commands were actually executed at 06:24–06:25 UTC on macOS aarch64, not inferred from `rust-version` or dependency metadata:

```bash
cargo package --offline --list
cargo package --offline
rustc +1.85 --version
cargo +1.85 check --offline --all-features --all-targets
cargo +1.85 test --offline --all-features
```

- Source archive: **34 files, 296.2 KiB uncompressed / 74.7 KiB compressed**, with package verification build **exit 0**. This was the archive before this documentation refresh, not a guaranteed size/count for future packages. `publish=false` does not prevent local packaging and remains set.
- The installed `+1.85` toolchain resolved to **rustc 1.85.1**. All-targets/all-features check passed; tests passed **59 integration + 2 unit + 3 doctests**, zero failures. This verifies 1.85.1, not the exact 1.85.0 release or other OSes.
- Cargo warned that license/license-file, documentation, homepage and repository metadata were absent. No crates.io upload, name-availability check or owner authentication was performed.
- Common credential-pattern and sensitive-filename scans across reachable Git history found no matching real credentials or committed auth/models/environment files. The retry fixture's API key is an explicitly synthetic constant. This is a limited local scan, not a proof that all secrets or privacy risks are absent.
- At 06:26 UTC the committed session and extension examples were rerun against real Pi, both **exit 0** with no additional hosted-model request: projected-history/identity checks and **292690-byte HTML** passed; both UI policies, retry recovery/active cancellation/disabled behavior and synthetic automatic compaction passed again.
- All ten existing commits' author and committer identities were unified at 06:36 UTC. Tree contents, messages and dates were verified unchanged. Local commit references in this document use the rewritten IDs; upstream Pi commit IDs and source hashes are unaffected. Rewriting authors does **not** remove private paths from older file contents.

See [public release checklist](publishing.md) for remaining metadata, third-party provenance, history-privacy and CI decisions. The owner selected Apache-2.0 later on the same date; the pre-license warnings above are historical observations, not the current license state. Local logs for package/MSRV checks are `pi-public-package.log`, `pi-public-msrv.log` and `pi-public-msrv-tests.log`; they are not included in the package.

## Apache-2.0 adoption (2026-10-09)

At the owner's direction, this project's original code and documentation now use **[Apache-2.0](../LICENSE)**. Cargo declares `license = "Apache-2.0"`; `publish = false` is unchanged. Pi and dependencies remain separately licensed. Third-party provenance/notices review remains on the [release checklist](publishing.md), rather than being inferred from build success.

- `LICENSE` matches the official `https://www.apache.org/licenses/LICENSE-2.0.txt` byte-for-byte, including the appendix. SHA-256: `cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30`.
- `cargo metadata --offline --no-deps` reports `Apache-2.0` and publication disabled. Actual `cargo package --offline --allow-dirty` built and verified a **36-file** archive; its `LICENSE` matches the working text and its normalized manifest preserves both settings. Cargo's missing-license warning is gone; documentation/homepage/repository warnings remain.
- Both all-features and no-default-features doctests passed (**6 executions**) and strict rustdoc passed. The first rustdoc attempt rejected the bare `LICENSE` target as an intra-doc identifier; changing the README target to `./LICENSE` resolved it. Local Markdown/link checks and Git whitespace checks passed.
- No paid request, live Pi configuration change, publication, push or history rewrite was performed. Local packaging log: `pi-apache-package.log` (not distributed).

## Validation supplement (2026-10-09)

The supplement implements reproducible release checks, not a new production-feature claim. Local platform: **macOS aarch64**, stable **rustc 1.97.1**, exact minimum **rustc 1.85.0 (4d91de4e4 2025-02-17)**. Exact 1.85.0 was installed without changing the default toolchain; `verify-msrv.sh` exports `RUSTUP_TOOLCHAIN=1.85.0` so its nested standalone fake compilation also uses the minimum compiler. `Cargo.lock` is unchanged, and resolving Cargo commands use `--locked`.

| Requirement | Executed check / observation | Result and boundary |
|---|---|---|
| Four-feature build/test/lint/docs/examples | `CARGO_NET_OFFLINE=true bash scripts/verify.sh` | PASS, **222 executions**, strict Clippy, strict docs in all four configurations and runnable injected/native/fake-local examples |
| Exact minimum compiler | `CARGO_NET_OFFLINE=true bash scripts/verify-msrv.sh` | PASS, four all-targets checks and **222 test executions** on **1.85.0**, not merely the installed `+1.85` alias (1.85.1) |
| Repeat process/terminal regressions | `CARGO_NET_OFFLINE=true bash scripts/verify-stress.sh` | PASS, three rounds of **29 client + 11 subprocess tests**, **120 executions**, no automatic failure retry; existing 3000/1000 inner loops retained |
| Windows-specific test compilation | `cargo check --locked --target x86_64-pc-windows-msvc --all-features --all-targets` and standalone fake `rustc --emit=metadata` for that target | PASS cross-check only, not linking or native Windows execution |
| Windows EOF/drop test implementation | Retained Win32 observation handle, synchronized stdout-close fixture, exactly-one-terminal assertions; Unix counterparts rerun | macOS counterparts PASS; Windows runtime awaits CI. Direct-child ownership only |
| CI syntax and integration | Six OS/toolchain jobs, pinned checkout/Python actions, timeouts, read-only permissions, stable three-round stress and package checks; `actionlint 1.7.7` | Static check PASS. ShellCheck/Pyflakes unavailable and disabled in that check. Hosted jobs **not run** |
| Real executable public interface | Isolated `verify-release.sh`, official digest-checked Darwin archive and actual installed npm launcher | Both PASS, **10 read-only RPCs each**, clean exit and unique terminal; no auth, model request or normal agent directory |
| Local documentation/archive checks | `check-docs.py` with explicit root and `check-package.py` against the actual Cargo-built archive | Explicit root, archive-local targets, official LICENSE hash, original/normalized metadata, source docs inventory and optional clean VCS checks; see final package result |
| Checker failure paths | `python3 scripts/test_checks.py` | **11 tests PASS**: changed CWD/root without Git, missing link/anchor, malformed fences, missing archive/source files, source/docs drift, wrong license/publish, wrong/dirty VCS, symlink/traversal/duplicate and known sensitive file names |

### Corrections discovered while validating

- The old private `pi-doc-check.py` ignored the supplied unpacked-root argument and always checked the current working directory. Earlier invocations that supplied a package path did **not** establish package-root Markdown validation. New committed tools actually read the specified root/archive; tests prove a deleted target fails even when it exists in the worktree. Current archive documents were separately read and checked before implementation as well.
- Independent review found `verify-release.sh` pinned the output directory but not Cargo's default build target, allowing a fresh cross-target build followed by an old host runner. It now explicitly selects the rustc host target and its corresponding executable path. Invalid inherited `CARGO_BUILD_TARGET` and alternate `CARGO_TARGET_DIR` are exercised without redirecting the runner.
- Independent review found initial `--source-root` comparison silently omitted deleted source files. It now requires core source comparison files and a bidirectional docs inventory, with deletion regressions.
- During shared-workspace delivery, a transient Git index lock prevented one command before any verification ran; it released normally and the command was rerun without removing an active lock. An early clean `cargo package` correctly rejected a concurrently edited script rather than including uncommitted work. Final packaging runs only after commits.

### Remaining external verification

The CI has a default-off manual real-Pi job for Linux x64 and macOS arm64. It pins Pi **1.1.0** official assets and their SHA-256 values: Linux x64 `3faa94666cd3849d37af320ff749407d0271b07a9b94f420c87e30866e10e289`, Darwin arm64 `3455b13de35c15a5893cdebc922678199e90a7ce99b06cbc23f37860e90d63c7`. Linux's digest was read from the official release API, not from an executed Linux acceptance. No Linux/Windows host or configured remote repository was available for actual hosted jobs, and no source upload/push was authorized. Windows real-Pi launching is not yet accepted; the POSIX release script explicitly rejects other platforms. Static CI checks and Windows cross-compilation cannot replace those runtime results.

### Final package result

After the implementation and documentation commits, `cargo package --locked --offline` built and verified a **44-file** archive. `check-package.py --source-root . --expected-commit <full-HEAD>` passed against that actual archive, including its clean VCS commit, exact official LICENSE, original/normalized metadata and source-document inventory. `check-docs.py target/package/pi-rpc-0.1.0` then passed **4 Markdown documents / 22 local links and anchors** from Cargo's actual unpacked package directory without Git or worktree fallback. No cache/private session/auth files were present in the inspected package file list; this is not an exhaustive secret scan.

### Packaged workflow acceptance

The actual **`bd976a2`** archive was also exercised from Cargo's unpacked package root, not the development worktree: `verify.sh`, `verify-msrv.sh`, three-round `verify-stress.sh`, all 11 checker tests, explicit-root document checks and the official Pi ten-read-only-RPC smoke all completed **exit 0**. This checks that the shipped scripts, Rust fixtures, examples and README inclusion work together without a `.git` dependency. It does not add Linux/Windows runtime evidence.

The first extra unpacked run failed during compilation with **No space left on device**. Only that run's generated package-local build cache was cleaned. The full workflow then passed with `CARGO_INCREMENTAL=0`, `CARGO_PROFILE_DEV_DEBUG=0`, `CARGO_PROFILE_TEST_DEBUG=0`, `CARGO_BUILD_JOBS=2` and offline dependencies. This is an explicit resource adjustment, not a silent test retry or timeout relaxation. Both failure and successful rerun logs are retained locally as `pi-verify-packaged-acceptance.log` and `pi-verify-packaged-acceptance-lowdisk.log`. The subsequent documentation-only commit and rebuilt archive were rechecked for source/document/VCS consistency.

Local logs: `pi-verify-complete.log`, `pi-verify-stress-final.log`, `pi-verify-msrv-final.log`, `pi-verify-real-release.log`, `pi-verify-real-npm.log` and `pi-verify-package.log`, not distributed. This supplement made no paid request, publication, push, history rewrite or production SDK semantic change.

## Historical verification: Pi 0.99.2 (2026-10-02)

Completed 2026-10-02 on macOS aarch64, from the repository root. All subsections in this archive describe that date and the former Pi 0.99.2 target, not the current coverage counts or remaining verification limits.
Toolchain: `rustc 1.97.1 (8bab26f4f 2026-07-14)`, `cargo 1.97.1 (c980f4866 2026-06-30)`.
No real Pi process or paid model request was used. The local executable tests/example use `tests/fixtures/fake_pi.rs`, compiled with rustc. No credentials/global configuration were read, printed or modified, and nothing was pushed, published or remotely created. Implementation was performed by the current agent without changing provider/model or spawning another implementation agent.

### Historical reproducible command

```bash
bash scripts/verify.sh
```

**Actual result: exit 0**, after fixing macOS Bash 3.2's empty-array/nounset incompatibility in the first script attempt. The delivered script was itself executed successfully, not just inspected. Full output was captured in the session scratch file `pi-rpc-final-verification.log`. Earlier full matrix and example runs were also captured as `pi-rpc-feature-matrix.log` and `pi-rpc-examples-docs.log`.

**Latest final run after the independent terminal-event race repair:** 2026-10-02 13:47 UTC, `bash scripts/verify.sh` again returned **exit 0**, with full output in `pi-rpc-terminal-race-final-matrix.log` in session scratch. The matrix below reflects this repaired code. The earlier successful runs were real observations but did not expose the event-consumer-before-owner race identified by the external harness.

### Historical feature matrix

Each row ran the actual public crate and its integration tests, not copied source substitutes. `--all-targets` also compiles applicable examples and tests. Strict clippy had **zero warnings**, including no-default-features dead code.

| Feature flags | `cargo check FLAGS --all-targets` | `cargo test FLAGS` | `cargo clippy FLAGS --all-targets -- -D warnings` |
|---|---|---|---|
| default (no flags) | PASS | 46 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--all-features` | PASS | 51 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--no-default-features` | PASS | 35 integration + 2 unit + 3 doctests, 0 failed | PASS |
| `--no-default-features --features session_file` | PASS | 40 integration + 2 unit + 3 doctests, 0 failed | PASS |

Other commands actually run and passed:

```bash
cargo fmt --check
rustfmt --edition 2024 --check tests/fixtures/fake_pi.rs
cargo run --example injected --no-default-features
cargo run --example injected --all-features
cargo run --example native_session --no-default-features --features session_file
cargo run --example native_session --all-features
rustc --edition=2024 tests/fixtures/fake_pi.rs -o target/verification/fake-pi
cargo run --example local -- "$PWD/target/verification/fake-pi"
cargo run --example local --all-features -- "$PWD/target/verification/fake-pi"
RUSTDOCFLAGS='-D warnings' cargo doc --all-features --no-deps
RUSTDOCFLAGS='-D warnings' cargo doc --no-default-features --no-deps
git diff --check
```

Both local-example runs reported `streaming=false pending=0` and terminal `ProcessExit(Some(0))` through the same `PiProcess` API used for real Pi. In-memory/native examples exited 0. README code examples are included as crate docs and compile as three doctests. With no default features, the local subprocess snippet is intentionally feature-gated.

### Historical requirement-to-test coverage

- **26 injected transport tests** (`tests/client.rs`): partial/coalesced CRLF and Unicode records, immediate reading before submission, out-of-order ID responses, abort during long execution, cancellation without withdrawal, timeout and ignored late response, no auto-retry, prompt dispositions, cancelled new/switch/fork/clone, nonterminal rejection, uncorrelated parse response, malformed known event failure, explicit reader transport error, wrong-command ID failure, independent count/byte overflow, event budget release, stalled writer count/byte bounds, owner vs clone drop, admitted-write drain/close, close deadline, record limits/unterminated EOF, default UI cancellation, manual no-ack UI and stop queue race. Four added event-first regressions cover 3000 immediately failing readers, 1000 buffered event then failure sequences, EOF/task panic, and owner drop after a cancelled receive.
- **2 deterministic terminal publication unit tests** (`src/client.rs`): event channel deliberately closed while terminal publication is delayed, cancelled recv retains the error, and disappearing terminal authority returns Closed exactly once.
- **9 protocol tests** (`tests/protocol.rs`): exhaustive **33** command round trips and all **33** success data schemas, all **24** known ordinary event kinds, **12** delta-only assistant variants, extensible native messages, UI answer checks, strict known shapes, required nullable fields and required open payload presence.
- **5 native session tests** (`tests/session_file.rs`, optional): all **11** entry kinds, native messages/system tools and sections, opaque metadata, context replacement object/null, native parentId absence vs null, branch edits/retain-none compaction/multiple roots, cycles/orphans/duplicates, legacy rejection, CRLF and file/record/entry bounds.
- **11 local process tests** (`tests/subprocess.rs`, default feature): real OS fake-executable pipes and fragmented replies, version mismatch/oversized/hung probes, continuously drained bounded stderr, process exit status, malformed final record, explicit terminate/reap, close deadline cleanup, event overflow cleanup, owner drop with surviving clients, descendant-held stdout after direct-child exit, and fd1 closed while direct child remains alive.

The wrong-command ID regression uses Tokio `multi_thread`, four workers and **100 iterations** per run. Both caller and connection are asserted to receive the same Protocol reason. The process race tests require terminal state within 700 ms: descendant-held stdout resolves `ProcessExit(Some(9))` and the pending command, while unexpected stdout EOF with a living child resolves `Eof` and confirms the direct child was reaped. Required nullable tests distinguish missing fields from explicit JSON null through both RPC decoding and native file parsing. Missing tool args/partialResult/result are rejected, while JSON null remains accepted.

### Historical fixes encountered and verified

- Corrected documentation/test counts from initial 32/25 assumptions to the exhaustively checked **33 commands / 24 events**.
- Explicitly delivered Protocol to a removed pending waiter on ID/command mismatch before returning, eliminating the sender-drop race. Multithread regression passes.
- Added independent process-exit wakeup with a bounded 50 ms ready-output drain, and a bounded 100 ms child-status race grace after stdout EOF.
- Added required-nullable custom deserialization for native parentId/context replacement and entries/tree/text, plus success-envelope presence checks for nullable cycle data.
- Added required open tool property presence and nested assistant update/role/content validation without closing arbitrary tool/provider/extension fields.
- Fixed one strict clippy guarded-match warning and a too-short test startup budget for copied macOS fake executables.
- Fixed the verification script for macOS Bash 3.2, then reran the full delivered script successfully.

### Historical boundaries and additional observations

#### Independent terminal-event race: reproduced, repaired, externally verified

The external test file `review-harness/src/lib.rs` was read without changing the external harness. On the unchanged crate, the command below failed at **iteration 2**. The path is a portable placeholder for that private harness, which is not included in this repository:

```bash
cargo test --offline --manifest-path /absolute/path/to/review-harness/Cargo.toml
```

Observed failure: `events.recv()` returned `None`, while `owner.wait_terminal()` returned `Transport("review transport error")`. The reader dropped its event sender before the supervisor published the terminal watch. The earlier transport-failure test called owner.wait_terminal first, so it accidentally hid this interleaving. Previous feedback-loop closure claims were therefore too broad for terminal delivery.

The fix makes `PiEvents::recv` drain the event channel and then **wait for the authoritative terminal reason**, marking the error delivered only when actually returning it. A cancelled wait does not consume the terminal error. Missing terminal authority defensively yields Closed once instead of hanging. No new queue, detached task or polling loop was introduced.

The unchanged external command then passed **twice**, each running all **3000 iterations** on four Tokio workers (0.12 s and 0.11 s test execution). It was rerun after the full final matrix as well. In-crate stress adds **3000 immediate reader failures** consumed before waiting on the owner and **1000 buffered event then failure** connections, asserting precisely `buffered events -> one matching terminal error -> None`. Additional multithread tests cover reader-task panic, EOF and owner drop, with bounded receive deadlines. Two deterministic unit tests force delayed terminal publication, cancel the waiting recv and check terminal-authority disappearance. These observations directly test the previously failing acceptance order, rather than only inspecting the repair.

The final script observed **46/51/35/40 integration tests**, plus **2 unit tests and 3 doctests in each feature configuration**, with zero failures. Every requested check --all-targets, test, clippy --all-targets -D warnings and fmt --check passed. Strict all-features/no-default docs and runnable injected/native_session examples passed. Local examples were only executed against `target/verification/fake-pi`, never a live Pi installation. No paid model requests were made. No existing source or harness work was removed. Git diff checks passed, license remains undecided, and publish=false is retained.

#### Post-completion feedback-loop observation

##### Whole-result rerun after finalized requirement mapping

At 2026-10-02 **13:50 UTC**, the complete delivered result at local commit `9534f9a` was checked again, not just the new race tests. `bash scripts/verify.sh` returned **exit 0**. Session scratch log: `pi-rpc-whole-result-reverification.log`. All mapped executable checks ran after the requirement-to-test mapping was finalized. The unchanged external harness was also rerun and passed its 3000 four-thread iterations in **0.11 s**. The working tree was clean before recording these observations.

| Requirement / changed public output | Check actually run over complete result | Observed outcome |
|---|---|---|
| `PiEvents::recv`: terminal error reaches consumer before owner wait | `event_consumer_gets_exactly_one_terminal_before_waiting_for_owner_3000_runs` in all four configurations, unchanged external `terminal_error_always_reaches_event_consumer` | Both passed every iteration, no premature `None` |
| Buffered events stay ordered, one terminal error, then `None` | `buffered_event_precedes_exactly_one_terminal_error_1000_runs`, `owner_drop_wakes_waiting_events_and_preserves_buffered_event_order` | Passed in all four configurations |
| Delayed terminal publication and cancelled recv retain delivery | `closed_event_channel_waits_for_publication_and_cancelled_recv_keeps_error` | Passed in all four configurations |
| Missing terminal authority does not hang or repeat an error | `terminal_authority_disappearing_after_event_close_returns_closed_once` | Passed in all four configurations |
| EOF and reader panic wake event consumer with live owner | `event_first_eof_and_panicking_reader_do_not_hang_with_live_owner` | Passed with bounded receives in all four configurations |
| Bounded transport, explicit overflow and released budgets | Count/byte overflow, stalled-writer bounds and event-budget-release tests in `tests/client.rs` | All passed in all four configurations, no silent-drop acceptance |
| Matching caller/owner reasons and remaining RPC lifecycle | All 26 client integration tests, including 100-run wrong-command/same-ID race, cancellation, timeout, close, UI and stop-race tests | All passed in each configuration |
| Pinned protocol surface and required fields | All 9 protocol tests, exhaustively covering 33 commands/success schemas, 24 event kinds, 12 deltas, nullable and open tool payload validation | All passed in each configuration |
| Default subprocess lifecycle and process races | All 11 subprocess tests, real fake-executable pipes, descendant-held stdout, living-child EOF, cleanup and version probes | Passed in default and all-features; intentionally absent without subprocess feature |
| Optional native v3 session-file API | All 5 native session tests covering 11 entry kinds, branching, required-nullable fields, legacy rejection and bounds | Passed in all-features and session-only; intentionally absent without session_file |
| Public examples and README code | Injected example in no-default/all, native example in session-only/all, local example in default/all, three README doctests per configuration | Every invocation exited 0; both fake-backed local runs printed `streaming=false pending=0` and process exit code `Some(0)` |
| README/compatibility terminal semantics and generated public docs | README doctests, `RUSTDOCFLAGS='-D warnings' cargo doc` for all-features/no-default, terminal behavior checks above | All passed; generated `target/doc/pi_rpc/index.html`; prose claims match observed terminal checks |
| Complete feature/build/lint matrix | Four `check --all-targets`, four `test`, four `clippy --all-targets -- -D warnings`, crate and fake-fixture fmt | All passed, zero warnings/failures; 46/51/35/40 integration + 2 unit + 3 doctests per configuration |
| Preserve user constraints and local delivery | Manifest inspected, Git diff/status checks and local history | `publish=false`, no license invented; clean tree at `9534f9a`, no remote action or live/paid Pi request |

This rerun includes the earlier protocol, transport, subprocess, native-session, examples and documentation outputs as well as the latest receiver repair. It verifies the requested local fake/mock acceptance workflow. It does not extend evidence to live model requests, vendor deployment, other OSes or the unexercised Rust 1.85 minimum, whose limitations remain below.

At 2026-10-02 13:42 UTC, the delivered `bash scripts/verify.sh` was executed again and returned **exit 0**. All four feature-matrix test counts remained 42/47/31/36 integration tests plus three doctests each, with zero failures. Both local-example invocations again observed `streaming=false pending=0` and `ProcessExit(Some(0))`, rather than relying on source inspection.

The six independent-review regression tests were then run individually through the public crate and each passed: descendant-held stdout resolved exit and pending command (0.41 s including fake compilation), stdout closed by a living child resolved EOF and reaping (0.42 s including fake compilation), wrong-command/same-ID delivered matching Protocol reasons over 100 multithread iterations (0.01 s), required-nullable RPC/native fields rejected absence but accepted null, missing open tool payload fields/nested malformed deltas were rejected, and native file parsing rejected absent parentId/replacement. `git diff --check` passed and the working tree was clean before recording this observation. The SDK's executable acceptance checks therefore continue to satisfy the requested mock/fake-process workflow after the review fixes. Paid live model behavior remains deliberately untested.

There are **no unresolved build/test/lint blockers on this machine**. Live paid-model acceptance was deliberately not attempted. The stop continuation discrepancy is tested as client behavior with a mock race, not as a live upstream model execution. Remote Sandbox integration is the generic real AsyncRead/AsyncWrite boundary tested with Tokio byte pipes, not a particular vendor SDK deployment.

Only macOS aarch64 with the stated toolchain was exercised. The manifest's Rust 1.85 minimum was not separately tested with an installed 1.85 toolchain, and Windows/Linux process behavior was not run here. Native timestamps are preserved/presence-checked, not calendrically parsed. Native v1/v2 migration, complete Pi model-context projection, vendor history conversion and remote session delivery are intentionally excluded. Child ownership is direct-child only, not detached descendants. Exit output after the bounded 50 ms drain is not guaranteed. Owner cleanup requires a live Tokio runtime; explicit close/terminate is preferred before runtime shutdown. All are documented in README/compatibility notes. License remains undecided and `publish = false` remains set.
