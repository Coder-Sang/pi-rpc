#!/usr/bin/env python3
"""Prepare a fresh downstream project from a verified source crate archive."""
import argparse
import json
from pathlib import Path, PurePosixPath
import subprocess
import sys
import tarfile
import tomllib

sys.dont_write_bytecode = True
from validation_checks import CheckError, check_package_contents, read_archive, require


def cargo_package_location(source_root):
    """Use Cargo's effective target directory, including environment/config overrides."""
    root = Path(source_root).resolve(strict=True)
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--no-deps", "--locked", "--offline",
         "--manifest-path", str(root / "Cargo.toml")],
        cwd=root, check=True, capture_output=True, text=True,
    )
    metadata = json.loads(result.stdout)
    packages = [package for package in metadata["packages"]
                if Path(package["manifest_path"]).resolve() == root / "Cargo.toml"]
    require(len(packages) == 1 and packages[0]["name"] == "pi-rpc", "expected the pi-rpc source package")
    return packages[0], Path(metadata["target_directory"]).resolve()


def package_archive(source_root):
    package, target = cargo_package_location(source_root)
    return target / "package" / f"{package['name']}-{package['version']}.crate"


def prepare_consumer(source_root, destination, archive=None):
    root = Path(source_root).resolve(strict=True)
    package, target = cargo_package_location(root)
    archive = Path(archive).resolve(strict=True) if archive is not None else (
        target / "package" / f"{package['name']}-{package['version']}.crate"
    )
    commit = subprocess.run(["git", "-C", str(root), "rev-parse", "HEAD"],
                            check=True, capture_output=True, text=True).stdout.strip()
    prefix, files = read_archive(archive)
    for name in files:
        # A colon can denote a Windows drive or NTFS alternate data stream.
        require(all(":" not in part for part in PurePosixPath(name).parts), f"unsafe extraction path: {name}")
    manifest = tomllib.loads(files["Cargo.toml"].decode("utf-8"))["package"]
    require(manifest["name"] == package["name"] and manifest["version"] == package["version"],
            "archive name/version differs from the source package")
    counts = check_package_contents(prefix, files, root, commit)
    for name, data in files.items():
        if name in {"Cargo.toml", ".cargo_vcs_info.json"}:
            continue
        source = root / ("Cargo.toml" if name == "Cargo.toml.orig" else name)
        require(source.is_file() and not source.is_symlink() and source.read_bytes() == data,
                f"archive differs from source file: {name}")

    destination = Path(destination)
    require(not destination.is_symlink(), "consumer destination must not be a symlink")
    destination = destination.resolve(strict=True)
    require(destination.is_dir() and not any(destination.iterdir()), "consumer destination must be empty")
    extracted = destination / "package" / prefix
    for name, data in files.items():
        path = extracted / name
        require(path.resolve().is_relative_to(extracted.resolve()), f"unsafe extraction path: {name}")
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(data)

    consumer = destination / "consumer"
    consumer.mkdir()
    text = f'''[package]
name = "pi-rpc-independent-consumer"
version = "0.0.0"
edition = "2024"
rust-version = "1.85"
publish = false

[features]
default = ["subprocess"]
subprocess = ["pi-rpc/subprocess"]
session_file = ["pi-rpc/session_file"]

[dependencies]
pi-rpc = {{ path = {json.dumps(extracted.as_posix(), ensure_ascii=False)}, version = "={package['version']}", default-features = false }}
serde = "1"
serde_json = "1"
tokio = {{ version = "1", features = ["rt", "macros", "io-util"] }}

[[example]]
name = "native_session"
required-features = ["session_file"]
'''
    (consumer / "Cargo.toml").write_text(text, encoding="utf-8")
    for name, source in [("src/main.rs", "examples/injected.rs"),
                         ("tests/public_paths.rs", "tests/public_paths.rs"),
                         ("examples/native_session.rs", "examples/native_session.rs")]:
        path = consumer / name
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_bytes(files[source])
    require(not (consumer / "Cargo.lock").exists(), "consumer must start without the SDK lockfile")
    print(f"PASS verified archive={archive}: {counts[0]} files, commit={commit}; fresh consumer pi-rpc {package['version']}",
          file=sys.stderr, flush=True)
    return target / "independent-consumer"


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", required=True, type=Path)
    parser.add_argument("--destination", required=True, type=Path)
    parser.add_argument("--archive", type=Path, help="explicit archive, relative to the caller's working directory")
    args = parser.parse_args()
    try:
        print(prepare_consumer(args.source_root, args.destination, args.archive).as_posix())
    except (CheckError, OSError, UnicodeError, tarfile.TarError, KeyError, ValueError,
            subprocess.CalledProcessError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
