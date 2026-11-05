#!/usr/bin/env python3
"""Regression tests for feature selection, fail-fast checks and include! formatting."""
import itertools
import os
from pathlib import Path
import re
import runpy
import subprocess
import sys
import tempfile
import unittest

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent
FEATURES = ("pi_1_3", "pi_1_4", "pi_1_5", "subprocess", "session_file")


class FeatureGates(unittest.TestCase):
    def run_matrix(self, *arguments, fail_kind=""):
        # Source the actual script with command observers. No Rust build, Pi or
        # network request is needed to verify the selected commands and failures.
        wrapper = r'''
record() {
  printf '%s\0' "$@" >> "$PI_RPC_GATE_LOG"
  printf '\n' >> "$PI_RPC_GATE_LOG"
}
cargo() {
  record cargo "$@"
  if [ "${1-}" = "$PI_RPC_FAIL_KIND" ]; then return 23; fi
}
python3() { record python3 "$@"; }
source "$0" "$@"
'''
        with tempfile.TemporaryDirectory() as work:
            log = Path(work) / "commands"
            environment = os.environ.copy()
            environment.update(PI_RPC_GATE_LOG=log.as_posix(), PI_RPC_FAIL_KIND=fail_kind)
            result = subprocess.run(
                ["bash", "-c", wrapper, (ROOT / "scripts/verify-protocols.sh").as_posix(), *arguments],
                env=environment, capture_output=True, encoding="utf-8", errors="replace",
            )
            calls = [line.decode().split("\0")[:-1] for line in log.read_bytes().splitlines()] if log.exists() else []
        return result, calls

    def assert_matrix(self, calls, clippy):
        expected = {frozenset(name for name, enabled in zip(FEATURES, bits) if enabled)
                    for bits in itertools.product((False, True), repeat=len(FEATURES))}
        selected = {kind: [] for kind in ("check", "test", "clippy")}
        for call in calls:
            if call[0] != "cargo":
                continue
            kind = call[1]
            self.assertIn(kind, selected)
            self.assertIn("--locked", call)
            self.assertIn("--no-default-features", call)
            features = frozenset(call[call.index("--features") + 1].split(",")) if "--features" in call else frozenset()
            selected[kind].append(features)
            if kind in ("check", "clippy"):
                self.assertIn("--all-targets", call)
            if kind == "clippy":
                self.assertEqual(call[-3:], ["--", "-D", "warnings"])
            if kind == "test":
                targets = [call[index + 1] for index, value in enumerate(call) if value == "--test"]
                self.assertEqual(targets, ["protocol", "public_paths", "versioned"]
                                 if features.intersection(FEATURES[:3]) else [])
        for kind in ("check", "test", "clippy") if clippy else ("check", "test"):
            self.assertEqual(len(selected[kind]), len(expected), kind)
            self.assertEqual(set(selected[kind]), expected, kind)
        if not clippy:
            self.assertEqual(selected["clippy"], [])
        self.assertEqual([call for call in calls if call[0] == "python3"],
                         [["python3", "scripts/check-protocol-types.py"]])

    def test_strict_matrix_checks_all_targets_and_every_feature_combination(self):
        result, calls = self.run_matrix("--clippy")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_matrix(calls, clippy=True)

    def test_msrv_matrix_does_not_require_the_clippy_component(self):
        result, calls = self.run_matrix()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assert_matrix(calls, clippy=False)

    def test_check_test_and_clippy_failures_are_not_hidden(self):
        for kind in ("check", "test", "clippy"):
            with self.subTest(kind=kind):
                result, calls = self.run_matrix("--clippy", fail_kind=kind)
                self.assertEqual(result.returncode, 23, result.stderr)
                self.assertEqual(calls[-1][:2], ["cargo", kind])
                self.assertFalse(any(call[0] == "python3" for call in calls))

    def test_unknown_or_duplicate_options_fail_before_commands(self):
        for arguments in [("--unknown",), ("--clippy", "--clippy"), ("extra", "arguments")]:
            with self.subTest(arguments=arguments):
                result, calls = self.run_matrix(*arguments)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("usage:", result.stderr)
                self.assertEqual(calls, [])

    def test_windows_observer_follows_enabled_protocols(self):
        source = (ROOT / "tests/versioned/process.rs").read_text(encoding="utf-8")
        declaration = re.search(r"(?:#\[[^\]]+\]\s*)+mod windows_process;", source)
        self.assertIsNotNone(declaration, "missing Windows observer declaration")
        with tempfile.TemporaryDirectory() as work:
            work = Path(work)
            (work / "tests/versioned").mkdir(parents=True)
            (work / "tests/support").mkdir()
            sentinel = "WINDOWS_OBSERVER_PROFILE_CONTROL"
            (work / "tests/support/windows_process.rs").write_text(
                f'compile_error!("{sentinel}");\n', encoding="utf-8")
            probe = work / "tests/versioned/probe.rs"
            probe.write_text(declaration.group() + "\nfn main() {}\n", encoding="utf-8")
            # Check only the real declaration's cfg truth table. Explicitly
            # enabling Windows here is a compile probe, not a Windows runtime.
            for bits in itertools.product((False, True), repeat=3):
                features = [name for name, enabled in zip(FEATURES[:3], bits) if enabled]
                with self.subTest(features=features):
                    command = ["rustc", "--edition", "2024", "--emit", "metadata", "--cfg", "windows",
                               "-Aexplicit_builtin_cfgs_in_flags", "--out-dir", str(work), str(probe)]
                    for feature in features:
                        command.extend(["--cfg", f'feature="{feature}"'])
                    result = subprocess.run(command, capture_output=True, encoding="utf-8", errors="replace")
                    if features:
                        self.assertNotEqual(result.returncode, 0, "observer control was not compiled")
                        self.assertIn(sentinel, result.stderr)
                    else:
                        self.assertEqual(result.returncode, 0, result.stderr)

    def test_stable_and_msrv_entrypoints_keep_the_required_checks(self):
        wrapper = r'''
record() {
  printf '%s\0' "$@" >> "$PI_RPC_GATE_LOG"
  printf '\n' >> "$PI_RPC_GATE_LOG"
}
cargo() { record cargo "$@"; }
python3() { record python3 "$@"; }
bash() { record bash "$@"; }
rustc() { record rustc "$@"; printf 'rustc 1.88.0 (command observer)\n'; }
source "$0"
'''
        for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as work:
                root = Path(work)
                script = root / "scripts" / name
                script.parent.mkdir()
                script.write_bytes((ROOT / "scripts" / name).read_bytes())
                log = root / "commands"
                environment = os.environ.copy()
                environment["PI_RPC_GATE_LOG"] = log.as_posix()
                result = subprocess.run(["bash", "-c", wrapper, script.as_posix()], env=environment,
                                        capture_output=True, encoding="utf-8", errors="replace")
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                calls = [line.decode().split("\0")[:-1] for line in log.read_bytes().splitlines()]
                self.assertIn(["bash", "scripts/verify-protocols.sh", *(["--clippy"] if clippy else [])], calls)
                if clippy:
                    self.assertIn(["python3", "scripts/check-format.py"], calls)
                    self.assertIn(["python3", "scripts/test_gates.py"], calls)
                else:
                    self.assertIn(["python3", "scripts/test_gates.py",
                                   "FeatureGates.test_windows_observer_follows_enabled_protocols"], calls)


