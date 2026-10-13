#!/usr/bin/env python3
"""Validate actual .crate contents without extracting or consulting host link targets."""
import argparse
import sys
import tarfile
from pathlib import Path

sys.dont_write_bytecode = True
from validation_checks import CheckError, check_package


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("archive", type=Path)
    parser.add_argument("--source-root", type=Path, help="also compare original manifest, lockfile, LICENSE and docs")
    parser.add_argument("--expected-commit", help="require matching clean Cargo VCS metadata")
    args = parser.parse_args()
    try:
        files, documents, links = check_package(args.archive, args.source_root, args.expected_commit)
        print(f"PASS archive={args.archive.resolve()}: {files} files, Apache-2.0/publish=[crates-io], {documents} Markdown documents, {links} archive-local links/anchors")
    except (CheckError, OSError, UnicodeError, tarfile.TarError, KeyError, ValueError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
