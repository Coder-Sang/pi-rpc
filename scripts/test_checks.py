#!/usr/bin/env python3
"""Regression tests for release-check failure paths, not SDK acceptance tests."""
import io
import json
import os
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import unittest

sys.dont_write_bytecode = True
from validation_checks import CheckError, REQUIRED_FILES, check_documents, check_package, read_archive

ROOT = Path(__file__).resolve().parent.parent


class DocumentChecks(unittest.TestCase):
    def test_duplicate_anchors_and_fenced_prose(self):
        files = {"README.md": b"# A\n# A\n[second](#a-1)\n```rust\n[not prose](missing)\n```\n"}
        self.assertEqual(check_documents(files), (1, 1))

    def test_missing_target_anchor_escape_and_unclosed_fence(self):
        for text in ["[missing](absent.md)", "[bad](#absent)", "[escape](../README.md)", "```rust\nunclosed"]:
            with self.subTest(text=text), self.assertRaises(CheckError):
                check_documents({"README.md": text.encode()})

    def test_explicit_root_cli_ignores_cwd_and_git(self):
        with tempfile.TemporaryDirectory(dir=os.environ.get("JCODE_SCRATCH_DIR")) as tmp:
            root = Path(tmp) / "package"
            root.mkdir()
            (root / "README.md").write_text("[license](LICENSE)\n", encoding="utf-8")
            (root / "LICENSE").write_text("fixture", encoding="utf-8")
            command = [sys.executable, str(ROOT / "scripts/check-docs.py"), str(root)]
            passed = subprocess.run(command, cwd=tmp, capture_output=True, text=True)
            self.assertEqual(passed.returncode, 0, passed.stderr)
            self.assertIn(f"root={root.resolve()}", passed.stdout)
            (root / "LICENSE").unlink()
            failed = subprocess.run(command, cwd=ROOT, capture_output=True, text=True)
            self.assertEqual(failed.returncode, 1)
            self.assertIn("missing local link target", failed.stderr)


class PackageChecks(unittest.TestCase):
    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=os.environ.get("JCODE_SCRATCH_DIR"))
        self.addCleanup(self.temp.cleanup)
        self.archive = Path(self.temp.name) / "fixture.crate"
        self.files = {name: b"" for name in REQUIRED_FILES}
        manifest = b'[package]\nname="pi-rpc"\nversion="0.1.0"\nlicense="Apache-2.0"\npublish=false\n'
        self.files.update({"Cargo.toml": manifest, "Cargo.toml.orig": manifest,
                           "LICENSE": (ROOT / "LICENSE").read_bytes(),
                           "README.md": b"[license](LICENSE)\n[docs](docs/compatibility.md#scope)\n",
                           "docs/compatibility.md": b"# Scope\n",
                           ".cargo_vcs_info.json": json.dumps({"git": {"sha1": "expected"}}).encode()})

    def write_archive(self, extras=()):
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, data in self.files.items():
                info = tarfile.TarInfo("pi-rpc-0.1.0/" + name)
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            for info, data in extras:
                archive.addfile(info, io.BytesIO(data) if data is not None else None)

    def test_complete_package(self):
        self.write_archive()
        self.assertEqual(check_package(self.archive, expected_commit="expected")[1:], (4, 2))

    def test_missing_archive_target_never_falls_back_to_worktree(self):
        del self.files["LICENSE"]
        self.write_archive()
        with self.assertRaises(CheckError):
            check_package(self.archive)

    def test_changed_license_and_metadata(self):
        for name, replacement in [("LICENSE", b"changed"),
                                  ("Cargo.toml", self.files["Cargo.toml"].replace(b'Apache-2.0', b'MIT')),
                                  ("Cargo.toml.orig", self.files["Cargo.toml.orig"].replace(b'false', b'true'))]:
            with self.subTest(name=name):
                saved = self.files[name]
                self.files[name] = replacement
                self.write_archive()
                with self.assertRaises(CheckError):
                    check_package(self.archive)
                self.files[name] = saved

    def test_vcs_mismatch_and_dirty(self):
        for git in [{"sha1": "other"}, {"sha1": "expected", "dirty": True}]:
            self.files[".cargo_vcs_info.json"] = json.dumps({"git": git}).encode()
            self.write_archive()
            with self.assertRaises(CheckError):
                check_package(self.archive, expected_commit="expected")

    def test_source_mismatch(self):
        root = Path(self.temp.name) / "source"
        root.mkdir()
        (root / "README.md").write_text("changed", encoding="utf-8")
        self.write_archive()
        with self.assertRaises(CheckError):
            check_package(self.archive, source_root=root)

    def test_symlink_traversal_duplicate_and_sensitive_file(self):
        for name, kind in [("pi-rpc-0.1.0/link", tarfile.SYMTYPE),
                           ("pi-rpc-0.1.0/../escape", tarfile.REGTYPE),
                           ("pi-rpc-0.1.0/README.md", tarfile.REGTYPE)]:
            with self.subTest(name=name):
                entry = tarfile.TarInfo(name)
                entry.type = kind
                entry.linkname = "/outside"
                self.write_archive([(entry, b"" if kind == tarfile.REGTYPE else None)])
                with self.assertRaises(CheckError):
                    read_archive(self.archive)
        self.files["auth.json"] = b"fixture only"
        self.write_archive()
        with self.assertRaises(CheckError):
            check_package(self.archive)


if __name__ == "__main__":
    unittest.main(verbosity=2)
