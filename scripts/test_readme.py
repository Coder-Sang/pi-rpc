#!/usr/bin/env python3
"""Compile the copied README and reject regressions; never execute its Pi call."""
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import tomllib
import unittest

sys.dont_write_bytecode = True
from prepare_consumer import cargo_package_location
from readme_consumer import readme_consumer_sources

ROOT = Path(__file__).resolve().parent.parent


class ReadmeConsumer(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.temporary = tempfile.TemporaryDirectory(prefix="pi-rpc-readme-")
        cls.addClassCleanup(cls.temporary.cleanup)
        cls.root = Path(cls.temporary.name) / "consumer with spaces"
        cls.root.mkdir()
        cls.readme = (ROOT / "README.md").read_text(encoding="utf-8")
        package, target = cargo_package_location(ROOT)
        cls.version = package["version"]
        cls.environment = os.environ.copy()
        cls.environment["CARGO_TARGET_DIR"] = str(target / "readme-consumer")
        cls.write_sources(cls.readme)
        result = cls.cargo("generate-lockfile")
        if result.returncode:
            raise AssertionError(result.stdout + result.stderr)

    @classmethod
    def cargo(cls, command):
        arguments = ["cargo", command, "--manifest-path", str(cls.root / "Cargo.toml")]
        if command == "check":
            arguments.append("--locked")
        return subprocess.run(arguments, env=cls.environment, capture_output=True,
                              encoding="utf-8", errors="replace")

    @classmethod
    def write_sources(cls, readme):
        sources = readme_consumer_sources(readme, ROOT, cls.version)
        for name, content in sources.items():
            path = cls.root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(content, encoding="utf-8", newline="\n")
        return sources

    def check(self, readme):
        sources = self.write_sources(readme)
        self.assertNotIn("features", tomllib.loads(sources["Cargo.toml"]))
        self.assertIn("fn main() { let _ = inspect_session; }", sources["src/main.rs"])
        return self.cargo("check")

    def assert_compiles(self, readme):
        result = self.check(readme)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def test_compiles_without_local_features(self):
        self.assert_compiles(self.readme)

    def test_original_local_feature_gate_is_rejected(self):
        self.assert_compiles(self.readme)
        gate = '#[cfg(all(feature = "subprocess", feature = "pi_1_5"))]\n'
        self.assertIn("async fn inspect_session", self.readme)
        mutated = self.readme.replace("async fn inspect_session", gate + "async fn inspect_session", 1)
        result = self.check(mutated)
        self.assertNotEqual(result.returncode, 0, "local cfg silently removed the copied example")
        self.assertIn("E0425", result.stderr)
        self.assertIn("inspect_session", result.stderr)

    def test_example_body_is_checked(self):
        self.assert_compiles(self.readme)
        call = "PiProcess::<Pi15>::spawn("
        self.assertIn(call, self.readme)
        result = self.check(self.readme.replace(call, "PiProcess::<Pi15>::missing_readme_method(", 1))
        self.assertNotEqual(result.returncode, 0, "the example body was not type checked")
        self.assertIn("E0599", result.stderr)
        self.assertNotIn("E0425", result.stderr)

    def test_dependency_features_are_honored(self):
        self.assert_compiles(self.readme)
        dependency = 'pi-rpc = { path = "../pi-rpc" }'
        self.assertIn(dependency, self.readme)
        for protocol, success in [("pi_1_5", True), ("pi_1_4", False)]:
            with self.subTest(protocol=protocol):
                configured = 'pi-rpc = { path = "../pi-rpc", default-features = false, features = ["subprocess", "' + protocol + '"] }'
                result = self.check(self.readme.replace(dependency, configured, 1))
                if success:
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                else:
                    self.assertNotEqual(result.returncode, 0, "documented dependency features were overridden")
                    self.assertIn("E0432", result.stderr)
                    self.assertIn("Pi15", result.stderr)

    def test_documented_version_is_honored(self):
        self.assert_compiles(self.readme)
        dependency = 'pi-rpc = { path = "../pi-rpc" }'
        self.assertIn(dependency, self.readme)
        for version, success in [(self.version, True), ("0.2.2", False)]:
            with self.subTest(version=version):
                configured = 'pi-rpc = { path = "../pi-rpc", version = "' + version + '" }'
                result = self.check(self.readme.replace(dependency, configured, 1))
                if success:
                    self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                else:
                    self.assertNotEqual(result.returncode, 0, "documented dependency version was overridden")
                    self.assertIn("failed to select a version", result.stderr)
                    self.assertIn("0.2.2", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
