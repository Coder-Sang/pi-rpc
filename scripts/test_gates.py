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
from unittest.mock import patch

sys.dont_write_bytecode = True
ROOT = Path(__file__).resolve().parent.parent
FEATURES = ("pi_1_3", "pi_1_4", "pi_1_5", "subprocess", "session_file")


def verification_bash(environment=None):
    environment = os.environ if environment is None else environment
    value = environment.get("PI_RPC_TEST_BASH")
    if not value:
        raise ValueError("PI_RPC_TEST_BASH is required; use the verification entrypoint")
    path = Path(value)
    if not path.is_absolute() or not path.is_file() or not os.access(path, os.X_OK):
        raise ValueError("PI_RPC_TEST_BASH must name an existing absolute Bash executable")
    return str(path)


class FeatureGates(unittest.TestCase):
    def run_matrix(self, *arguments, fail_kind="", fail_features=None):
        # Source the actual script with command observers. No Rust build, Pi or
        # network request is needed to verify the selected commands and failures.
        wrapper = r'''
record() {
  printf '%s\0' "$@" >> "$PI_RPC_GATE_LOG"
  printf '\n' >> "$PI_RPC_GATE_LOG"
}
cargo() {
  record cargo "$@"
  local observed_features= previous= argument
  for argument in "$@"; do
    if [ "$previous" = --features ]; then observed_features="$argument"; break; fi
    previous="$argument"
  done
  if [ "${1-}" = "$PI_RPC_FAIL_KIND" ] &&
     { [ "$PI_RPC_FAIL_FEATURES" = '*' ] || [ "$observed_features" = "$PI_RPC_FAIL_FEATURES" ]; }; then
    return 23
  fi
}
python3() { record python3 "$@"; }
source "$0" "$@"
'''
        with tempfile.TemporaryDirectory() as work:
            log = Path(work) / "commands"
            environment = os.environ.copy()
            environment.update(PI_RPC_GATE_LOG=log.as_posix(), PI_RPC_FAIL_KIND=fail_kind,
                               PI_RPC_FAIL_FEATURES="*" if fail_features is None else fail_features)
            result = subprocess.run(
                [verification_bash(environment), "-c", wrapper,
                 (ROOT / "scripts/verify-protocols.sh").as_posix(), *arguments],
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

    def test_later_pi13_failures_stop_after_successful_prefix(self):
        for clippy in (False, True):
            arguments = ("--clippy",) if clippy else ()
            positive, complete = self.run_matrix(*arguments)
            self.assertEqual(positive.returncode, 0, positive.stderr)
            self.assert_matrix(complete, clippy=clippy)
            for kind in ("check", "test", "clippy") if clippy else ("check", "test"):
                with self.subTest(clippy=clippy, kind=kind):
                    stop = next(index for index, call in enumerate(complete)
                                if call[:2] == ["cargo", kind] and "--features" in call
                                and call[call.index("--features") + 1] == "pi_1_3")
                    self.assertGreater(stop, 0)
                    self.assertTrue(all("--features" not in call for call in complete[:2 + clippy]))
                    result, calls = self.run_matrix(*arguments, fail_kind=kind, fail_features="pi_1_3")
                    self.assertEqual(result.returncode, 23, result.stderr)
                    self.assertEqual(calls, complete[:stop + 1], "commands continued or preceding successes changed")
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

    def run_entrypoint(self, name, fail_readme=False):
        wrapper = r'''
record() {
  printf '%s\0' "$@" >> "$PI_RPC_GATE_LOG"
  printf '\n' >> "$PI_RPC_GATE_LOG"
}
cargo() { record cargo "$@"; }
python3() {
  record python3 "$@"
  if [ "${1-}" = scripts/test_readme.py ] && [ "$PI_RPC_FAIL_README" = 1 ]; then return 23; fi
}
rustc() { record rustc "$@"; printf 'rustc 1.88.0 (command observer)\n'; }
source "$0"
'''
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            script = root / "scripts" / name
            script.parent.mkdir()
            script.write_bytes((ROOT / "scripts" / name).read_bytes())
            (script.parent / "verify-protocols.sh").write_text(
                'printf \'%s\\0\' protocol-matrix "$@" >> "$PI_RPC_GATE_LOG"\n'
                'printf \'\\n\' >> "$PI_RPC_GATE_LOG"\n'
                'printf \'%s\\0\' selected-shell "$PI_RPC_TEST_BASH" >> "$PI_RPC_GATE_LOG"\n'
                'printf \'\\n\' >> "$PI_RPC_GATE_LOG"\n', encoding="utf-8", newline="\n")
            log = root / "commands"
            environment = os.environ.copy()
            environment["PI_RPC_GATE_LOG"] = log.as_posix()
            environment["PI_RPC_FAIL_README"] = "1" if fail_readme else "0"
            result = subprocess.run([verification_bash(environment), "-c", wrapper, script.as_posix()],
                                    env=environment, capture_output=True, encoding="utf-8", errors="replace")
            calls = [line.decode().split("\0")[:-1] for line in log.read_bytes().splitlines()] if log.exists() else []
        return result, calls

    def assert_entrypoint(self, calls, clippy):
        self.assertIn(["python3", "scripts/test_readme.py"], calls)
        cargo = ["cargo", *([] if clippy else ["+1.88.0"])]
        for features in ["subprocess,pi_1_4", "pi_1_5"]:
            flags = ["--locked", "--no-default-features", "--features", features]
            self.assertIn([*cargo, "test", *flags, "--doc"], calls)
            self.assertIn([*cargo, "doc", *flags, "--no-deps"], calls)
        self.assertIn(["protocol-matrix", *(["--clippy"] if clippy else [])], calls)
        selected = [call[1] for call in calls if call[0] == "selected-shell"]
        self.assertEqual(len(selected), 1)
        verification_bash({"PI_RPC_TEST_BASH": selected[0]})
        if clippy:
            self.assertIn(["python3", "scripts/check-format.py"], calls)
            self.assertIn(["python3", "scripts/test_gates.py"], calls)
        else:
            self.assertIn(["python3", "scripts/test_gates.py",
                           "FeatureGates.test_windows_observer_follows_enabled_protocols",
                           "FeatureGates.test_later_pi13_failures_stop_after_successful_prefix",
                           "FeatureGates.test_readme_failure_stops_both_entrypoints",
                           "ShellSelection"], calls)

    def test_stable_and_msrv_entrypoints_keep_the_required_checks(self):
        for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
            with self.subTest(name=name):
                result, calls = self.run_entrypoint(name)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                self.assert_entrypoint(calls, clippy)

    def test_readme_failure_stops_both_entrypoints(self):
        for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
            with self.subTest(name=name):
                positive, complete = self.run_entrypoint(name)
                self.assertEqual(positive.returncode, 0, positive.stdout + positive.stderr)
                self.assert_entrypoint(complete, clippy)
                stop = complete.index(["python3", "scripts/test_readme.py"])
                result, calls = self.run_entrypoint(name, fail_readme=True)
                self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
                self.assertEqual(calls, complete[:stop + 1], "verification continued after the README failure")


class ShellSelection(unittest.TestCase):
    @staticmethod
    def windows_text_writer(writer):
        def write(path, data, encoding=None, errors=None, newline=None):
            return writer(path, data, encoding=encoding, errors=errors,
                          newline="\r\n" if newline is None else newline)
        return write

    def assert_lf_shell_scripts(self, command, name):
        entrypoint = Path(command[-1])
        self.assertEqual(entrypoint.name, name)
        self.assertNotEqual(entrypoint.parent.resolve(), (ROOT / "scripts").resolve())
        scripts = sorted(entrypoint.parent.glob("*.sh"))
        self.assertEqual({script.name for script in scripts}, {name, "verify-protocols.sh"})
        for script in scripts:
            data = script.read_bytes()
            self.assertIn(b"\n", data)
            self.assertNotIn(b"\r", data, f"CRLF in temporary Shell script: {script.name}")
            self.assertTrue(data.endswith(b"\n"))
        return [script.name for script in scripts]

    def test_temporary_shell_scripts_keep_lf_under_windows_text_translation(self):
        write_text = Path.write_text
        run = subprocess.run
        gates = FeatureGates()
        for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
            with self.subTest(name=name), tempfile.TemporaryDirectory() as work:
                observed = []

                def checked_launch(command, *arguments, **options):
                    observed.extend(self.assert_lf_shell_scripts(command, name))
                    return run(command, *arguments, **options)

                with patch.object(Path, "write_text", self.windows_text_writer(write_text)):
                    control = Path(work) / "platform-default.txt"
                    control.write_text("first\nsecond\n", encoding="utf-8")
                    self.assertEqual(control.read_bytes(), b"first\r\nsecond\r\n",
                                     "Windows text translation control was inactive")
                    with patch("subprocess.run", checked_launch):
                        result, calls = gates.run_entrypoint(name)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                gates.assert_entrypoint(calls, clippy)
                self.assertEqual(set(observed), {name, "verify-protocols.sh"})

    def test_crlf_corruption_is_rejected_before_shell_execution(self):
        gates = FeatureGates()
        for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
            with self.subTest(name=name):
                result, calls = gates.run_entrypoint(name)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                gates.assert_entrypoint(calls, clippy)
                corrupted = []

                def corrupt_and_check(command, *arguments, **options):
                    entrypoint = Path(command[-1])
                    self.assertNotEqual(entrypoint.parent.resolve(), (ROOT / "scripts").resolve())
                    stub = entrypoint.parent / "verify-protocols.sh"
                    data = stub.read_bytes()
                    self.assertNotIn(b"\r", data)
                    stub.write_bytes(data.replace(b"\n", b"\r\n"))
                    self.assertIn(b"\r\n", stub.read_bytes())
                    corrupted.append(stub.name)
                    # Reject bytes before launch even if an interpreter is
                    # configured to tolerate CRLF or would exit successfully.
                    self.assert_lf_shell_scripts(command, name)
                    self.fail("CRLF fixture was accepted")

                with patch("subprocess.run", corrupt_and_check), \
                     self.assertRaisesRegex(AssertionError, "CRLF in temporary Shell script"):
                    gates.run_entrypoint(name)
                self.assertEqual(corrupted, ["verify-protocols.sh"])

    def test_selector_rejects_missing_relative_and_unusable_paths(self):
        with tempfile.TemporaryDirectory() as work:
            for value in (None, "", "bash", "relative/bash.exe", str(Path(work) / "missing"), work):
                with self.subTest(value=value):
                    environment = os.environ.copy()
                    environment.pop("PI_RPC_TEST_BASH", None)
                    if value is not None:
                        environment["PI_RPC_TEST_BASH"] = value
                    with patch.dict(os.environ, environment, clear=True), patch("subprocess.run") as launch:
                        with self.assertRaisesRegex(ValueError, "PI_RPC_TEST_BASH"):
                            FeatureGates().run_matrix("--clippy")
                        launch.assert_not_called()

    def test_entrypoint_shell_wins_over_wsl_on_path(self):
        selected = verification_bash()
        with tempfile.TemporaryDirectory() as work:
            root = Path(work)
            shadow = root / "WSL installation"
            shadow.mkdir()
            source = root / "wrong_bash.rs"
            source.write_text('fn main() { let marker = std::env::var_os("PI_RPC_WRONG_BASH_MARKER").unwrap(); '
                              'std::fs::write(marker, b"wrong-bash").unwrap(); std::process::exit(97); }\n',
                              encoding="utf-8")
            executable = shadow / ("bash.exe" if os.name == "nt" else "bash")
            built = subprocess.run(["rustc", "--edition", "2024", str(source), "-o", str(executable)],
                                   capture_output=True, encoding="utf-8", errors="replace")
            self.assertEqual(built.returncode, 0, built.stderr)
            marker = root / "wrong-shell-used"
            environment = {"PI_RPC_TEST_BASH": selected, "PI_RPC_WRONG_BASH_MARKER": str(marker),
                           "PATH": str(shadow) + os.pathsep + os.environ["PATH"]}
            with patch.dict(os.environ, environment):
                gates = FeatureGates()
                result, calls = gates.run_matrix("--clippy")
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                gates.assert_matrix(calls, clippy=True)
                for name, clippy in [("verify.sh", True), ("verify-msrv.sh", False)]:
                    result, calls = gates.run_entrypoint(name)
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                    gates.assert_entrypoint(calls, clippy)
            self.assertFalse(marker.exists(), "a PATH-selected WSL Bash was executed")

    def test_entrypoints_export_native_current_shell(self):
        selected = verification_bash()
        wrapper = r'''
cygpath() { printf '%s\n' "$PI_RPC_CYGPATH_RESULT"; }
python3() { printf '%s' "$PI_RPC_TEST_BASH" > "$PI_RPC_GATE_LOG"; return 23; }
rustc() { printf 'rustc 1.88.0 (command observer)\n'; }
source "$0"
'''
        for name in ("verify.sh", "verify-msrv.sh"):
            for suffix in ("", ".exe", ".EXE"):
                with self.subTest(name=name, suffix=suffix), tempfile.TemporaryDirectory() as work:
                    root = Path(work)
                    script = root / "scripts" / name
                    script.parent.mkdir()
                    script.write_bytes((ROOT / "scripts" / name).read_bytes())
                    native = "C:/Program Files/Git/usr/bin/bash" + suffix
                    log = root / "exported-shell"
                    environment = os.environ.copy()
                    environment.update(PI_RPC_TEST_BASH="wrong-inherited-shell", PI_RPC_GATE_LOG=log.as_posix(),
                                       PI_RPC_CYGPATH_RESULT=native)
                    result = subprocess.run([selected, "-c", wrapper, script.as_posix()], env=environment,
                                            capture_output=True, encoding="utf-8", errors="replace")
                    self.assertEqual(result.returncode, 23, result.stdout + result.stderr)
                    self.assertEqual(log.read_text(), native if suffix else native + ".exe")


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
