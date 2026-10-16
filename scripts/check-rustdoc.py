#!/usr/bin/env python3
"""Check links in generated crate HTML against output and source snapshots."""
import argparse
from pathlib import Path
import subprocess
import sys

sys.dont_write_bytecode = True
from validation_checks import CheckError, check_rustdoc, read_root
from prepare_consumer import cargo_package_location


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--doc-root", type=Path, help="default: Cargo's effective target directory/doc")
    args = parser.parse_args()
    try:
        root, files = read_root(args.source_root)
        doc_root = args.doc_root
        if doc_root is None:
            _, target = cargo_package_location(root)
            doc_root = target / "doc"
        pages, links = check_rustdoc(doc_root, files)
        print(f"PASS generated rustdoc: {pages} reachable crate pages, {links} local/repository links")
    except (CheckError, OSError, UnicodeError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
