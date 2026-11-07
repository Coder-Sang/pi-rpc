#!/usr/bin/env python3
"""Prove extension regressions detect plausible broken implementations, offline."""
import os
from pathlib import Path
import shutil
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parent.parent


def main():
    environment = os.environ.copy()
    target = Path(environment.get("CARGO_TARGET_DIR", ROOT / "target")).resolve()
    environment["CARGO_TARGET_DIR"] = str(target / "bolt-negative-controls")
    # README consumers fetch normal dependencies, but do not populate SDK
    # dev-dependencies on a cold runner. Honor CARGO_NET_OFFLINE when set;
    # otherwise fetch the locked test graph once before the offline controls.
    subprocess.run(["cargo", "fetch", "--locked"], cwd=ROOT, env=environment, check=True)
    cases = [
        ("omitted-input-id", "src/client.rs", "message_id: message_id.as_deref().map(String::as_str),", "message_id: None,",
         "identified_inputs_use_separate_rpc_ids_and_preserve_existing_payloads"),
        ("unchecked-echo", "src/client/bolt_message_id.rs", "!= Some(expected.as_str())", "!= response.data.as_ref().and_then(|data| data.get(\"message_id\")).and_then(|id| id.as_str())",
         "missing_or_mismatched_echo_has_uncertain_outcome_and_is_not_retried"),
        ("skipped-tracking", "src/protocol/bolt_message_id_v1.rs", "json::decode(value).map(Event::QueueUpdate)", "<Pi15 as Sealed>::known(value).map(Event::Pi)",
         "malformed_tracking_is_never_downgraded_to_unknown_or_untracked"),
        ("fixed-instance-id", "src/protocol/bolt_message_id_v1.rs", "instance_id: w.instance_id,", 'instance_id: "instance-A".into(),',
         "fragmented_crlf_tracking_precedes_acceptance_and_keeps_wire_order"),
        ("fixed-sequence", "src/protocol/bolt_message_id_v1.rs", "sequence: w.sequence,", "sequence: 7,",
         "fragmented_crlf_tracking_precedes_acceptance_and_keeps_wire_order"),
        ("fixed-batch-id", "src/protocol/bolt_message_id_v1.rs", "batch_id: w.batch_id,", 'batch_id: w.batch_id.map(|_| "opaque-batch".into()),',
         "fragmented_crlf_tracking_precedes_acceptance_and_keeps_wire_order"),
        ("echo-starts-with-request", "src/client/bolt_message_id.rs", "!= Some(expected.as_str())", ".is_none_or(|id| !id.starts_with(expected.as_str()))",
         "identified_echoes_require_exact_bytes_without_retry"),
        ("request-starts-with-echo", "src/client/bolt_message_id.rs", "!= Some(expected.as_str())", ".is_none_or(|id| !expected.as_str().starts_with(id))",
         "identified_echoes_require_exact_bytes_without_retry"),
        ("trimmed-echo", "src/client/bolt_message_id.rs", "!= Some(expected.as_str())", ".map(str::trim) != Some(expected.as_str().trim())",
         "identified_echoes_require_exact_bytes_without_retry"),
        ("case-insensitive-echo", "src/client/bolt_message_id.rs", "!= Some(expected.as_str())", ".is_none_or(|id| !id.eq_ignore_ascii_case(expected.as_str()))",
         "identified_echoes_require_exact_bytes_without_retry"),
    ]
    # Copy only source-controlled/nonignored caller files, never credentials,
    # build products or the repository's .git. Do not mutate the working source.
    names = subprocess.run(["git", "ls-files", "--cached", "--others", "--exclude-standard", "-z"],
                           cwd=ROOT, check=True, capture_output=True).stdout.split(b"\0")
    with tempfile.TemporaryDirectory(prefix="pi-rpc-bolt-controls-") as work:
        work = Path(work)
        for raw in names:
            if not raw:
                continue
            relative = Path(os.fsdecode(raw))
            source = ROOT / relative
            if not source.is_file() or source.is_symlink():
                raise SystemExit(f"FAIL unsupported source entry: {relative}")
            destination = work / relative
            destination.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source, destination)

        def check(label, test, rejected):
            result = subprocess.run(
                ["cargo", "test", "--offline", "--locked", "--no-default-features", "--features", "bolt_message_id",
                 "--test", "bolt_message_id", test, "--", "--exact"],
                cwd=work, env=environment, capture_output=True, encoding="utf-8", errors="replace",
            )
            expected = f"test {test} ... {'FAILED' if rejected else 'ok'}"
            if result.returncode != (101 if rejected else 0) or expected not in result.stdout:
                raise SystemExit(f"FAIL negative-control validity: {label}, {test}, rejected={rejected}\n{result.stdout}\n{result.stderr}")
            print(f"PASS {label} ({test}): {'broken implementation rejected' if rejected else 'positive control'}", flush=True)

        for label, file, original, replacement, test in cases:
            source = work / file
            text = source.read_text(encoding="utf-8")
            if text.count(original) != 1:
                raise SystemExit(f"FAIL ambiguous mutation target: {file}")
            check(label, test, False)
            try:
                source.write_text(text.replace(original, replacement), encoding="utf-8", newline="\n")
                check(label, test, True)
            finally:
                source.write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
