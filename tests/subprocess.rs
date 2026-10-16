#![cfg(feature = "subprocess")]
use pi_rpc::{Config, Error, PiEvents, PiProcess, ProcessOptions, protocol::Event};
#[tokio::test]
async fn child_exit_with_descendant_holding_stdout_resolves_pending_and_owner() {
    let (c, mut e, mut p) = PiProcess::spawn(options("descendant-exit")).await.unwrap();
    ready(&mut e).await;
    let command = tokio::spawn(async move { c.abort().await });
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(700), p.wait_terminal())
            .await
            .unwrap(),
        Error::ProcessExit(Some(9))
    );
    assert_eq!(command.await.unwrap(), Err(Error::ProcessExit(Some(9))));
    p.close().await;
}
#[cfg(unix)]
#[tokio::test]
async fn stdout_eof_while_child_lives_fails_promptly_and_reaps() {
    let (c, mut e, mut p) = PiProcess::spawn(options("closed-stdout")).await.unwrap();
    let pid = ready(&mut e).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(700), p.wait_terminal())
            .await
            .unwrap(),
        Error::Eof
    );
    assert_eq!(c.abort().await, Err(Error::Eof));
    p.close().await;
    assert!(p.process_exit().is_some());
    let out = std::process::Command::new("ps")
        .args(["-p", &pid.to_string(), "-o", "pid="])
        .output()
        .unwrap();
    assert!(out.stdout.is_empty());
    terminal_once(&mut e, Error::Eof).await;
}
use std::{path::PathBuf, sync::OnceLock, time::Duration};
fn fixture() -> PathBuf {
    static BINARY: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    BINARY
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let binary = dir
                .path()
                .join(format!("fake-pi{}", std::env::consts::EXE_SUFFIX));
            let status = std::process::Command::new("rustc")
                .args(["--edition=2024", "tests/fixtures/fake_pi.rs", "-o"])
                .arg(&binary)
                .status()
                .unwrap();
            assert!(status.success());
            (dir, binary)
        })
        .1
        .clone()
}
fn options(mode: &str) -> ProcessOptions {
    ProcessOptions {
        executable: fixture(),
        args: vec!["--fake-mode".into(), mode.into()],
        ..Default::default()
    }
}
async fn ready(events: &mut PiEvents) -> u32 {
    let e = tokio::time::timeout(Duration::from_secs(3), events.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    let Event::Unknown(v) = e else {
        panic!("expected fake ready")
    };
    v["pid"].as_u64().unwrap() as u32
}
async fn terminal_once(events: &mut PiEvents, reason: Error) {
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), events.recv())
            .await
            .unwrap(),
        Some(Err(reason))
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), events.recv())
            .await
            .unwrap(),
        None
    );
}

#[cfg(windows)]
mod windows_process {
    use std::ffi::c_void;

    const SYNCHRONIZE: u32 = 0x0010_0000;
    const WAIT_OBJECT_0: u32 = 0;
    const WAIT_TIMEOUT: u32 = 258;

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }

    pub struct Process(*mut c_void);
    impl Process {
        pub fn open(pid: u32) -> Self {
            // SAFETY: requests only observation rights for the ready child.
            let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
            assert!(
                !handle.is_null(),
                "OpenProcess: {}",
                std::io::Error::last_os_error()
            );
            Self(handle)
        }
        pub fn assert_alive(&self) {
            // SAFETY: this object retains a valid handle until Drop.
            assert_eq!(unsafe { WaitForSingleObject(self.0, 0) }, WAIT_TIMEOUT);
        }
        pub async fn wait_terminated(&self) {
            tokio::time::timeout(super::Duration::from_secs(3), async {
                loop {
                    // A retained handle observes the original child, not a
                    // later process that happens to reuse its PID.
                    match unsafe { WaitForSingleObject(self.0, 0) } {
                        WAIT_OBJECT_0 => break,
                        WAIT_TIMEOUT => tokio::time::sleep(super::Duration::from_millis(10)).await,
                        result => panic!("WaitForSingleObject failed: {result:#x}"),
                    }
                }
            })
            .await
            .unwrap();
        }
    }
    impl Drop for Process {
        fn drop(&mut self) {
            // SAFETY: uniquely owned handle, closed exactly once.
            assert_ne!(unsafe { CloseHandle(self.0) }, 0);
        }
    }
}

