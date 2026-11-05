#!/usr/bin/env python3
"""Compile positive controls and reject cross-version event/client types, offline."""
import json
import os
from pathlib import Path
import subprocess
import tempfile


def main():
    root = Path(__file__).resolve().parent.parent
    environment = os.environ.copy()
    # Match the enclosing verification toolchain and keep each toolchain's cache.
    target = Path(environment.get("CARGO_TARGET_DIR", root / "target"))
    if not target.is_absolute():
        target = root / target
    environment["CARGO_TARGET_DIR"] = str(target / "protocol-types")
    with tempfile.TemporaryDirectory(prefix="pi-rpc-protocol-types-") as work:
        work = Path(work)
        (work / "src").mkdir()
        manifest = f'''[package]
name = "pi-rpc-protocol-type-probe"
version = "0.0.0"
edition = "2024"
rust-version = "1.88"
publish = false
[dependencies]
pi-rpc = {{ path = {json.dumps(root.as_posix(), ensure_ascii=False)}, default-features = false, features = ["subprocess", "pi_1_3", "pi_1_4", "pi_1_5"] }}
'''
        (work / "Cargo.toml").write_text(manifest, encoding="utf-8")
        source = work / "src/main.rs"
        source.write_text("fn main() {}\n", encoding="utf-8")
        subprocess.run(["cargo", "generate-lockfile", "--offline"], cwd=work, env=environment, check=True)

        def check(text, label, rejected=False, expected_code="E0308"):
            source.write_text("#![allow(dead_code)]\n" + text + "\nfn main() {}\n", encoding="utf-8")
            result = subprocess.run(["cargo", "check", "--offline", "--locked", "--message-format=json"],
                                    cwd=work, env=environment, capture_output=True, text=True)
            diagnostics = []
            for line in result.stdout.splitlines():
                item = json.loads(line)
                if item.get("reason") == "compiler-message":
                    diagnostics.append(item["message"])
            errors = [item for item in diagnostics if item["level"] == "error"]
            codes = {item.get("code", {}).get("code") for item in errors if item.get("code")}
            if (not rejected and result.returncode != 0) or (rejected and not (result.returncode != 0 and expected_code in codes)):
                raise SystemExit(f"FAIL {label}: status={result.returncode}, error_codes={codes}\n{result.stderr}\n" + "\n".join(item.get("rendered", "") for item in errors))
            print(f"PASS {label}: {expected_code + ' rejected' if rejected else 'positive control'}", flush=True)

        for earlier, later in [(3, 4), (3, 5), (4, 5)]:
            for kind in ["Event", "KnownEvent", "AssistantEvent", "PiClient", "PiEvents", "PiConnection", "PiProcess"]:
                def name(minor):
                    return (f"pi_rpc::protocol::v1_{minor}::{kind}" if kind in {"Event", "KnownEvent", "AssistantEvent"}
                            else f"pi_rpc::{kind}<pi_rpc::Pi1{minor}>")
                for first, second in [(earlier, later), (later, earlier)]:
                    check(f"fn same(value: {name(first)}) -> {name(first)} {{ value }}", f"{kind} Pi1{first} positive")
                    check(f"fn cross(value: {name(first)}) -> {name(second)} {{ value }}", f"{kind} Pi1{first}->Pi1{second}", rejected=True)
        check("fn public<P: pi_rpc::PiProtocol>() -> Option<P::Event> { None }", "public generic bound")
        for hook in ["known", "ui", "unknown", "response", "ui_request"]:
            check(f"fn hidden<P: pi_rpc::PiProtocol>() {{ let _ = P::{hook}; }}",
                  f"private {hook} hook", rejected=True, expected_code="E0624")
        check("#[derive(Debug, Clone, Copy, PartialEq, Eq)] struct Unaudited;"
              "impl pi_rpc::PiProtocol for Unaudited { type Event = pi_rpc::protocol::v1_4::Event; const MINOR: u64 = 4; }",
              "unaudited implementation", rejected=True, expected_code="E0277")
    print("PASS protocol isolation: 43 positive controls, 42 E0308 and six privacy/sealing failures", flush=True)


if __name__ == "__main__":
    main()
