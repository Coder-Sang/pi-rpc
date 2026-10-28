//! Standalone std-only fake Pi. Compiled by subprocess integration tests.
use std::{
    io::{self, BufRead, Write},
    time::Duration,
};
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let name = std::path::Path::new(&args[0])
        .file_name()
        .unwrap()
        .to_string_lossy();
    if args.iter().any(|a| a == "--version") {
        std::fs::write(
            std::path::Path::new(&args[0]).with_extension("probe-started"),
            b"probe",
        )
        .unwrap();
        if name.contains("probe-forbidden") {
            std::process::exit(86);
        }
        if name.contains("probe-hang") {
            std::thread::sleep(Duration::from_secs(60));
        }
        if name.contains("probe-large") {
            println!("{}", "x".repeat(100_000));
            return;
        }
        if name.contains("invalid-utf8-version") {
            io::stdout().write_all(b"\xff\n").unwrap();
            return;
        }
        println!(
            "{}",
            if name.contains("wrong-version") {
                "0.99.2"
            } else if name.contains("below-version") {
                "1.0.4"
            } else if name.contains("previous-version") {
                "1.1.0"
            } else if name.contains("baseline-version") {
                "1.2.0"
            } else if name.contains("prefix-short-version") {
                "1.3.1"
            } else if name.contains("prior-version") {
                "1.3.0"
            } else if name.contains("prior-patch") {
                "1.3.10"
            } else if name.contains("patch-version") {
                "1.2.10"
            } else if name.contains("intermediate-version") {
                "1.1.1"
            } else if name.contains("numeric-version") {
                "1.1.10"
            } else if name.contains("future-version") {
                "1.4.1"
            } else if name.contains("prerelease-version") {
                "1.4.0-rc.1"
            } else if name.contains("metadata-version") {
                "1.4.0+build"
            } else if name.contains("leading-zero-version") {
                "1.01.0"
            } else if name.contains("padded-version") {
                " \t1.1.0\r\n "
            } else {
                "1.4.0"
            }
        );
        return;
    }
    std::fs::write(
        std::path::Path::new(&args[0]).with_extension("rpc-started"),
        b"rpc",
    )
    .unwrap();
    assert!(args.windows(2).any(|w| w == ["--mode", "rpc"]));
    if let Some(expected) = args.windows(2).find(|w| w[0] == "--expect-cwd") {
        assert_eq!(
            std::env::current_dir().unwrap().canonicalize().unwrap(),
            std::path::Path::new(&expected[1]).canonicalize().unwrap()
        );
    }
    let mode = args
        .windows(2)
        .find(|w| w[0] == "--fake-mode")
        .map(|w| w[1].as_str())
        .unwrap_or("normal");
    if mode == "hold-stdout" {
        std::thread::sleep(Duration::from_secs(1));
        return;
    }
    let mut out = io::stdout().lock();
    writeln!(
        out,
        "{{\"type\":\"fake_ready\",\"pid\":{}}}",
        std::process::id()
    )
    .unwrap();
    out.flush().unwrap();
    if mode == "descendant-exit" {
        // Hold stdout open independently after the direct child has exited.
        let _descendant = std::process::Command::new(&args[0])
            .args(["--fake-mode", "hold-stdout", "--mode", "rpc"])
            .stdin(std::process::Stdio::null())
            .spawn()
            .unwrap();
        let _ = io::stdin().lock().lines().next();
        std::process::exit(9);
    }
    if mode == "closed-stdout" {
        // A shell can close fd1 portably without unsafe FFI in the fixture.
        // exec keeps the same direct-child PID and then holds the process alive.
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            let error = std::process::Command::new("/bin/sh")
                .args(["-c", "exec 1>&-; exec sleep 60"])
                .exec();
            panic!("exec failed: {error}");
        }
        #[cfg(windows)]
        {
            // Let the test retain a process handle before triggering EOF.
            let _ = io::stdin().lock().lines().next();
            out.flush().unwrap();
            drop(out);
            close_stdout();
            // Never use stdout or return through std's exit-time flush after
            // closing its raw handle. The owner must terminate this child.
            loop {
                std::thread::sleep(Duration::from_secs(60));
            }
        }
    }
    match mode {
        "exit" => std::process::exit(7),
        "partial" => {
            write!(out, "{{\"type\":").unwrap();
            out.flush().unwrap();
            return;
        }
        "flood" => {
            for _ in 0..100 {
                if writeln!(out, "{{\"type\":\"agent_start\"}}").is_err() {
                    return;
                }
            }
            out.flush().unwrap();
        }
        "stderr" => {
            let mut err = io::stderr().lock();
            for _ in 0..1000 {
                err.write_all(b"DIAGNOSTIC_ONLY\n").unwrap();
            }
            err.flush().unwrap();
        }
        _ => {}
    }
    if mode == "hang" {
        std::thread::sleep(Duration::from_secs(60));
        return;
    }
    for line in io::stdin().lock().lines() {
        let line = line.unwrap();
        fn field<'a>(line: &'a str, key: &str) -> Option<&'a str> {
            let prefix = format!("\"{key}\":\"");
            let start = line.find(&prefix)? + prefix.len();
            Some(&line[start..start + line[start..].find('"')?])
        }
        let id = field(&line, "id").unwrap();
        let kind = field(&line, "type").unwrap();
        let data = match kind {
            "get_state" => {
                ",\"data\":{\"thinkingLevel\":\"off\",\"isStreaming\":false,\"isCompacting\":false,\"steeringMode\":\"all\",\"followUpMode\":\"all\",\"sessionId\":\"fake\",\"autoCompactionEnabled\":true,\"messageCount\":0,\"pendingMessageCount\":0}"
            }
            "get_messages" => ",\"data\":{\"messages\":[]}",
            "prompt" => ",\"data\":{\"disposition\":\"handled\"}",
            "clear_queue" => ",\"data\":{\"steering\":[],\"followUp\":[]}",
            _ => "",
        };
        // Small fragmented writes deliberately exercise the byte reader.
        let response = format!(
            "{{\"type\":\"response\",\"id\":\"{id}\",\"command\":\"{kind}\",\"success\":true{data}}}\n"
        );
        for fragment in response.as_bytes().chunks(3) {
            if out.write_all(fragment).is_err() {
                return;
            }
            out.flush().unwrap();
        }
    }
}

#[cfg(windows)]
fn close_stdout() {
    use std::ffi::c_void;
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn GetStdHandle(kind: u32) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    // SAFETY: stdout was flushed and its lock dropped. This is the fixture's
    // inherited stdout handle, closed exactly once and never used again.
    unsafe {
        let handle = GetStdHandle(-11_i32 as u32);
        assert!(!handle.is_null() && handle as isize != -1);
        assert_ne!(CloseHandle(handle), 0);
    }
}
