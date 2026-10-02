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
    for name in ["wrong-version", "probe-hang", "probe-large"] {
        let path = dir
            .path()
            .join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        std::fs::copy(fixture(), &path).unwrap();
        let result = PiProcess::spawn(ProcessOptions {
            executable: path,
            startup_timeout: if name == "probe-hang" {
                Duration::from_millis(100)
            } else {
                Duration::from_secs(3)
            },
            ..Default::default()
        })
        .await;
        if name == "wrong-version" {
            assert!(matches!(result, Err(Error::Version)));
        } else {
            assert!(result.is_err());
        }
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
}
