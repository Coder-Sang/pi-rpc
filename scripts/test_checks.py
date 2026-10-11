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
from validation_checks import CheckError, REQUIRED_FILES, check_documents, check_package, read_archive, unwanted_package_file

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
        manifest = (b'[package]\nname="pi-rpc"\nversion="0.1.0"\nlicense="Apache-2.0"\npublish=false\n'
                    b'edition="2024"\nrust-version="1.85"\nreadme="README.md"\n'
                    b'repository="https://github.com/Coder-Sang/pi-rpc"\n'
                    b'keywords=["pi","rpc","jsonl","tokio","async"]\n'
                    b'[package.metadata.docs.rs]\nall-features=true\n')
        self.files.update({"Cargo.toml": manifest, "Cargo.toml.orig": manifest,
                           ".gitignore": (ROOT / ".gitignore").read_bytes(),
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

    def test_required_metadata_missing_wrong_or_wrong_type(self):
        mutations = [
            (b'edition="2024"\n', [b'', b'edition="2021"\n', b'edition=2024\n']),
            (b'rust-version="1.85"\n', [b'', b'rust-version="1.88"\n', b'rust-version=1.85\n']),
            (b'readme="README.md"\n', [b'', b'readme="missing.md"\n', b'readme=false\n']),
            (b'repository="https://github.com/Coder-Sang/pi-rpc"\n',
             [b'', b'repository="https://example.invalid"\n', b'repository=false\n']),
            (b'keywords=["pi","rpc","jsonl","tokio","async"]\n',
             [b'', b'keywords=["pi"]\n', b'keywords="pi"\n']),
            (b'all-features=true\n', [b'', b'all-features=false\n', b'all-features=1\n']),
            (b'[package.metadata.docs.rs]\nall-features=true\n',
             [b'', b'metadata="invalid"\n', b'[package.metadata]\ndocs=false\n',
              b'[package.metadata.docs]\nrs=[]\n']),
        ]
        root = self.source_root()
        for name in ["Cargo.toml", "Cargo.toml.orig"]:
            saved = self.files[name]
            for old, replacements in mutations:
                for replacement in replacements:
                    with self.subTest(name=name, old=old, replacement=replacement):
                        self.files[name] = saved.replace(old, replacement)
                        self.write_archive()
                        for options in [{}, {"source_root": root, "expected_commit": "expected"}]:
                            with self.assertRaises(CheckError):
                                check_package(self.archive, **options)
            self.files[name] = saved

    def test_manifest_versions_must_agree(self):
        self.files["Cargo.toml.orig"] = self.files["Cargo.toml.orig"].replace(b'version="0.1.0"', b'version="0.2.0"')
        self.write_archive()
        with self.assertRaisesRegex(CheckError, "versions differ"):
            check_package(self.archive)

    def test_package_table_and_version_required(self):
        for name in ["Cargo.toml", "Cargo.toml.orig"]:
            saved = self.files[name]
            for replacement in [b'package=false\n', b'[other]\n',
                                saved.replace(b'version="0.1.0"\n', b''),
                                saved.replace(b'version="0.1.0"', b'version=1')]:
                with self.subTest(name=name, replacement=replacement):
                    self.files[name] = replacement
                    self.write_archive()
                    with self.assertRaises(CheckError):
                        check_package(self.archive)
            self.files[name] = saved

    def source_root(self):
        root = Path(self.temp.name) / "source"
        root.mkdir(exist_ok=True)
        for name in ["Cargo.toml", "Cargo.lock", "LICENSE", "README.md", ".gitignore", *sorted(n for n in self.files if n.startswith("docs/"))]:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(self.files[name])
        return root

    def test_source_mismatch(self):
        root = self.source_root()
        self.write_archive()
        for name in ["README.md", ".gitignore"]:
            with self.subTest(name=name):
                path = root / name
                saved = path.read_bytes()
                path.write_text("changed", encoding="utf-8")
                with self.assertRaisesRegex(CheckError, "differs from source"):
                    check_package(self.archive, source_root=root)
                path.write_bytes(saved)

    def test_missing_source_files(self):
        root = self.source_root()
        self.write_archive()
        check_package(self.archive, source_root=root)
        for name in ["LICENSE", "Cargo.lock", "README.md", ".gitignore", "docs/verification.md"]:
            with self.subTest(name=name):
                path = root / name
                saved = path.read_bytes()
                path.unlink()
                with self.assertRaisesRegex(CheckError, "missing source comparison files"):
                    check_package(self.archive, source_root=root)
                path.write_bytes(saved)

    def test_bidirectional_docs_inventory(self):
        root = self.source_root()
        self.files["docs/extra.md"] = b"# Extra\n"
        self.write_archive()
        with self.assertRaisesRegex(CheckError, "docs inventory differs"):
            check_package(self.archive, source_root=root)
        (root / "docs/extra.md").write_bytes(self.files["docs/extra.md"])
        check_package(self.archive, source_root=root)
        del self.files["docs/extra.md"]
        self.write_archive()
        with self.assertRaisesRegex(CheckError, "docs inventory differs"):
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

    def test_unwanted_files_rejected_at_root_and_in_subdirectories(self):
        names = [".env", ".env.local", ".env.production", ".env.example.bak", ".DS_Store", "Thumbs.db",
                 "desktop.ini", "Desktop.ini", "file.swp", "file.swo", "file~", "file.pyc",
                 "auth.json", "settings.json", "certificate.pem", "private.key"]
        names += ["target/output", ".git/config", ".pi/config", "__pycache__/cache", "nested/__pycache__/cache",
                  ".env.local/.env.example", "nested/.env.production/data"]
        for name in [*names, *("nested/" + name for name in names[:16])]:
            with self.subTest(name=name):
                self.files[name] = b"harmless fixture"
                self.write_archive()
                with self.assertRaisesRegex(CheckError, "unexpected sensitive/build file"):
                    check_package(self.archive)
                del self.files[name]

    def test_env_example_and_legitimate_json_files_preserved(self):
        for name in [".env.example", "nested/.env.example", "nested/target/data", "tests/fixture.json"]:
            self.files[name] = b"harmless fixture"
        self.write_archive()
        check_package(self.archive)

    def test_ignore_policy_drift_rejected(self):
        saved = self.files[".gitignore"]
        reordered = saved.replace(b'!.env.example\n', b'').replace(b'.env.*\n', b'!.env.example\n.env.*\n')
        for data in [saved.replace(b'.env.*\n', b''), saved.replace(b'!.env.example\n', b''), saved + b'*.json\n', reordered]:
            with self.subTest(data=data):
                self.files[".gitignore"] = data
                self.write_archive()
                with self.assertRaisesRegex(CheckError, "ignore policy"):
                    check_package(self.archive)
        self.files[".gitignore"] = saved

    def test_ignore_policy_leading_space_and_tab_rejected(self):
        saved = self.files[".gitignore"]
        for rule in saved.splitlines():
            for prefix in [b" ", b"\t"]:
                with self.subTest(rule=rule, prefix=prefix):
                    self.files[".gitignore"] = saved.replace(rule + b"\n", prefix + rule + b"\n")
                    self.write_archive()
                    with self.assertRaisesRegex(CheckError, "ignore policy"):
                        check_package(self.archive)
        self.files[".gitignore"] = saved

    def test_ignore_policy_indented_hash_is_not_comment(self):
        saved = self.files[".gitignore"]
        for prefix in [b" ", b"\t"]:
            with self.subTest(prefix=prefix):
                self.files[".gitignore"] = saved + prefix + b"#literal-pattern\n"
                self.write_archive()
                with self.assertRaisesRegex(CheckError, "ignore policy"):
                    check_package(self.archive)
        self.files[".gitignore"] = saved

    def test_ignore_policy_comments_blank_lines_and_crlf(self):
        self.files[".gitignore"] = (b"# Policy comment\n\n" + self.files[".gitignore"]).replace(b"\n", b"\r\n")
        self.write_archive()
        check_package(self.archive)

    def test_ignore_policy_matches_git(self):
        root = Path(self.temp.name) / "git-policy"
        root.mkdir()
        subprocess.run(["git", "init", "--quiet", "--template=", str(root)], check=True, capture_output=True)
        subprocess.run(["git", "-C", str(root), "config", "core.ignorecase", "false"], check=True)
        excludes = root / "empty-excludes"
        excludes.write_bytes(b"")
        subprocess.run(["git", "-C", str(root), "config", "core.excludesFile", str(excludes)], check=True)
        (root / ".gitignore").write_bytes(self.files[".gitignore"])
        names = [".env", ".env.local", "nested/.env.local", ".env.example", "nested/.env.example",
                 ".env.example.bak", ".env.local/.env.example", "nested/.env.production/data",
                 ".DS_Store", "nested/.DS_Store", "Thumbs.db", "desktop.ini", "Desktop.ini",
                 "a.swp", "a.swo", "a~", "nested/auth.json", "settings.json", "a.pem", "a.key",
                 "target/output", "nested/target/output", ".pi/config", "nested/__pycache__/a.pyc",
                 "target", "__pycache__", "Cargo.lock", "tests/fixture.json", "README.md",
                 "nested/line\nbreak.key", 'nested/quoted"name.pem', "nested/中文.key", "nested/中文.json"]
        # NUL-delimited bytes avoid Windows newline translation and Git path quoting.
        result = subprocess.run(["git", "-C", str(root), "check-ignore", "--no-index", "--stdin", "-z"],
                                input=("\0".join(names) + "\0").encode("utf-8"), capture_output=True, check=True)
        ignored = set(result.stdout.decode("utf-8").rstrip("\0").split("\0"))
        for name in names:
            with self.subTest(name=name):
                self.assertEqual(unwanted_package_file(name), name in ignored)


if __name__ == "__main__":
    unittest.main(verbosity=2)
