//! Caller-declared versions use the same RPC ownership without a probe process.
use super::*;

fn declared_options(dir: &std::path::Path, label: &str, mode: &str) -> ProcessOptions {
    let executable = dir.join(format!(
        "probe-forbidden-{label}{}",
        std::env::consts::EXE_SUFFIX
    ));
    named_fixture(&executable);
    ProcessOptions {
        executable,
        ..options(mode)
    }
}

fn no_children(path: &std::path::Path) {
    assert!(!path.with_extension("probe-started").exists());
    assert!(!path.with_extension("rpc-started").exists());
}

async fn version_rejected(result: pi_rpc::Result<(pi_rpc::PiClient, PiEvents, PiProcess)>) {
    match result {
        Err(error) => assert_eq!(error, Error::Version),
        Ok((_client, _events, mut owner)) => {
            // Mutations must not leak a child when they incorrectly accept input.
            owner.close().await;
            panic!("invalid or mismatched version started RPC");
        }
    }
}

async fn finish(client: pi_rpc::PiClient, mut events: PiEvents, mut owner: PiProcess) {
    assert_eq!(client.get_state().await.unwrap().session_id, "fake");
    assert!(client.get_messages().await.unwrap().messages.is_empty());
    assert_eq!(owner.close().await, Error::ProcessExit(Some(0)));
    assert_eq!(owner.process_exit(), Some(Error::ProcessExit(Some(0))));
    terminal_once(&mut events, Error::ProcessExit(Some(0))).await;
    assert_eq!(client.abort().await, Err(Error::ProcessExit(Some(0))));
}

#[tokio::test]
async fn declared_version_skips_probe_with_default_and_zero_probe_deadlines() {
    let dir = tempfile::tempdir().unwrap();
    for (label, deadline) in [
        ("default", Duration::from_secs(5)),
        ("zero", Duration::ZERO),
    ] {
        let mut options = declared_options(dir.path(), label, "normal");
        let path = options.executable.clone();
        options.startup_timeout = deadline;
        let (client, mut events, owner) = PiProcess::spawn_with_version(options, "1.4.0")
            .await
            .unwrap();
        ready(&mut events).await;
        assert_eq!(
            std::fs::read(path.with_extension("rpc-started")).unwrap(),
            b"rpc"
        );
        assert!(!path.with_extension("probe-started").exists());
        finish(client, events, owner).await;
        assert!(!path.with_extension("probe-started").exists());
    }
}

#[tokio::test]
async fn ordinary_spawn_still_probes_with_default_and_exact_expectations() {
    let dir = tempfile::tempdir().unwrap();
    for (index, expected) in [pi_rpc::PI_VERSION_REQUIREMENT, "1.4.0"]
        .into_iter()
        .enumerate()
    {
        let mut options = declared_options(dir.path(), &format!("control-{index}"), "normal");
        options.expected_version = expected.into();
        let path = options.executable.clone();
        match PiProcess::spawn(options).await {
            Err(error) => assert_eq!(error, Error::ProcessExit(Some(86))),
            Ok((_client, _events, mut owner)) => {
                owner.close().await;
                panic!("ordinary spawn bypassed its required version probe");
            }
        }
        assert_eq!(
            std::fs::read(path.with_extension("probe-started")).unwrap(),
            b"probe"
        );
        assert!(!path.with_extension("rpc-started").exists());
    }
}

#[tokio::test]
async fn declared_versions_obey_default_legacy_and_exact_expectations() {
    let dir = tempfile::tempdir().unwrap();
    let mut cases = Vec::new();
    for version in [
        "1.1.0", "1.1.1", "1.1.10", "1.2.0", "1.2.10", "1.3.0", "1.3.1", "1.3.10", "1.4.0",
    ] {
        cases.push((version, pi_rpc::PI_VERSION_REQUIREMENT));
        cases.push((version, version));
    }
    cases.extend([
        ("1.1.0", ">=1.1.0, <=1.2.0"),
        ("1.2.0", ">=1.1.0, <=1.2.0"),
        ("1.2.10", ">=1.1.0, <=1.3.0"),
        ("1.3.0", ">=1.1.0, <=1.3.0"),
    ]);
    for (index, (version, expected)) in cases.into_iter().enumerate() {
        let mut options = declared_options(dir.path(), &format!("accepted-{index}"), "normal");
        let path = options.executable.clone();
        options.expected_version = expected.into();
        let (client, mut events, owner) = PiProcess::spawn_with_version(options, version)
            .await
            .unwrap();
        ready(&mut events).await;
        finish(client, events, owner).await;
        assert!(path.with_extension("rpc-started").exists());
        assert!(!path.with_extension("probe-started").exists());
    }
}

