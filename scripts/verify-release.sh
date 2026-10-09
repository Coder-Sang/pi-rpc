#!/usr/bin/env bash
# Opt-in real release check. No installation, credentials, prompts or paid requests.
set -euo pipefail
cd "$(dirname "$0")/.."
case "$(uname -s)" in
  Darwin|Linux) ;;
  *) echo "Real Pi release verification supports macOS/Linux only" >&2; exit 2 ;;
esac
if [ "$#" -ne 1 ]; then
  echo "usage: bash scripts/verify-release.sh /absolute/path/to/pi" >&2
  exit 2
fi
case "$1" in
  /*) executable="$1" ;;
  *) echo "Pi executable path must be absolute" >&2; exit 2 ;;
esac
test -x "$executable"
# Pin both output directory and host target so Cargo config/CARGO_BUILD_TARGET
# cannot redirect the fresh build while an older runner is executed.
host=$(rustc -vV | awk '/^host: / { print $2 }')
test -n "$host"
cargo build --locked --example release_smoke --target "$host" --target-dir "$PWD/target"
runner="$PWD/target/$host/debug/examples/release_smoke"
# Host-provided scratch directory avoids writing in the project or real HOME.
scratch="${JCODE_SCRATCH_DIR:-${TMPDIR:-/tmp}}"
isolation=$(mktemp -d "$scratch/pi-rpc-release.XXXXXX")
trap 'rm -rf "$isolation"' EXIT
mkdir -p "$isolation/home" "$isolation/agent" "$isolation/work" "$isolation/tmp" "$isolation/bin"
# npm's Pi uses /usr/bin/env node. Admit only the resolved runtime, not the host PATH.
node=$(command -v node || true)
case "$node" in
  /*) ln -s "$node" "$isolation/bin/node" ;;
esac
cd "$isolation/work"
env -i HOME="$isolation/home" PI_CODING_AGENT_DIR="$isolation/agent" \
  TMPDIR="$isolation/tmp" PATH="$isolation/bin:/usr/bin:/bin:/usr/sbin:/sbin" \
  "$runner" "$executable"
