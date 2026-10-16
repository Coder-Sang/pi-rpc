#!/usr/bin/env python3
"""Record a clean candidate's inspected archive, source hashes and optional CI run."""
import argparse
import json
from pathlib import Path
import subprocess
import sys
import tarfile

sys.dont_write_bytecode = True
from prepare_consumer import cargo_package_location
from validation_checks import CheckError, release_evidence, require, unwanted_package_file


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source-root", type=Path, default=Path(__file__).resolve().parent.parent)
    parser.add_argument("--archive", type=Path, help="default: Cargo's effective target package archive")
    parser.add_argument("--output", type=Path, help="default: effective target/verification/release-evidence.json")
    parser.add_argument("--ci-run-id")
    parser.add_argument("--ci-os")
    parser.add_argument("--ci-toolchain")
    args = parser.parse_args()
    try:
        root = args.source_root.resolve(strict=True)
        package, target = cargo_package_location(root)
        archive = args.archive or target / "package" / f"{package['name']}-{package['version']}.crate"
        output = (args.output or target / "verification/release-evidence.json").resolve()
        require(output != archive.resolve() and not (
                    output.exists() and archive.exists() and output.samefile(archive)),
                "evidence output must not overwrite the crate archive")
        require(not output.is_relative_to(root) or unwanted_package_file(output.relative_to(root).as_posix()),
                "evidence output must stay outside packaged source files")
        commit = subprocess.check_output(["git", "-C", str(root), "rev-parse", "HEAD"], text=True).strip()
        require(not subprocess.check_output(["git", "-C", str(root), "status", "--porcelain"], text=True).strip(),
                "evidence requires a clean working tree")
        ci = None
        if any(value is not None for value in [args.ci_run_id, args.ci_os, args.ci_toolchain]):
            ci = {"run_id": args.ci_run_id, "os": args.ci_os, "toolchain": args.ci_toolchain}
        rustc = subprocess.check_output(["rustc", "--version"], text=True).strip()
        evidence = release_evidence(archive, root, commit, rustc, ci)
        output.parent.mkdir(parents=True, exist_ok=True)
        output.write_text(json.dumps(evidence, ensure_ascii=False, indent=2) + "\n", encoding="utf-8")
        print(f"PASS release evidence: {output}; commit={commit}; archive={evidence['archive']['sha256']}")
    except (CheckError, OSError, UnicodeError, ValueError, tarfile.TarError,
            subprocess.CalledProcessError) as error:
        parser.exit(1, f"FAIL: {error}\n")


if __name__ == "__main__":
    main()