#[tokio::test]
async fn invalid_declared_versions_fail_before_any_child() {
    let dir = tempfile::tempdir().unwrap();
    let mut versions: Vec<String> = [
        "",
        "1",
        "1.4",
        "1.4.0.0",
        "1.0.4",
        "1.4.1",
        "2.0.0",
        " 1.4.0",
        "1.4.0 ",
        "1.4.0\n",
        "\t1.4.0",
        "01.4.0",
        "1.04.0",
        "1.4.00",
        "v1.4.0",
        "1.4.0-rc.1",
        "1.4.0+build",
        "1.1.18446744073709551616",
        "１.4.0",
        pi_rpc::PI_VERSION_REQUIREMENT,
        ">=1.1.0, <=1.2.0",
        ">=1.1.0, <=1.3.0",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect();
    versions.push(format!("1.1.{}", "9".repeat(100_000)));
    for (index, version) in versions.iter().enumerate() {
        let options = declared_options(dir.path(), &format!("invalid-{index}"), "normal");
        let path = options.executable.clone();
        version_rejected(PiProcess::spawn_with_version(options, version).await).await;
        no_children(&path);
    }
}

#[tokio::test]
async fn invalid_or_mismatched_expectations_fail_before_any_child() {
    let dir = tempfile::tempdir().unwrap();
    let mut cases = vec![
        ("1.2.1", ">=1.1.0, <=1.2.0"),
        ("1.2.10", ">=1.1.0, <=1.2.0"),
        ("1.3.1", ">=1.1.0, <=1.3.0"),
        ("1.3.10", ">=1.1.0, <=1.3.0"),
        ("1.4.0", ">=1.1.0, <=1.3.0"),
        ("1.3.1", "1.3.10"),
        ("1.3.10", "1.3.1"),
        ("1.4.0", "1.1.0"),
    ];
    for expected in [
        "",
        "*",
        "^1.1.0",
        ">=1.1.0",
        ">=1.1.0,<=1.4.0",
        " 1.4.0",
        "1.0.4",
        "1.4.1",
        "1.4.0-rc.1",
        "1.4.0+build",
        "1.04.0",
        "1.4",
        "1.1.18446744073709551616",
    ] {
        cases.push(("1.4.0", expected));
    }
    for (index, (version, expected)) in cases.into_iter().enumerate() {
        let mut options = declared_options(dir.path(), &format!("mismatch-{index}"), "normal");
        let path = options.executable.clone();
        options.expected_version = expected.into();
        version_rejected(PiProcess::spawn_with_version(options, version).await).await;
        no_children(&path);
    }
}

#[tokio::test]
async fn declared_version_validates_before_reporting_a_launch_failure() {
    let dir = tempfile::tempdir().unwrap();
    let executable = dir.path().join("missing-pi");
    let options = || ProcessOptions {
        executable: executable.clone(),
        ..Default::default()
    };
    version_rejected(PiProcess::spawn_with_version(options(), "1.4.1").await).await;
    let invalid_expected = ProcessOptions {
        expected_version: "*".into(),
        ..options()
    };
    version_rejected(PiProcess::spawn_with_version(invalid_expected, "1.4.0").await).await;
    assert!(matches!(
        PiProcess::spawn_with_version(options(), "1.4.0").await,
        Err(Error::Transport(_))
    ));
}

#[tokio::test]
async fn declared_version_preserves_args_cwd_and_bounded_stderr() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = declared_options(dir.path(), "context", "stderr");
    let path = options.executable.clone();
    let cwd = dir.path().join("owned work");
    std::fs::create_dir(&cwd).unwrap();
    options.cwd = Some(cwd.clone());
    options
        .args
        .extend(["--expect-cwd".into(), cwd.into_os_string()]);
    options.stderr_bytes = 31;
    let (client, mut events, owner) = PiProcess::spawn_with_version(options, "1.4.0")
        .await
        .unwrap();
    let diagnostics = owner.diagnostics();
    ready(&mut events).await;
    finish(client, events, owner).await;
    let bytes = diagnostics.snapshot();
    assert_eq!(bytes.len(), 31);
    assert!(String::from_utf8_lossy(&bytes).contains("DIAGNOSTIC_ONLY"));
    assert!(!path.with_extension("probe-started").exists());
}

