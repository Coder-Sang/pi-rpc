#!/usr/bin/env python3
"""Validate actual .crate contents without extracting or consulting host link targets."""
import argparse
import subprocess
import sys
import tarfile
from pathlib import Path

sys.dont_write_bytecode = True
from validation_checks import CheckError, check_package
from prepare_consumer import package_archive


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", nargs="?", type=Path, help="default: resolve via Cargo with --source-root")
    parser.add_argument("--source-root", type=Path, help="also compare original manifest, lockfile, LICENSE and docs")
    parser.add_argument("--expected-commit", help="require matching clean Cargo VCS metadata")
    args = parser.parse_args()
    if args.archive is None and args.source_root is None:
        parser.error("provide an archive or --source-root")
    try:
        archive = args.archive if args.archive is not None else package_archive(args.source_root)
        files, documents, links = check_package(archive, args.source_root, args.expected_commit)
        print(f"PASS archive={archive.resolve()}: {files} files, Apache-2.0/publish=[crates-io], {documents} Markdown documents, {links} archive-local links/anchors")
    except (CheckError, OSError, UnicodeError, tarfile.TarError, KeyError, ValueError,
            subprocess.CalledProcessError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
