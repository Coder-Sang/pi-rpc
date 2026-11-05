#!/usr/bin/env python3
"""Compile the copied README and reject regressions; never execute its Pi call."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys
import tempfile
import tomllib
import unittest

sys.dont_write_bytecode = True
from prepare_consumer import cargo_package_location
from readme_consumer import readme_consumer_sources, toml_dependencies

ROOT = Path(__file__).resolve().parent.parent


def dependency_example(readme):
    matches = list(re.finditer(r"^```toml\s*\n(.*?)^```\s*$", readme, re.MULTILINE | re.DOTALL))
    if len(matches) != 1:
        raise AssertionError("expected one README dependency example")
    document = tomllib.loads(matches[0][1])
    if set(document) != {"dependencies"} or "pi-rpc" not in document["dependencies"]:
        raise AssertionError("expected a standalone pi-rpc dependency example")
    return matches[0], document["dependencies"]


def dependency_variant(readme, specification, declaration=None):
    block, dependencies = dependency_example(readme)
    expected = dependencies | {"pi-rpc": specification}
    if declaration is None:
        text = toml_dependencies(expected)
    else:
        text = toml_dependencies({name: value for name, value in dependencies.items() if name != "pi-rpc"})
        text += declaration + "\n"
    if tomllib.loads(text) != {"dependencies": expected}:
        raise AssertionError("dependency fixture changes unintended TOML values")
    return readme[:block.start(1)] + text + readme[block.end(1):]


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
        generated = tomllib.loads(sources["Cargo.toml"])
        self.assertNotIn("features", generated)
        _, documented = dependency_example(readme)
        self.assertEqual({name: value for name, value in generated["dependencies"].items() if name != "pi-rpc"},
                         {name: value for name, value in documented.items() if name != "pi-rpc"})
        self.assertIn("fn main() { let _ = inspect_session; }", sources["src/main.rs"])
        return self.cargo("check")

    def assert_compiles(self, readme):
        result = self.check(readme)
        self.assertEqual(result.returncode, 0, result.stdout + result.stderr)

    def forms(self):
        yield "documented", self.readme
        path = {"path": "../pi-rpc"}
        version = json.dumps(self.version)
        fixtures = [
            ("path", path, 'pi-rpc = { path = "../pi-rpc" }'),
            ("version_string", self.version, 'pi-rpc = ' + version),
            ("version_table", {"version": self.version}, 'pi-rpc = { version = ' + version + ' }'),
            ("compact_path", path, 'pi-rpc={path="../pi-rpc"}'),
            ("literal_quotes", path, "'pi-rpc' = {path='../pi-rpc'} # SDK"),
            ("indented_reordered", path | {"version": self.version},
             '  "pi-rpc"\t= { version = ' + version + ', path = "../pi-rpc" } # SDK'),
            ("dependency_subtable", path | {"version": self.version},
             '[dependencies."pi-rpc"]\nversion = ' + version + '\npath = "../pi-rpc"'),
        ]
        for name, specification, declaration in fixtures:
            yield name, dependency_variant(self.readme, specification, declaration)
        yield "crlf", dependency_variant(self.readme, path).replace("\n", "\r\n")

    def options(self, readme, updates):
        _, dependencies = dependency_example(readme)
        original = dependencies["pi-rpc"]
        options = dict(original) if isinstance(original, dict) else {"version": original}
        return dependency_variant(readme, options | updates)

    def record_form(self, name):
        print(f"README dependency form: {self._testMethodName} / {name}", flush=True)

    def test_compiles_without_local_features(self):
        for name, readme in self.forms():
            with self.subTest(form=name):
                self.record_form(name)
                self.assert_compiles(readme)

    def test_original_local_feature_gate_is_rejected(self):
        for name, readme in self.forms():
            with self.subTest(form=name):
                self.record_form(name)
                self.assert_compiles(readme)
                gate = '#[cfg(all(feature = "subprocess", feature = "pi_1_5"))]\n'
                self.assertIn("async fn inspect_session", readme)
                result = self.check(readme.replace("async fn inspect_session", gate + "async fn inspect_session", 1))
                self.assertNotEqual(result.returncode, 0, "local cfg silently removed the copied example")
                self.assertIn("E0425", result.stderr)
                self.assertIn("inspect_session", result.stderr)

    def test_example_body_is_checked(self):
        for name, readme in self.forms():
            with self.subTest(form=name):
                self.record_form(name)
                self.assert_compiles(readme)
                call = "PiProcess::<Pi15>::spawn("
                self.assertIn(call, readme)
                result = self.check(readme.replace(call, "PiProcess::<Pi15>::missing_readme_method(", 1))
                self.assertNotEqual(result.returncode, 0, "the example body was not type checked")
                self.assertIn("E0599", result.stderr)
                self.assertNotIn("E0425", result.stderr)

    def test_dependency_features_are_honored(self):
        for name, readme in self.forms():
            with self.subTest(form=name):
                self.record_form(name)
                self.assert_compiles(readme)
                for protocol, success in [("pi_1_5", True), ("pi_1_4", False)]:
                    with self.subTest(protocol=protocol):
                        configured = self.options(readme, {"default-features": False, "features": ["subprocess", protocol]})
                        result = self.check(configured)
                        if success:
                            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        else:
                            self.assertNotEqual(result.returncode, 0, "documented dependency features were overridden")
                            self.assertIn("E0432", result.stderr)
                            self.assertIn("Pi15", result.stderr)

    def test_documented_version_is_honored(self):
        for name, readme in self.forms():
            with self.subTest(form=name):
                self.record_form(name)
                self.assert_compiles(readme)
                for version, success in [(self.version, True), ("0.2.2", False)]:
                    with self.subTest(version=version):
                        result = self.check(self.options(readme, {"version": version}))
                        if success:
                            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                        else:
                            self.assertNotEqual(result.returncode, 0, "documented dependency version was overridden")
                            self.assertIn("failed to select a version", result.stderr)
                            self.assertIn("0.2.2", result.stderr)


if __name__ == "__main__":
    unittest.main(verbosity=2)