#[cfg(windows)]
#[tokio::test]
async fn stdout_eof_while_child_lives_fails_promptly_and_terminates() {
    let (c, mut e, mut p) = PiProcess::spawn(options("closed-stdout")).await.unwrap();
    let child = windows_process::Process::open(ready(&mut e).await);
    child.assert_alive();
    // The fixture closes stdout only after receiving this request, then lives
    // indefinitely unless the subprocess owner terminates it.
    let command = tokio::spawn(async move { c.abort().await });
    assert_eq!(
        tokio::time::timeout(Duration::from_millis(700), p.wait_terminal())
            .await
            .unwrap(),
        Error::Eof
    );
    assert_eq!(command.await.unwrap(), Err(Error::Eof));
    p.close().await;
    assert_eq!(p.terminal_reason(), Some(Error::Eof));
    assert!(p.process_exit().is_some());
    child.wait_terminated().await;
    terminal_once(&mut e, Error::Eof).await;
}

#[cfg(windows)]
#[tokio::test]
async fn process_owner_drop_terminates_direct_child_even_with_live_clients() {
    let (c, mut e, p) = PiProcess::spawn(options("hang")).await.unwrap();
    let child = windows_process::Process::open(ready(&mut e).await);
    child.assert_alive();
    drop(p);
    assert_eq!(c.abort().await, Err(Error::Closed));
    child.wait_terminated().await;
    terminal_once(&mut e, Error::Closed).await;
}
#[tokio::test]
async fn local_subprocess_fragmented_protocol_and_orderly_close() {
    let (c, mut e, mut p) = PiProcess::spawn(options("normal")).await.unwrap();
    ready(&mut e).await;
    drop(c.clone());
    assert!(c.get_messages().await.unwrap().messages.is_empty());
    c.abort().await.unwrap();
    assert_eq!(p.close().await, Error::ProcessExit(Some(0)));
    assert_eq!(p.process_exit(), Some(Error::ProcessExit(Some(0))));
}
#[tokio::test]
async fn stderr_drains_continuously_with_bounded_opt_in_diagnostics() {
    let (c, mut e, mut p) = PiProcess::spawn(ProcessOptions {
        stderr_bytes: 31,
        ..options("stderr")
    })
    .await
    .unwrap();
    ready(&mut e).await;
    // The fake emits 15k stderr before reading stdin. This must never block RPC.
    assert!(c.get_messages().await.unwrap().messages.is_empty());
    let bytes = p.diagnostics().snapshot();
    assert_eq!(bytes.len(), 31);
    assert!(String::from_utf8_lossy(&bytes).contains("DIAGNOSTIC_ONLY"));
    assert_eq!(p.close().await, Error::ProcessExit(Some(0)));
}
#[tokio::test]
async fn unexpected_exit_is_not_eof_or_transport_error() {
    let (c, mut e, mut p) = PiProcess::spawn(options("exit")).await.unwrap();
    ready(&mut e).await;
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(3), p.wait_terminal())
            .await
            .unwrap(),
        Error::ProcessExit(Some(7))
    );
    assert_eq!(c.abort().await, Err(Error::ProcessExit(Some(7))));
}
#[tokio::test]
async fn partial_record_at_process_exit_remains_protocol_error() {
    let (_c, mut e, mut p) = PiProcess::spawn(options("partial")).await.unwrap();
    ready(&mut e).await;
    assert!(matches!(p.wait_terminal().await, Error::Protocol(_)));
    p.terminate().await;
}
#[tokio::test]
async fn terminate_is_explicit_and_reaps_child() {
    let (c, mut e, mut p) = PiProcess::spawn(options("hang")).await.unwrap();
    ready(&mut e).await;
    let exit = p.terminate().await;
    assert!(matches!(exit, Error::ProcessExit(_)));
    assert_eq!(p.terminal_reason(), Some(Error::Terminated));
    assert_eq!(c.abort().await, Err(Error::Terminated));
    assert!(p.process_exit().is_some());
}
#[tokio::test]
async fn close_deadline_kills_uncooperative_local_process() {
    let (_c, mut e, mut p) = PiProcess::spawn(ProcessOptions {
        connection: Config {
            close_timeout: Duration::from_millis(20),
            ..Default::default()
        },
        ..options("hang")
    })
    .await
    .unwrap();
    ready(&mut e).await;
    assert_eq!(p.close().await, Error::CloseTimeout);
    assert!(p.process_exit().is_some());
}
#[tokio::test]
async fn event_overflow_notifies_owner_and_cleans_local_process() {
    let (c, _e, mut p) = PiProcess::spawn(ProcessOptions {
        connection: Config {
            max_event_count: 1,
            ..Default::default()
        },
        ..options("flood")
    })
    .await
    .unwrap();
    assert_eq!(p.wait_terminal().await, Error::EventOverflow);
    assert_eq!(c.abort().await, Err(Error::EventOverflow));
    p.close().await;
    assert!(p.process_exit().is_some());
}
#[tokio::test]
async fn version_mismatch_and_bounded_probe_fail_before_rpc_launch() {
    let dir = tempfile::tempdir().unwrap();
    for name in [
        "wrong-version",
        "below-version",
        "future-version",
        "prerelease-version",
        "metadata-version",
        "leading-zero-version",
        "invalid-utf8-version",
        "probe-hang",
        "probe-large",
    ] {
        let path = dir
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(fixture(), &path).unwrap();
        let result = PiProcess::spawn(ProcessOptions {
            executable: path.clone(),
            startup_timeout: if name == "probe-hang" {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(3)
            },
            ..Default::default()
        })
        .await;
        if name.ends_with("version") {
            match result {
                Err(error) => assert_eq!(error, Error::Version),
                Ok(_) => panic!("{name} fixture unexpectedly started RPC"),
            }
        } else {
            assert!(result.is_err());
        }
        // The intentionally short hang deadline may expire before main starts.
        if name != "probe-hang" {
            assert!(path.with_extension("probe-started").exists(), "{name}");
        }
        assert!(!path.with_extension("rpc-started").exists());
    }
}

