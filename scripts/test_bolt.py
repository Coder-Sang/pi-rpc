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
    cases = [
        ("src/client.rs", "message_id: message_id.as_deref().map(String::as_str),", "message_id: None,",
         "identified_inputs_use_separate_rpc_ids_and_preserve_existing_payloads"),
        ("src/client/bolt_message_id.rs", "!= Some(expected.as_str())", "!= response.data.as_ref().and_then(|data| data.get(\"message_id\")).and_then(|id| id.as_str())",
         "missing_or_mismatched_echo_has_uncertain_outcome_and_is_not_retried"),
        ("src/protocol/bolt_message_id_v1.rs", "json::decode(value).map(Event::QueueUpdate)", "<Pi15 as Sealed>::known(value).map(Event::Pi)",
         "malformed_tracking_is_never_downgraded_to_unknown_or_untracked"),
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

        def check(test, rejected):
            result = subprocess.run(
                ["cargo", "test", "--offline", "--locked", "--no-default-features", "--features", "bolt_message_id",
                 "--test", "bolt_message_id", test, "--", "--exact"],
                cwd=work, env=environment, capture_output=True, encoding="utf-8", errors="replace",
            )
            expected = f"test {test} ... {'FAILED' if rejected else 'ok'}"
            if result.returncode != (101 if rejected else 0) or expected not in result.stdout:
                raise SystemExit(f"FAIL negative-control validity: {test}, rejected={rejected}\n{result.stdout}\n{result.stderr}")
            print(f"PASS {test}: {'broken implementation rejected' if rejected else 'positive control'}", flush=True)

        for file, original, replacement, test in cases:
            source = work / file
            text = source.read_text(encoding="utf-8")
            if text.count(original) != 1:
                raise SystemExit(f"FAIL ambiguous mutation target: {file}")
            check(test, False)
            try:
                source.write_text(text.replace(original, replacement), encoding="utf-8", newline="\n")
                check(test, True)
            finally:
                source.write_text(text, encoding="utf-8", newline="\n")


if __name__ == "__main__":
    main()
