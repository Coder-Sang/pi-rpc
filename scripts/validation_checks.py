"""Local release checks, using only Python 3.11+ standard-library modules.

This is a bounded check of this repository's Markdown links and crate contents,
not a general Markdown parser, secret scanner, or filesystem sandbox.
"""

import hashlib
import posixpath
import re
import tarfile
import tomllib
from fnmatch import fnmatchcase
from html.parser import HTMLParser
from pathlib import Path, PurePosixPath
from urllib.parse import unquote, urlsplit

APACHE_SHA256 = "cfc7749b96f63bd31c3c42b5c471bf756814053e847c10f3eb003417bc523d30"
PI_MIT_SHA256 = "0457f5bcec3b3b211605dfb5d1a49042fd638f3686a410fe099c24a25af13c48"
REQUIRED_FILES = {
    "Cargo.toml", "Cargo.toml.orig", "Cargo.lock", "LICENSE", "README.md", ".gitignore",
    "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "docs/provenance.md", "scripts/verify-consumer.sh",
    "scripts/prepare_consumer.py", "tests/public_paths.rs", "examples/injected.rs", "examples/native_session.rs",
    "src/lib.rs", "docs/compatibility.md", "docs/verification.md", "docs/publishing.md",
    "tests/fixtures/fake_pi.rs", "tests/fixtures/rpc_acceptance_extension.ts",
    "tests/fixtures/rpc_retry_extension.ts", "tests/fixtures/rpc_live_tool.ts",
    "scripts/check-rustdoc.py",
}
PACKAGE_METADATA = {
    "name": "pi-rpc", "license": "Apache-2.0", "publish": ["crates-io"],
    "edition": "2024", "rust-version": "1.88", "readme": "README.md",
    "repository": "https://github.com/Coder-Sang/pi-rpc",
    "keywords": ["pi", "rpc", "jsonl", "tokio", "async"],
}
IGNORED_ROOT_DIRECTORIES = ("target", ".git", ".pi")
IGNORED_DIRECTORIES = ("__pycache__",)
IGNORED_BASENAMES = ("*.pyc", ".DS_Store", "Thumbs.db", "[Dd]esktop.ini", "*.swp", "*.swo", "*~",
                     ".env", ".env.*", "auth.json", "settings.json", "*.pem", "*.key")
ALLOWED_BASENAMES = (".env.example",)


def package_ignore_rules():
    """The deliberately limited ignore policy shared with .gitignore."""
    return [*(f"/{name}/" for name in IGNORED_ROOT_DIRECTORIES),
            *(f"{name}/" for name in IGNORED_DIRECTORIES),
            *IGNORED_BASENAMES, *(f"!{name}" for name in ALLOWED_BASENAMES)]


def unwanted_package_file(name):
    parts = PurePosixPath(name).parts
    return (len(parts) > 1 and parts[0] in IGNORED_ROOT_DIRECTORIES
            or any(part in IGNORED_DIRECTORIES for part in parts[:-1])
            or any(part not in ALLOWED_BASENAMES and any(fnmatchcase(part, pattern) for pattern in IGNORED_BASENAMES)
                   for part in parts))