#[tokio::test]
async fn default_range_and_exact_locks_run_supported_versions() {
    let dir = tempfile::tempdir().unwrap();
    // Intermediate profiles are synthetic range tests, not published Pi versions.
    for name in [
        "current-version",
        "previous-version",
        "intermediate-version",
        "numeric-version",
        "padded-version",
    ] {
        let path = dir
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(fixture(), &path).unwrap();
        let actual = match name {
            "previous-version" | "padded-version" => "1.1.0",
            "intermediate-version" => "1.1.1",
            "numeric-version" => "1.1.10",
            _ => "1.2.0",
        };
        for expected in [pi_rpc::PI_VERSION_REQUIREMENT, actual] {
            let (client, mut events, mut owner) = PiProcess::spawn(ProcessOptions {
                executable: path.clone(),
                expected_version: expected.into(),
                ..Default::default()
            })
            .await
            .unwrap();
            ready(&mut events).await;
            assert_eq!(client.get_state().await.unwrap().session_id, "fake");
            assert!(client.get_messages().await.unwrap().messages.is_empty());
            assert_eq!(owner.close().await, Error::ProcessExit(Some(0)));
            terminal_once(&mut events, Error::ProcessExit(Some(0))).await;
            assert_eq!(client.abort().await, Err(Error::ProcessExit(Some(0))));
        }
    }
}

#[tokio::test]
async fn exact_lock_mismatch_never_launches_rpc() {
    let dir = tempfile::tempdir().unwrap();
    for (name, expected) in [("previous-version", "1.2.0"), ("current-version", "1.1.0")] {
        let path = dir
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(fixture(), &path).unwrap();
        assert!(matches!(
            PiProcess::spawn(ProcessOptions {
                executable: path.clone(),
                expected_version: expected.into(),
                ..Default::default()
            })
            .await,
            Err(Error::Version)
        ));
        assert!(path.with_extension("probe-started").exists());
        assert!(!path.with_extension("rpc-started").exists());
    }
}

#[tokio::test]
async fn invalid_expectations_fail_before_executable_probe() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir
        .path()
        .join(format!("current-version{}", std::env::consts::EXE_SUFFIX));
    std::fs::copy(fixture(), &path).unwrap();
    for expected in [
        "",
        "*",
        "^1.1.0",
        ">=1.1.0",
        ">=1.1.0,<=1.2.0",
        "1.0.4",
        "1.2.1",
        "1.1.0-rc.1",
        "1.2.0+build",
        "1.01.0",
        "1.1",
        "1.1.18446744073709551616",
    ] {
        assert!(
            matches!(
                PiProcess::spawn(ProcessOptions {
                    executable: path.clone(),
                    expected_version: expected.into(),
                    ..Default::default()
                })
                .await,
                Err(Error::Version)
            ),
            "{expected}"
        );
        assert!(!path.with_extension("probe-started").exists());
        assert!(!path.with_extension("rpc-started").exists());
    }
}
#[cfg(unix)]
#[tokio::test]
async fn process_owner_drop_kills_and_reaps_even_with_live_clients() {
    let (c, mut e, p) = PiProcess::spawn(options("hang")).await.unwrap();
    let pid = ready(&mut e).await;
    drop(p);
    assert_eq!(c.abort().await, Err(Error::Closed));
    // ps is observation-only. A missing process also confirms no zombie remains.
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let out = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .unwrap();
            if out.stdout.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
    terminal_once(&mut e, Error::Closed).await;
}
