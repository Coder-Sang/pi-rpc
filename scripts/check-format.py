#!/usr/bin/env python3
"""Check Cargo targets and every Rust source, including macro include! files."""
import argparse
from pathlib import Path
import subprocess


def rust_sources(root):
    return sorted(path.relative_to(root).as_posix()
                  for directory in ("src", "tests", "examples")
                  for path in (root / directory).rglob("*.rs") if path.is_file())


def check_format(root):
    root = root.resolve(strict=True)
    # Cargo/rustfmt do not expand include! inside macros. Check those files and
    # standalone fixtures directly too, independent of feature selection.
    subprocess.run(["cargo", "fmt", "--check"], cwd=root, check=True)
    sources = rust_sources(root)
    if not sources:
        raise ValueError("no Rust sources found")
    subprocess.run(["rustfmt", "--edition", "2024", "--check", *sources], cwd=root, check=True)
    print(f"PASS Cargo formatting and {len(sources)} explicit Rust sources", flush=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("root", nargs="?", type=Path, default=Path(__file__).resolve().parent.parent)
    args = parser.parse_args()
    try:
        check_format(args.root)
    except (OSError, ValueError, subprocess.CalledProcessError) as error:
        parser.exit(1, f"FAIL formatting: {error}\n")


if __name__ == "__main__":
    main()
