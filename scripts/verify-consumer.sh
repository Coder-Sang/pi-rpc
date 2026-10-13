#!/usr/bin/env bash
# Fresh downstream resolution; compile/run only packaged fake/in-memory examples.
set -euo pipefail
cd "$(dirname "$0")/.."
root="$PWD"
package="$root/target/package/pi-rpc-0.1.0"
test -f "$package/Cargo.toml"
consumer=$(mktemp -d)
trap 'rm -rf -- "$consumer"' EXIT
if command -v cygpath >/dev/null 2>&1; then
  root_native=$(cygpath -m "$root")
  package=$(cygpath -m "$package")
  consumer_native=$(cygpath -m "$consumer")
else
  root_native="$root"
  consumer_native="$consumer"
fi
python_command=python3
if ! command -v "$python_command" >/dev/null 2>&1; then
  python_command=python
fi
"$python_command" - "$package" "$consumer_native" <<'PY'
import json
from pathlib import Path
import shutil
import sys
import tomllib

package, consumer = map(Path, sys.argv[1:])
metadata = tomllib.loads((package / "Cargo.toml").read_text(encoding="utf-8"))["package"]
manifest = f'''[package]
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
pi-rpc = {{ path = {json.dumps(package.as_posix())}, version = "={metadata['version']}", default-features = false }}
serde = "1"
serde_json = "1"
tokio = {{ version = "1", features = ["rt", "macros", "io-util"] }}

[[example]]
name = "native_session"
required-features = ["session_file"]
'''
(consumer / "Cargo.toml").write_text(manifest, encoding="utf-8")
for target, source in [("src/main.rs", "examples/injected.rs"),
                       ("tests/public_paths.rs", "tests/public_paths.rs"),
                       ("examples/native_session.rs", "examples/native_session.rs")]:
    destination = consumer / target
    destination.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(package / source, destination)
assert not (consumer / "Cargo.lock").exists()
print(f"Fresh consumer uses packaged pi-rpc {metadata['version']}; no repository lockfile copied", flush=True)
PY
export CARGO_TARGET_DIR="$root_native/target/independent-consumer"
manifest="$consumer_native/Cargo.toml"
rustc --version
cargo generate-lockfile --manifest-path "$manifest"
for configuration in default all none session_only; do
  case "$configuration" in
    default) flags=() ;;
    all) flags=(--all-features) ;;
    none) flags=(--no-default-features) ;;
    session_only) flags=(--no-default-features --features session_file) ;;
  esac
  echo "Packaged independent consumer: $configuration"
  cargo test --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"}
  cargo run --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"}
  if [ "$configuration" = all ] || [ "$configuration" = session_only ]; then
    cargo run --locked --manifest-path "$manifest" ${flags[@]+"${flags[@]}"} --example native_session
  fi
done