#[tokio::test]
async fn declared_version_exit_and_protocol_errors_keep_single_terminal() {
    let dir = tempfile::tempdir().unwrap();
    for mode in ["exit", "partial"] {
        let options = declared_options(dir.path(), mode, mode);
        let (client, mut events, mut owner) = PiProcess::spawn_with_version(options, "1.4.0")
            .await
            .unwrap();
        ready(&mut events).await;
        let terminal = tokio::time::timeout(Duration::from_secs(3), owner.wait_terminal())
            .await
            .unwrap();
        if mode == "exit" {
            assert_eq!(terminal, Error::ProcessExit(Some(7)));
        } else {
            assert!(matches!(terminal, Error::Protocol(_)), "{terminal:?}");
        }
        assert_eq!(client.abort().await, Err(terminal.clone()));
        assert_eq!(owner.close().await, terminal);
        assert!(owner.process_exit().is_some());
        terminal_once(&mut events, terminal).await;
    }
}

#[tokio::test]
async fn declared_version_terminate_reaps_child_and_delivers_one_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let options = declared_options(dir.path(), "terminate", "hang");
    let (client, mut events, mut owner) = PiProcess::spawn_with_version(options, "1.4.0")
        .await
        .unwrap();
    let _pid = ready(&mut events).await;
    #[cfg(windows)]
    let child = windows_process::Process::open(_pid);
    assert!(matches!(owner.terminate().await, Error::ProcessExit(_)));
    assert_eq!(owner.terminal_reason(), Some(Error::Terminated));
    assert_eq!(client.abort().await, Err(Error::Terminated));
    assert!(owner.process_exit().is_some());
    #[cfg(windows)]
    child.wait_terminated().await;
    #[cfg(unix)]
    assert_child_reaped(_pid).await;
    terminal_once(&mut events, Error::Terminated).await;
}

#[tokio::test]
async fn declared_version_close_deadline_reaps_uncooperative_child() {
    let dir = tempfile::tempdir().unwrap();
    let mut options = declared_options(dir.path(), "close-deadline", "hang");
    options.connection.close_timeout = Duration::from_millis(20);
    let (client, mut events, mut owner) = PiProcess::spawn_with_version(options, "1.4.0")
        .await
        .unwrap();
    let _pid = ready(&mut events).await;
    #[cfg(windows)]
    let child = windows_process::Process::open(_pid);
    assert_eq!(owner.close().await, Error::CloseTimeout);
    assert_eq!(client.abort().await, Err(Error::CloseTimeout));
    assert!(owner.process_exit().is_some());
    #[cfg(windows)]
    child.wait_terminated().await;
    #[cfg(unix)]
    assert_child_reaped(_pid).await;
    terminal_once(&mut events, Error::CloseTimeout).await;
}

#[tokio::test]
async fn declared_version_owner_drop_reaps_child_with_live_client() {
    let dir = tempfile::tempdir().unwrap();
    let options = declared_options(dir.path(), "drop", "hang");
    let (client, mut events, owner) = PiProcess::spawn_with_version(options, "1.4.0")
        .await
        .unwrap();
    let _pid = ready(&mut events).await;
    #[cfg(windows)]
    let child = windows_process::Process::open(_pid);
    drop(owner);
    assert_eq!(client.abort().await, Err(Error::Closed));
    #[cfg(windows)]
    child.wait_terminated().await;
    #[cfg(unix)]
    assert_child_reaped(_pid).await;
    terminal_once(&mut events, Error::Closed).await;
}

#[cfg(unix)]
async fn assert_child_reaped(pid: u32) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let output = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .unwrap();
            if output.stdout.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}