def check_ignore_policy(data):
    # Git treats leading whitespace as part of a pattern, including before '#'.
    rules = [line for line in data.decode("utf-8").splitlines()
             if line and not line.startswith("#")]
    require(rules == package_ignore_rules(), ".gitignore differs from the package ignore policy")


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
    names = [*(n for n in ("README.md", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md") if n in files),
             *sorted(n for n in files if n.startswith("docs/") and n.endswith(".md"))]
    require("README.md" in files, "missing README.md")
    links = 0
    for name in names:
        text = files[name].decode("utf-8")
        require("/Users/" not in text, f"{name}: personal absolute path found")
        prose = "\n".join(prose_lines(text, name))
        for target in re.findall(r"\[[^\]]*\]\(([^\s)]+)\)", prose):
            parsed = urlsplit(target)
            repository_path = repository_document_path(parsed)
            if parsed.scheme and repository_path is None:
                continue
            require(repository_path is not None or not parsed.netloc,
                    f"{name}: unsupported network-relative link {target}")
            path = repository_path if repository_path is not None else unquote(parsed.path)
            require("\\" not in path and not path.startswith("/"), f"{name}: non-relative link {target}")
            base = "" if repository_path is not None else posixpath.dirname(name)
            destination = posixpath.normpath(posixpath.join(base, path)) if path else name
            require(destination != ".." and not destination.startswith("../"), f"{name}: escaping link {target}")
            require(destination in files, f"{name}: missing local link target {target}")
            if parsed.fragment:
                require(unquote(parsed.fragment) in heading_anchors(files[destination].decode("utf-8"), destination),
                        f"{name}: missing anchor {target}")
            links += 1
    return len(names), links


def repository_document_path(parsed):
    """Map project main-branch guide URLs back to the reviewed source snapshot."""
    prefix = "/Coder-Sang/pi-rpc/blob/main/"
    if parsed.scheme == "https" and parsed.netloc == "github.com" and parsed.path.startswith(prefix):
        require(not parsed.query, "repository document link must not have a query")
        return unquote(parsed.path[len(prefix):])
    return None


class HtmlLinks(HTMLParser):
    def __init__(self, text):
        super().__init__(convert_charrefs=True)
        self.links, self.anchors = [], set()
        self.feed(text)
        self.close()

    def handle_starttag(self, tag, attrs):
        attrs = dict(attrs)
        if "id" in attrs:
            self.anchors.add(attrs["id"])
        if tag == "a":
            if "name" in attrs:
                self.anchors.add(attrs["name"])
            if "href" in attrs:
                self.links.append(attrs["href"])


def check_rustdoc(doc_root, files, crate="pi_rpc"):
    """Check reachable generated HTML, including local files/fragments and guides.

    External sites are not fetched. Project guide URLs are verified against the
    source/archive snapshot, so CI is independent of network or branch timing.
    """
    root = Path(doc_root).resolve(strict=True)
    require(root.is_dir(), "rustdoc root must be a directory")
    start = root / crate / "index.html"
    require(start.is_file(), "missing generated crate index.html")
    pending, visited, parsed_pages = [start], set(), {}

    def page(path):
        if path not in parsed_pages:
            parsed_pages[path] = HtmlLinks(path.read_text(encoding="utf-8"))
        return parsed_pages[path]

    links = 0
    while pending:
        current = pending.pop()
        if current in visited:
            continue
        visited.add(current)
        for href in page(current).links:
            parsed = urlsplit(href)
            source_path = repository_document_path(parsed)
            if source_path is not None:
                require(source_path in files, f"missing repository document: {href}")
                if parsed.fragment:
                    require(unquote(parsed.fragment) in heading_anchors(files[source_path].decode("utf-8"), source_path),
                            f"missing repository anchor: {href}")
                links += 1
                continue
            if parsed.scheme:
                require(parsed.scheme in {"https", "http", "mailto"}, f"unsupported rustdoc link: {href}")
                continue
            require(not parsed.netloc, f"network-relative rustdoc link: {href}")
            path = unquote(parsed.path)
            require("\\" not in path and not path.startswith("/"), f"non-relative rustdoc link: {href}")
            target = (current.parent / path).resolve() if path else current
            require(target.is_relative_to(root), f"escaping rustdoc link: {href}")
            require(target.is_file(), f"missing rustdoc link target: {href}")
            if target.suffix == ".html":
                if parsed.fragment:
                    fragment = unquote(parsed.fragment)
                    anchors = page(target).anchors
                    source_range = re.fullmatch(r"(\d+)-(\d+)", fragment)
                    valid_range = (target.is_relative_to(root / "src") and source_range is not None
                                   and int(source_range[1]) <= int(source_range[2])
                                   and source_range[1] in anchors and source_range[2] in anchors)
                    require(parsed.fragment in anchors or fragment in anchors or valid_range,
                            f"missing rustdoc anchor: {href}")
                # Visit only the crate's pages. Shared rustdoc/source pages may
                # link to dependencies omitted by --no-deps; still check targets.
                if target.is_relative_to(start.parent):
                    pending.append(target)
            links += 1
    return len(visited), links


def read_root(root):
    root = Path(root).resolve(strict=True)
    require(root.is_dir(), f"not a directory: {root}")
    files = {}
    # Include potential local link destinations, but skip build/Git/private folders.
    for name in ["README.md", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "LICENSE", "Cargo.toml", "Cargo.toml.orig", "Cargo.lock", ".gitignore"]:
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
    description = metadata.get("description")
    require(isinstance(description, str) and bool(description.strip()),
            f"{name}: missing or invalid description")
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
    return check_package_contents(prefix, files, source_root, expected_commit)


def check_package_contents(prefix, files, source_root=None, expected_commit=None):
    """Validate the same in-memory archive snapshot that a consumer will use."""
    require(REQUIRED_FILES <= files.keys(), f"missing package files: {sorted(REQUIRED_FILES - files.keys())}")
    manifest = tomllib.loads(files["Cargo.toml"].decode("utf-8")).get("package")
    original = tomllib.loads(files["Cargo.toml.orig"].decode("utf-8")).get("package")
    for name, metadata in [("Cargo.toml", manifest), ("Cargo.toml.orig", original)]:
        check_package_metadata(metadata, name)
    require(prefix == f"{manifest['name']}-{manifest['version']}", "package root does not match manifest")
    require(manifest["version"] == original.get("version"), "manifest versions differ")
    require(manifest["description"] == original["description"], "manifest descriptions differ")
    require(hashlib.sha256(files["LICENSE"]).hexdigest() == APACHE_SHA256, "LICENSE is not exact official Apache-2.0 text")
    notice = files["THIRD_PARTY_NOTICES.md"].partition(b"```text\n")[2].partition(b"\n```")[0]
    require(hashlib.sha256(notice).hexdigest() == PI_MIT_SHA256, "missing or changed pinned Pi MIT notice")
    check_ignore_policy(files[".gitignore"])
    require(not any(unwanted_package_file(n) for n in files), "unexpected sensitive/build file in package")
    documents, links = check_documents(files)
    if source_root:
        root, source = read_root(source_root)
        required_source = {"Cargo.toml", "Cargo.lock", "LICENSE", "README.md", ".gitignore", "CHANGELOG.md", "THIRD_PARTY_NOTICES.md", "docs/provenance.md", "docs/compatibility.md", "docs/verification.md", "docs/publishing.md"}
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
