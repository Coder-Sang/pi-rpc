"""Local release checks, using only Python 3.11+ standard-library modules.

This is a bounded check of this repository's Markdown links and crate contents,
not a general Markdown parser, secret scanner, or filesystem sandbox.
"""

import hashlib
import posixpath
import re
import tarfile
import tomllib
from pathlib import Path, PurePosixPath
from urllib.parse import unquote, urlsplit

APACHE_SHA256 = "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
REQUIRED_FILES = {
    "Cargo.toml", "Cargo.toml.orig", "Cargo.lock", "LICENSE", "README.md",
    "src/lib.rs", "docs/compatibility.md", "docs/verification.md", "docs/publishing.md",
    "tests/fixtures/fake_pi.rs", "tests/fixtures/rpc_acceptance_extension.ts",
    "tests/fixtures/rpc_retry_extension.ts", "tests/fixtures/rpc_live_tool.ts",
}
PACKAGE_METADATA = {
    "name": "pi-rpc", "license": "Apache-2.0", "publish": False,
    "edition": "2024", "rust-version": "1.85", "readme": "README.md",
    "repository": "https://github.com/Coder-Sang/pi-rpc",
    "keywords": ["pi", "rpc", "jsonl", "tokio", "async"],
}


class CheckError(ValueError):
    """An actionable release-check failure."""


def require(condition, message):
    if not condition:
        raise CheckError(message)


def prose_lines(text, name):
    """Ignore fenced code while finding headings and links in the prose."""
    fence = None
    for line in text.splitlines():
        match = re.match(r"^\s{0,3}(`{3,}|~{3,})(.*)$", line)
        if match:
            marker, suffix = match.groups()
            if fence is None:
                fence = (marker[0], len(marker))
                continue
            if marker[0] == fence[0] and len(marker) >= fence[1] and not suffix.strip():
                fence = None
                continue
        if fence is None:
            yield line
    require(fence is None, f"{name}: unclosed Markdown code fence")


def heading_anchors(text, name):
    used, anchors = {}, set()
    for line in prose_lines(text, name):
        match = re.match(r"^#{1,6}\s+(.+?)(?:\s+#+)?$", line)
        if match:
            heading = re.sub(r"[^\w\- ]", "", match[1].lower()).replace(" ", "-")
            number = used.get(heading, 0)
            used[heading] = number + 1
            anchors.add(heading if number == 0 else f"{heading}-{number}")
    return anchors


def check_documents(files):
    """Check a mapping of POSIX relative names to bytes, never host targets."""
    names = ["README.md", *sorted(n for n in files if n.startswith("docs/") and n.endswith(".md"))]
    require("README.md" in files, "missing README.md")
    links = 0
    for name in names:
        text = files[name].decode("utf-8")
        require("/Users/" not in text, f"{name}: personal absolute path found")
        prose = "\n".join(prose_lines(text, name))
        for target in re.findall(r"\[[^\]]*\]\(([^\s)]+)\)", prose):
            parsed = urlsplit(target)
            if parsed.scheme:
                continue
            require(not parsed.netloc, f"{name}: unsupported network-relative link {target}")
            path = unquote(parsed.path)
            require("\\" not in path and not path.startswith("/"), f"{name}: non-relative link {target}")
            destination = posixpath.normpath(posixpath.join(posixpath.dirname(name), path)) if path else name
            require(destination != ".." and not destination.startswith("../"), f"{name}: escaping link {target}")
            require(destination in files, f"{name}: missing local link target {target}")
            if parsed.fragment:
                require(unquote(parsed.fragment) in heading_anchors(files[destination].decode("utf-8"), destination),
                        f"{name}: missing anchor {target}")
            links += 1
    return len(names), links


def read_root(root):
    root = Path(root).resolve(strict=True)
    require(root.is_dir(), f"not a directory: {root}")
    files = {}
    # Include potential local link destinations, but skip build/Git/private folders.
    for name in ["README.md", "LICENSE", "Cargo.toml", "Cargo.toml.orig", "Cargo.lock"]:
        path = root / name
        if path.is_file():
            require(not path.is_symlink(), f"symlink target not accepted: {name}")
            files[name] = path.read_bytes()
    docs = root / "docs"
    require(not docs.is_symlink(), "symlink docs directory not accepted")
    if docs.exists():
        for path in docs.rglob("*"):
            require(not path.is_symlink(), f"symlink document not accepted: {path}")
            if path.is_file():
                files[path.relative_to(root).as_posix()] = path.read_bytes()
    return root, files


