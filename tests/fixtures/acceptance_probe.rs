//! Std-only version probe. The RPC branch never executes a model or full Pi.
use std::{
    io::{self, BufRead},
    path::PathBuf,
};

fn main() {
    let mut args = std::env::args_os();
    let executable = PathBuf::from(args.next().unwrap());
    let args: Vec<_> = args.collect();
    if args.len() == 1 && args[0] == "--version" {
        std::fs::write(executable.with_extension("probe-started"), b"probe").unwrap();
        let version = std::fs::read_to_string(executable.with_extension("version")).unwrap();
        println!("{version}");
        return;
    }
    assert!(
        args.windows(2)
            .any(|pair| pair[0] == "--mode" && pair[1] == "rpc")
    );
    std::fs::write(executable.with_extension("rpc-started"), b"rpc").unwrap();
    // Wait for the first complete write, then end without any RPC/model work.
    // This avoids a broken-pipe race masking the deliberate process exit.
    io::stdin().lock().lines().next().unwrap().unwrap();
    std::process::exit(37);
}