class FormatGates(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory()
        self.addCleanup(temporary.cleanup)
        self.root = Path(temporary.name) / "fixture with spaces"
        self.root.mkdir()
        (self.root / "Cargo.toml").write_text(
            '[package]\nname = "format-gate-fixture"\nversion = "0.0.0"\nedition = "2024"\npublish = false\n',
            encoding="utf-8",
        )
        self.files = {
            "src/lib.rs": 'macro_rules! declare {\n    () => {\n        mod included {\n            include!("included.rs");\n        }\n    };\n}\ndeclare!();\n',
            "src/included.rs": "pub fn answer() -> u32 {\n    42\n}\n",
            "tests/protocol.rs": 'macro_rules! suite {\n    () => {\n        mod fields {\n            include!("protocol/fields.rs");\n        }\n    };\n}\nsuite!();\n',
            "tests/protocol/fields.rs": "#[test]\nfn fields() {\n    assert_eq!(2 + 2, 4);\n}\n",
            "tests/fixtures/helper.rs": "fn main() {}\n",
            "examples/example.rs": "fn main() {}\n",
        }
        for name, text in self.files.items():
            path = self.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text, encoding="utf-8")

    def check_format(self):
        return subprocess.run([sys.executable, str(ROOT / "scripts/check-format.py"), str(self.root)],
                              cwd=self.root.parent, capture_output=True, encoding="utf-8", errors="replace")

    def test_formatted_macro_includes_and_standalone_fixtures_pass(self):
        result = self.check_format()
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
        self.assertIn(f"{len(self.files)} explicit Rust sources", result.stdout)

    def test_include_and_fixture_format_failures_survive_cargo_fmt_blind_spot(self):
        for name in ("src/included.rs", "tests/protocol/fields.rs", "tests/fixtures/helper.rs"):
            with self.subTest(name=name):
                path = self.root / name
                path.write_text("fn malformed_spacing( ){let value=1;assert_eq!(value,1);}\n", encoding="utf-8")
                try:
                    cargo = subprocess.run(["cargo", "fmt", "--check"], cwd=self.root,
                                           capture_output=True, encoding="utf-8", errors="replace")
                    self.assertEqual(cargo.returncode, 0, cargo.stdout + cargo.stderr)
                    result = self.check_format()
                    self.assertNotEqual(result.returncode, 0, "formatting omission was accepted")
                    self.assertIn("Diff in", result.stdout)
                    self.assertIn(Path(name).name, result.stdout)
                finally:
                    path.write_text(self.files[name], encoding="utf-8")

    def test_discovery_covers_source_roots_but_excludes_build_artifacts(self):
        generated = self.root / "target/generated.rs"
        generated.parent.mkdir()
        generated.write_text("not a Rust source", encoding="utf-8")
        checker = runpy.run_path(str(ROOT / "scripts/check-format.py"))
        self.assertEqual(checker["rust_sources"](self.root), sorted(self.files))


if __name__ == "__main__":
    unittest.main(verbosity=2)
