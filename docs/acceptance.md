# Real Pi acceptance

These checks are opt-in and are never run by the normal fake-only verification workflow. Start with the [development requirements](../CONTRIBUTING.md#development-requirements). Commands below run from the repository root. Pi must be supplied separately.

## Read-only release smoke

The read-only runner defaults to concrete `PI_VERSION` (**1.5.0**). SDK defaults instead admit >=1.3.0, <=1.5.0 subject to enabled profiles. Acceptance entrypoints reject every range expression, including the new default and all three historical strings, before probing. `release_smoke` uses automatic protocol selection while retaining the external probe. `verify-release.sh` enables Pi13 in addition to the default Pi14/Pi15 profiles, so an explicit older supported endpoint can be selected.

For separately authorized non-paid acceptance, run `bash scripts/verify-release.sh /absolute/path/to/pi-1.5.0`; select an older supported release with `bash scripts/verify-release.sh /absolute/path/to/pi-1.3.0 1.3.0`. The script runs ten read-only RPCs and checks orderly exit/terminal delivery. It clears inherited environment, uses temporary HOME/agent/work directories, and disables sessions/tools/MCP/extensions/skills/templates/themes. It installs no Pi and submits no prompt. These instructions are a procedure, not completed live acceptance of 0.3. Normal verification uses only compiled fake processes; Cargo offline mode also forbids dependency downloads.

## Minimal model acceptance

The typed `model_smoke` example and the broader examples below use **Pi15** and require `subprocess` plus `pi_1_5`. For **separately authorized paid acceptance**, prepare isolated agent and empty work directories, then run `PI_CODING_AGENT_DIR=/absolute/scratch/agent cargo run --example model_smoke -- /absolute/path/to/pi-1.5.0 /absolute/path/to/empty-work anthropic`. Omitting the final exact-version argument selects 1.5.0. Another declared minor requires selecting its protocol type in the harness; it is not silently coerced to Pi15. Nonmatching declarations fail before RPC starts.

The model is Claude Opus 4.6, Pi ID `claude-opus-4-6`. This example disables tools/extensions/session saving/automatic retry and sends one fixed prompt. Without the provider argument it lists matching providers without submitting the prompt. It checks acceptance, streaming text, assistant completion, settlement, state/history and terminal delivery. Pi owns authentication and can refresh it. Unlike verify-release.sh, this inherits the configured environment and its setting RPCs persist into the selected agent directory. Use private disposable credential/model copies only with explicit authorization, never the normal agent directory. The SDK/harness does not provide an isolation sandbox. Normal verification runs only no-Pi unit tests of this example.

## Broader real-Pi acceptance

All four examples below are opt-in, Pi15-typed and subprocess/pi_1_5-feature gated. They are compiled, but **not run**, by `verify.sh`. Pi setting commands can persist configuration. Never point them at your normal agent directory. Use an absolute scratch `ROOT`, set `PI_CODING_AGENT_DIR=ROOT/agent`, and prepare private copies of `auth.json` and, if required, `models.json` there without printing their contents. Give the directory mode 700 and files mode 600. Each run needs fresh work/session directories. `rpc_live_smoke` overwrites `ROOT/agent/settings.json` with its test-only settings, so give it a dedicated disposable agent directory rather than a shared scratch configuration. `rpc_session_smoke` changes its supplied session and overwrites `ROOT/acceptance-export.html`; use a disposable copy of any history you want to retain. No private files belong in this repository.

| Example | Arguments after `--` | Actual checks |
|---|---|---|
| `rpc_state_smoke` | `/path/pi ROOT` | Queries, model/thinking/queue modes, setting toggles, actual bash output and active bash cancellation, native entries/tree/since, new session. No model request. Unsaved bash-only export/clone restrictions are explicitly reported. |
| `rpc_live_smoke` | `/path/pi ROOT` | **Paid Opus 4.6** tool execution, prompt queue acceptance, steer/follow-up consumption, exact queue removal, active abort, default manual compaction, fork and switch. Only its explicit local gate tool is enabled. It writes dedicated scratch settings with `keepRecentTokens=0` so the small test conversation can compact. |
| `rpc_session_smoke` | `/path/pi ROOT ROOT/session.jsonl [--compact]` | Name, HTML file, clone, fork, new session and switch with exact history/identity checks. Existing session must be inside ROOT. No additional generation unless **paid** `--compact` is supplied; that option requires an appropriate small keepRecentTokens setting in the dedicated agent. |
| `rpc_extension_smoke` | `/path/pi /fresh/scratch/cwd` | Nine UI methods under manual and default-cancel policies, actual extension-received answers, retry recovery/active cancellation/disabled behavior and automatic compaction. Requires `PI_CODING_AGENT_DIR` and `PI_CODING_AGENT_SESSION_DIR` inside its scratch cwd. Uses an in-memory synthetic provider and extension-provided automatic summary, **not hosted-model fault evidence or Opus summarization**. |

Use `cargo run --offline --example NAME -- ...`. Path guards in the opt-in examples are lexical, not a filesystem sandbox: use trusted scratch paths without symlinks or parent-directory traversal. Existing provider authentication is handled only by Pi. Settings isolation does not guarantee a readonly authentication copy: Pi may refresh tokens in that private copy. Scratch authentication/configuration cleanup remains the caller's responsibility. See the dated [per-RPC evidence and limitations](verification.md#real-pi-acceptance).

## Pi 1.5.0 examples and historical acceptance

The manual CI recipe pins official 1.5.0 assets with verified API digests and is disabled on ordinary pushes. It has not been run for this local 0.3 change. Historical 1.1/1.2 live acceptance belongs to the recorded older source; use that tagged source to reproduce it. Current source rejects 1.1/1.2 connections. Synthetic default/exact/probe regressions are separate from real Pi/model acceptance, which always requires authorization.