def read_archive(path):
    files, prefix = {}, None
    # Never extract archive paths or follow archive symlinks.
    with tarfile.open(path, "r:gz") as archive:
        for member in archive:
            parts = PurePosixPath(member.name).parts
            require(not member.name.startswith("/") and ".." not in parts and "\\" not in member.name,
                    f"unsafe archive path: {member.name}")
            require(bool(parts), "empty archive path")
            if prefix is None:
                prefix = parts[0]
            require(parts[0] == prefix, "archive must have one package root")
            if member.isdir():
                continue
            require(member.isfile() and len(parts) >= 2, f"unsupported archive entry: {member.name}")
            name = "/".join(parts[1:])
            require(name not in files, f"duplicate archive entry: {name}")
            require(member.size <= 8 * 1024 * 1024, f"oversized archive entry: {name}")
            require(sum(len(data) for data in files.values()) + member.size <= 64 * 1024 * 1024,
                    "archive exceeds local check size limit")
            files[name] = archive.extractfile(member).read()
    require(bool(files), "empty package archive")
    return prefix, files


def check_package_metadata(metadata, name):
    require(isinstance(metadata, dict), f"{name}: missing or invalid package table")
    require(isinstance(metadata.get("version"), str) and bool(metadata["version"]),
            f"{name}: missing or invalid version")
    for key, expected in PACKAGE_METADATA.items():
        actual = metadata.get(key)
        require(type(actual) is type(expected) and actual == expected,
                f"{name}: unexpected or missing {key}")
    current = metadata
    for key in ("metadata", "docs", "rs"):
        current = current.get(key) if isinstance(current, dict) else None
        require(isinstance(current, dict), f"{name}: missing or invalid docs.rs metadata")
    require(current.get("all-features") is True, f"{name}: docs.rs all-features must be true")


def check_package(path, source_root=None, expected_commit=None):
    prefix, files = read_archive(path)
    require(REQUIRED_FILES <= files.keys(), f"missing package files: {sorted(REQUIRED_FILES - files.keys())}")
    manifest = tomllib.loads(files["Cargo.toml"].decode("utf-8")).get("package")
    original = tomllib.loads(files["Cargo.toml.orig"].decode("utf-8")).get("package")
    for name, metadata in [("Cargo.toml", manifest), ("Cargo.toml.orig", original)]:
        check_package_metadata(metadata, name)
    require(prefix == f"{manifest['name']}-{manifest['version']}", "package root does not match manifest")
    require(manifest["version"] == original.get("version"), "manifest versions differ")
    require(hashlib.sha256(files["LICENSE"]).hexdigest() == APACHE_SHA256, "LICENSE is not exact official Apache-2.0 text")
    require(not any(n.startswith((".git/", "target/", ".pi/")) or PurePosixPath(n).name in {"auth.json", ".env", "settings.json"}
                    or n.endswith((".pem", ".key")) for n in files), "unexpected sensitive/build file in package")
    documents, links = check_documents(files)
    if source_root:
        root, source = read_root(source_root)
        required_source = {"Cargo.toml", "Cargo.lock", "LICENSE", "README.md", "docs/compatibility.md", "docs/verification.md", "docs/publishing.md"}
        require(required_source <= source.keys(), f"missing source comparison files: {sorted(required_source - source.keys())}")
        require({n for n in source if n.startswith("docs/")} == {n for n in files if n.startswith("docs/")},
                "package/source docs inventory differs")
        for name in source:
            packaged_name = "Cargo.toml.orig" if name == "Cargo.toml" else name
            require(packaged_name in files and source[name] == files[packaged_name], f"package differs from source root: {name}")
        require(root.joinpath("Cargo.toml").read_bytes() == files["Cargo.toml.orig"], "original manifest differs from source")
    if expected_commit:
        import json
        require(".cargo_vcs_info.json" in files, "missing VCS metadata")
        vcs = json.loads(files[".cargo_vcs_info.json"])
        require(vcs.get("git", {}).get("sha1") == expected_commit, "package commit differs from expected commit")
        require(not vcs.get("git", {}).get("dirty", False), "package built from dirty tree")
    return len(files), documents, links
