#!/usr/bin/env python3
"""Regression tests for release-check failure paths, not SDK acceptance tests."""
import io
import json
import os
from contextlib import redirect_stderr
from pathlib import Path
import subprocess
import sys
import tarfile
import tempfile
import tomllib
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
from validation_checks import CheckError, PACKAGE_METADATA, REQUIRED_FILES, check_documents, check_package, read_archive, unwanted_package_file
from prepare_consumer import prepare_consumer

ROOT = Path(__file__).resolve().parent.parent


class DocumentChecks(unittest.TestCase):
    def test_duplicate_anchors_and_fenced_prose(self):
        files = {"README.md": b"# A\n# A\n[second](#a-1)\n```rust\n[not prose](missing)\n```\n"}
        self.assertEqual(check_documents(files), (1, 1))

    def test_missing_target_anchor_escape_and_unclosed_fence(self):
        for text in ["[missing](absent.md)", "[bad](#absent)", "[escape](../README.md)", "```rust\nunclosed"]:
            with self.subTest(text=text), self.assertRaises(CheckError):
                check_documents({"README.md": text.encode()})

    def test_release_documents_local_links_checked(self):
        for name in ["CHANGELOG.md", "THIRD_PARTY_NOTICES.md"]:
            with self.subTest(name=name):
                files = {"README.md": b"# Readme\n", name: b"[readme](README.md#readme)\n"}
                self.assertEqual(check_documents(files), (2, 1))
                files[name] = b"[missing](absent.md)\n"
                with self.assertRaisesRegex(CheckError, "missing local link target"):
                    check_documents(files)

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
    def test_repository_metadata_matches_the_0_2_release_policy(self):
        package = tomllib.loads((ROOT / "Cargo.toml").read_text(encoding="utf-8"))["package"]
        self.assertEqual(package["version"], "0.2.0")
        self.assertEqual(package["rust-version"], "1.88")
        for name, value in PACKAGE_METADATA.items():
            self.assertEqual(package[name], value, name)

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory(dir=os.environ.get("JCODE_SCRATCH_DIR"))
        self.addCleanup(self.temp.cleanup)
        self.archive = Path(self.temp.name) / "fixture.crate"
        self.files = {name: b"" for name in REQUIRED_FILES}
        manifest = (b'[package]\nname="pi-rpc"\nversion="0.2.0"\nlicense="Apache-2.0"\npublish=["crates-io"]\n'
                    b'description="Bounded asynchronous Tokio JSONL client for Pi 1.1.0 through 1.2.0"\n'
                    b'edition="2024"\nrust-version="1.88"\nreadme="README.md"\n'
                    b'repository="https://github.com/Coder-Sang/pi-rpc"\n'
                    b'keywords=["pi","rpc","jsonl","tokio","async"]\n'
                    b'[package.metadata.docs.rs]\nall-features=true\n')
        self.files.update({"Cargo.toml": manifest, "Cargo.toml.orig": manifest,
                           ".gitignore": (ROOT / ".gitignore").read_bytes(),
                           "LICENSE": (ROOT / "LICENSE").read_bytes(),
                           "README.md": b"[license](LICENSE)\n[docs](docs/compatibility.md#scope)\n",
                           "docs/compatibility.md": b"# Scope\n",
                           ".cargo_vcs_info.json": json.dumps({"git": {"sha1": "expected"}}).encode()})
        notice = (ROOT / "THIRD_PARTY_NOTICES.md").read_bytes().partition(b"```text\n")[2].partition(b"\n```")[0]
        self.files["THIRD_PARTY_NOTICES.md"] = b"```text\n" + notice + b"\n```\n"

    def write_archive(self, extras=()):
        with tarfile.open(self.archive, "w:gz") as archive:
            for name, data in self.files.items():
                info = tarfile.TarInfo("pi-rpc-0.2.0/" + name)
                info.size = len(data)
                archive.addfile(info, io.BytesIO(data))
            for info, data in extras:
                archive.addfile(info, io.BytesIO(data) if data is not None else None)

    def test_complete_package(self):
        self.write_archive()
        self.assertEqual(check_package(self.archive, expected_commit="expected")[1:], (7, 2))

    def test_missing_archive_target_never_falls_back_to_worktree(self):
        del self.files["LICENSE"]
        self.write_archive()
        with self.assertRaises(CheckError):
            check_package(self.archive)

    def test_release_materials_required_in_archive(self):
        for name in ["CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "docs/provenance.md", "scripts/verify-consumer.sh"]:
            saved = self.files.pop(name)
            with self.subTest(name=name):
                self.write_archive()
                with self.assertRaisesRegex(CheckError, "missing package files"):
                    check_package(self.archive)
            self.files[name] = saved

    def test_changed_license_and_metadata(self):
        for name, replacement in [("LICENSE", b"changed"),
                                  ("THIRD_PARTY_NOTICES.md", self.files["THIRD_PARTY_NOTICES.md"].replace(b'Mario Zechner', b'Changed attribution')),
                                  ("Cargo.toml", self.files["Cargo.toml"].replace(b'Apache-2.0', b'MIT')),
                                  ("Cargo.toml.orig", self.files["Cargo.toml.orig"].replace(b'publish=["crates-io"]', b'publish=true'))]:
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

    def test_publish_restriction_requires_only_crates_io_in_both_manifests(self):
        for name in ["Cargo.toml", "Cargo.toml.orig"]:
            saved = self.files[name]
            for replacement in [b'', b'publish=false\n', b'publish=true\n', b'publish=[]\n',
                                b'publish="crates-io"\n', b'publish=[false]\n',
                                b'publish=["private"]\n', b'publish=["crates-io","private"]\n',
                                b'publish=["crates-io","crates-io"]\n']:
                with self.subTest(name=name, replacement=replacement):
                    self.files[name] = saved.replace(b'publish=["crates-io"]\n', replacement)
                    self.write_archive()
                    with self.assertRaisesRegex(CheckError, "publish"):
                        check_package(self.archive)
            self.files[name] = saved

    def test_required_metadata_missing_wrong_or_wrong_type(self):
        mutations = [
            (b'edition="2024"\n', [b'', b'edition="2021"\n', b'edition=2024\n']),
            (b'rust-version="1.88"\n', [b'', b'rust-version="1.85"\n', b'rust-version="1.87"\n',
                                        b'rust-version="1.89"\n', b'rust-version=1.88\n']),
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
        self.files["Cargo.toml.orig"] = self.files["Cargo.toml.orig"].replace(b'version="0.2.0"', b'version="0.3.0"')
        self.write_archive()
        with self.assertRaisesRegex(CheckError, "versions differ"):
            check_package(self.archive)

    def test_description_required_nonblank_and_string_in_both_manifests(self):
        root = self.source_root()
        description = b'description="Bounded asynchronous Tokio JSONL client for Pi 1.1.0 through 1.2.0"\n'
        for name in ["Cargo.toml", "Cargo.toml.orig"]:
            saved = self.files[name]
            for replacement in [b'', b'description=""\n', b'description=" \\t "\n',
                                b'description=false\n', b'description=1\n', b'description=[]\n',
                                b'description={summary="x"}\n']:
                with self.subTest(name=name, replacement=replacement):
                    self.files[name] = saved.replace(description, replacement)
                    self.write_archive()
                    for options in [{}, {"source_root": root, "expected_commit": "expected"}]:
                        with self.assertRaisesRegex(CheckError, "description"):
                            check_package(self.archive, **options)
            self.files[name] = saved

    def test_manifest_descriptions_must_agree_but_allow_matching_updates(self):
        description = b'description="Bounded asynchronous Tokio JSONL client for Pi 1.1.0 through 1.2.0"'
        replacement = b'description="Updated nonempty package description"'
        self.files["Cargo.toml"] = self.files["Cargo.toml"].replace(description, replacement)
        self.write_archive()
        with self.assertRaisesRegex(CheckError, "descriptions differ"):
            check_package(self.archive)
        self.files["Cargo.toml.orig"] = self.files["Cargo.toml.orig"].replace(description, replacement)
        root = self.source_root()
        self.write_archive()
        check_package(self.archive, source_root=root, expected_commit="expected")

    def consumer_source_root(self):
        root = self.source_root()
        for name, data in self.files.items():
            if name in {"Cargo.toml", "Cargo.toml.orig", ".cargo_vcs_info.json"}:
                continue
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
        return root

    def consumer_process(self, root, target):
        def run(args, **kwargs):
            if args[0] == "cargo":
                self.assertIn("--locked", args)
                self.assertIn("--offline", args)
                self.assertEqual(Path(args[-1]).resolve(), (root / "Cargo.toml").resolve())
                metadata = {"packages": [{"name": "pi-rpc", "version": "0.2.0", "manifest_path": str(root / "Cargo.toml")}],
                            "target_directory": str(target)}
                return subprocess.CompletedProcess(args, 0, json.dumps(metadata), "")
            self.assertEqual(args, ["git", "-C", str(root.resolve()), "rev-parse", "HEAD"])
            return subprocess.CompletedProcess(args, 0, "expected\n", "")
        return run

    def prepare_fixture_consumer(self, root, target, destination, archive=None):
        with patch("prepare_consumer.subprocess.run", side_effect=self.consumer_process(root, target)), redirect_stderr(io.StringIO()):
            return prepare_consumer(root, destination, archive)

    def test_consumer_uses_cargo_target_directory_with_stale_default_package(self):
        root = self.consumer_source_root()
        stale = root / "target/package/pi-rpc-0.2.0"
        stale.mkdir(parents=True)
        (stale / "Cargo.toml").write_text('[package]\nname="pi-rpc"\nversion="0.0.9"\n', encoding="utf-8")
        for index, target in enumerate([Path(self.temp.name) / "env target", Path(self.temp.name) / "config target 中文"]):
            with self.subTest(target=target):
                target.joinpath("package").mkdir(parents=True)
                self.archive = target / "package/pi-rpc-0.2.0.crate"
                self.write_archive()
                destination = Path(self.temp.name) / f"consumer {index} 🚀"
                destination.mkdir()
                output = self.prepare_fixture_consumer(root, target, destination)
                self.assertEqual(output, target.resolve() / "independent-consumer")
                generated = tomllib.loads((destination / "consumer/Cargo.toml").read_text(encoding="utf-8"))
                dependency = generated["dependencies"]["pi-rpc"]
                self.assertEqual(dependency["version"], "=0.2.0")
                self.assertEqual(generated["package"]["rust-version"], "1.88")
                self.assertEqual(Path(dependency["path"]), (destination / "package/pi-rpc-0.2.0").resolve())
                self.assertFalse((destination / "consumer/Cargo.lock").exists())

    def test_consumer_explicit_archive_ignores_old_extracted_directory(self):
        root = self.consumer_source_root()
        target = Path(self.temp.name) / "configured-target"
        stale = target / "package/pi-rpc-0.2.0/src/lib.rs"
        stale.parent.mkdir(parents=True)
        stale.write_bytes(b"stale source")
        self.write_archive()
        destination = Path(self.temp.name) / "consumer"
        destination.mkdir()
        self.prepare_fixture_consumer(root, target, destination, self.archive)
        self.assertEqual((destination / "package/pi-rpc-0.2.0/src/lib.rs").read_bytes(), self.files["src/lib.rs"])
        self.assertEqual(stale.read_bytes(), b"stale source")

    def test_consumer_rejects_stale_commit_dirty_package_and_wrong_version(self):
        root = self.consumer_source_root()
        destination = Path(self.temp.name) / "consumer"
        destination.mkdir()
        mutations = [(".cargo_vcs_info.json", json.dumps({"git": {"sha1": "old"}}).encode()),
                     (".cargo_vcs_info.json", json.dumps({"git": {"sha1": "expected", "dirty": True}}).encode()),
                     ("Cargo.toml", self.files["Cargo.toml"].replace(b'version="0.2.0"', b'version="0.0.9"'))]
        for name, data in mutations:
            saved = self.files[name]
            with self.subTest(name=name, data=data):
                self.files[name] = data
                self.write_archive()
                with self.assertRaises(CheckError):
                    self.prepare_fixture_consumer(root, Path(self.temp.name), destination, self.archive)
                self.assertEqual(list(destination.iterdir()), [])
            self.files[name] = saved

    def test_consumer_rejects_archive_source_tampering_before_extraction(self):
        root = self.consumer_source_root()
        self.files["src/lib.rs"] = b"changed source with unchanged VCS claim"
        self.write_archive()
        destination = Path(self.temp.name) / "consumer"
        destination.mkdir()
        with self.assertRaisesRegex(CheckError, "archive differs from source file"):
            self.prepare_fixture_consumer(root, Path(self.temp.name), destination, self.archive)
        self.assertEqual(list(destination.iterdir()), [])

    def test_consumer_rejects_unsafe_windows_paths_before_extraction(self):
        root = self.consumer_source_root()
        destination = Path(self.temp.name) / "consumer"
        destination.mkdir()
        for name in ["C:/escape", "src/lib.rs:stream"]:
            with self.subTest(name=name):
                self.files[name] = b"fixture"
                self.write_archive()
                with self.assertRaisesRegex(CheckError, "unsafe extraction path"):
                    self.prepare_fixture_consumer(root, Path(self.temp.name), destination, self.archive)
                self.assertEqual(list(destination.iterdir()), [])
                del self.files[name]

    def test_consumer_rejects_nonempty_destination(self):
        root = self.consumer_source_root()
        self.write_archive()
        destination = Path(self.temp.name) / "consumer"
        destination.mkdir()
        (destination / "keep").write_bytes(b"caller data")
        with self.assertRaisesRegex(CheckError, "destination must be empty"):
            self.prepare_fixture_consumer(root, Path(self.temp.name), destination, self.archive)
        self.assertEqual((destination / "keep").read_bytes(), b"caller data")

    def test_package_table_and_version_required(self):
        for name in ["Cargo.toml", "Cargo.toml.orig"]:
            saved = self.files[name]
            for replacement in [b'package=false\n', b'[other]\n',
                                saved.replace(b'version="0.2.0"\n', b''),
                                saved.replace(b'version="0.2.0"', b'version=1')]:
                with self.subTest(name=name, replacement=replacement):
                    self.files[name] = replacement
                    self.write_archive()
                    with self.assertRaises(CheckError):
                        check_package(self.archive)
            self.files[name] = saved

    def source_root(self):
        root = Path(self.temp.name) / "source"
        root.mkdir(exist_ok=True)
        for name in ["Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", ".gitignore", *sorted(n for n in self.files if n.startswith("docs/"))]:
            path = root / name
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(self.files[name])
        return root

    def test_source_mismatch(self):
        root = self.source_root()
        self.write_archive()
        for name in ["README.md", ".gitignore", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "docs/provenance.md"]:
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
        for name in ["LICENSE", "Cargo.lock", "README.md", ".gitignore", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "docs/provenance.md", "docs/verification.md"]:
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
        for name, kind in [("pi-rpc-0.2.0/link", tarfile.SYMTYPE),
                           ("pi-rpc-0.2.0/../escape", tarfile.REGTYPE),
                           ("pi-rpc-0.2.0/README.md", tarfile.REGTYPE)]:
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
