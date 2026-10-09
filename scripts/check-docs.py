#!/usr/bin/env python3
"""Check local Markdown targets from an explicit repository/package root."""
import argparse
import sys
from pathlib import Path

sys.dont_write_bytecode = True
from validation_checks import CheckError, check_documents, read_root


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    try:
        root, files = read_root(args.root)
        documents, links = check_documents(files)
        print(f"PASS root={root}: {documents} Markdown documents, {links} local links/anchors; no Git dependency")
    except (CheckError, OSError, UnicodeError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
