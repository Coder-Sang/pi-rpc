use super::*;
#[path = "bolt.rs"]
mod bolt;
use pi_rpc::{AutoProcess, ProcessOptions, VersionSource};
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use pi_rpc::{PiClient, PiEvents, PiProcess, PiProtocol};
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use std::time::Duration;
#[path = "../support/fake_process.rs"]
mod fake_process;
#[cfg(all(
    windows,
    any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5")
))]
#[path = "../support/windows_process.rs"]
mod windows_process;

fn prepare(version: &str, forbidden: bool, mode: &str) -> (tempfile::TempDir, ProcessOptions) {
    let root = tempfile::tempdir().unwrap();
    let executable = root.path().join(format!(
        "{}{}",
        if forbidden {
            "probe-forbidden"
        } else {
            "synthetic-pi"
        },
        std::env::consts::EXE_SUFFIX
    ));
    let executable = fake_process::named_fixture(&executable);
    std::fs::write(executable.with_extension("version"), version).unwrap();
    let options = ProcessOptions {
        executable,
        args: vec!["--fake-mode".into(), mode.into()],
        ..Default::default()
    };
    (root, options)
}
fn markers(path: &std::path::Path, probe: bool, rpc: bool) {
    assert_eq!(
        path.with_extension("probe-started").exists(),
        probe,
        "probe marker"
    );
    assert_eq!(
        path.with_extension("rpc-started").exists(),
        rpc,
        "RPC marker"
    );
}
fn none_enabled() -> bool {
    !cfg!(any(
        feature = "pi_1_3",
        feature = "pi_1_4",
        feature = "pi_1_5"
    ))
}

#[cfg(all(unix, any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5")))]
async fn assert_reaped(pid: u32) {
    tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let result = std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid="])
                .output()
                .unwrap();
            assert!(result.status.success() || result.status.code() == Some(1));
            if result.stdout.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .unwrap();
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
trait ReadyEvent {
    fn pid(self) -> u32;
}
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! ready_event {
    ($module:ident) => {
        impl ReadyEvent for pi_rpc::protocol::$module::Event {
            fn pid(self) -> u32 {
                let Self::Unknown(value) = self else {
                    panic!("fake ready event missing")
                };
                assert_eq!(value["type"], "fake_ready");
                value["pid"].as_u64().unwrap().try_into().unwrap()
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
ready_event!(v1_3);
#[cfg(feature = "pi_1_4")]
ready_event!(v1_4);
#[cfg(feature = "pi_1_5")]
ready_event!(v1_5);
#[cfg(feature = "bolt_message_id")]
impl ReadyEvent for pi_rpc::protocol::bolt_message_id_v1::Event {
    fn pid(self) -> u32 {
        let Self::Pi(event) = self else {
            panic!("expected fake ready event")
        };
        event.pid()
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
async fn ready<P: PiProtocol>(events: &mut PiEvents<P>, executable: &std::path::Path) -> u32
where
    P::Event: ReadyEvent,
{
    let result = tokio::time::timeout(Duration::from_secs(3), events.recv()).await;
    if result.is_err() {
        let rpc = executable.with_extension("rpc-started").exists();
        let written = executable.with_extension("ready-written").exists();
        let pid = std::fs::read_to_string(executable.with_extension("rpc-pid"))
            .ok()
            .and_then(|pid| pid.parse::<u32>().ok());
        #[cfg(unix)]
        let status = pid.map(|pid| {
            std::process::Command::new("ps")
                .args(["-p", &pid.to_string(), "-o", "pid=,ppid=,stat=,etime="])
                .output()
                .unwrap()
                .stdout
        });
        #[cfg(not(unix))]
        let status = pid;
        panic!(
            "fake ready deadline: version={} rpc_started={rpc} ready_written={written} pid={pid:?} status={status:?}",
            events.info().version()
        );
    }
    result.unwrap().unwrap().unwrap().pid()
}
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
async fn finish<P: PiProtocol>(
    client: PiClient<P>,
    mut events: PiEvents<P>,
    mut owner: PiProcess<P>,
    version: &str,
    source: VersionSource,
    executable: &std::path::Path,
) where
    P::Event: ReadyEvent,
{
    assert_eq!(owner.info().version(), version);
    assert_eq!(owner.info().source(), source);
    assert_eq!(client.info(), owner.info());
    assert_eq!(events.info(), owner.info());
    ready(&mut events, executable).await;
    assert_eq!(client.get_state().await.unwrap().session_id, "fake");
    assert!(client.get_messages().await.unwrap().messages.is_empty());
    client.abort().await.unwrap();
    let reason = owner.close().await;
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(owner.process_exit(), Some(reason.clone()));
    assert_eq!(events.recv().await, Some(Err(reason.clone())));
    assert_eq!(events.recv().await, None);
    assert_eq!(client.abort().await, Err(reason));
}
async fn finish_auto(
    owner: AutoProcess,
    version: &str,
    source: VersionSource,
    _executable: &std::path::Path,
) {
    assert_eq!(owner.info().version(), version);
    assert_eq!(owner.info().source(), source);
    assert_eq!(owner.terminal_reason(), None);
    assert_eq!(owner.process_exit(), None);
    match owner {
        #[cfg(feature = "pi_1_3")]
        AutoProcess::V1_3 {
            client,
            events,
            process,
        } => {
            assert!(version.starts_with("1.3."));
            finish(client, events, process, version, source, _executable).await;
        }
        #[cfg(feature = "pi_1_4")]
        AutoProcess::V1_4 {
            client,
            events,
            process,
        } => {
            assert!(version.starts_with("1.4."));
            finish(client, events, process, version, source, _executable).await;
        }
        #[cfg(feature = "pi_1_5")]
        AutoProcess::V1_5 {
            client,
            events,
            process,
        } => {
            assert!(matches!(version, "1.5.0" | "1.5.1" | "1.5.2"));
            finish(client, events, process, version, source, _executable).await;
        }
        _ => panic!("unexpected protocol variant"),
    }
}

#[tokio::test]
async fn auto_probed_and_declared_versions_respect_enabled_features() {
    for (version, enabled, feature) in [
        ("1.3.0", cfg!(feature = "pi_1_3"), "pi_1_3"),
        ("1.3.10", cfg!(feature = "pi_1_3"), "pi_1_3"),
        ("1.4.0", cfg!(feature = "pi_1_4"), "pi_1_4"),
        ("1.4.10", cfg!(feature = "pi_1_4"), "pi_1_4"),
        ("1.5.0", cfg!(feature = "pi_1_5"), "pi_1_5"),
        ("1.5.1", cfg!(feature = "pi_1_5"), "pi_1_5"),
        ("1.5.2", cfg!(feature = "pi_1_5"), "pi_1_5"),
    ] {
        for probed in [false, true] {
            let (_root, options) = prepare(version, !probed, "normal");
            let path = options.executable.clone();
            let result = if probed {
                pi_rpc::subprocess::spawn_auto(options).await
            } else {
                pi_rpc::subprocess::spawn_auto_with_version(options, version).await
            };
            if enabled {
                finish_auto(
                    result.unwrap_or_else(|error| panic!("{version}: {error}")),
                    version,
                    if probed {
                        VersionSource::Probed
                    } else {
                        VersionSource::Declared
                    },
                    &path,
                )
                .await;
                markers(&path, probed, true);
            } else {
                assert_eq!(
                    result.err(),
                    Some(if none_enabled() {
                        Error::NoProtocolEnabled
                    } else {
                        Error::VersionNotEnabled {
                            version: version.into(),
                            feature,
                        }
                    })
                );
                markers(&path, probed && !none_enabled(), false);
            }
        }
    }
}

#[tokio::test]
async fn auto_skip_validates_declarations_and_expectations_before_children() {
    let mut cases: Vec<(String, String)> = [
        "",
        "1.3",
        "1.2.10",
        "1.5.3",
        " 1.5.0",
        "1.5.0\n",
        "01.5.0",
        "1.05.0",
        "1.5.00",
        "v1.5.0",
        "1.5.0-rc.1",
        "1.5.0+meta",
        "1.3.18446744073709551616",
        pi_rpc::PI_VERSION_REQUIREMENT,
        ">=1.1.0, <=1.3.0",
        ">=1.1.0, <=1.4.0",
    ]
    .into_iter()
    .map(|v| (v.into(), pi_rpc::PI_VERSION_REQUIREMENT.into()))
    .collect();
    cases.push((
        format!("1.3.{}", "9".repeat(100_000)),
        pi_rpc::PI_VERSION_REQUIREMENT.into(),
    ));
    for expected in [
        "*",
        ">=1.1.0, <=1.2.0",
        "1.2.0",
        "1.4.0",
        ">=1.1.0, <=1.3.0",
        ">=1.1.0, <=1.4.0",
    ] {
        cases.push(("1.5.0".into(), expected.into()));
    }
    for (declared, expected) in cases {
        let (_root, mut options) = prepare("1.5.0", true, "normal");
        let path = options.executable.clone();
        options.expected_version = expected;
        let result = pi_rpc::subprocess::spawn_auto_with_version(options, &declared).await;
        assert_eq!(result.err(), Some(Error::Version), "{declared:?}");
        markers(&path, false, false);
    }
}

#[tokio::test]
async fn auto_invalid_expectations_reject_before_probing() {
    for expected in [
        "",
        "*",
        ">=1.1.0, <=1.2.0",
        "1.2.0",
        ">=1.3.0,<=1.5.0",
        "1.5.3",
    ] {
        let (_root, mut options) = prepare("1.5.0", false, "normal");
        let path = options.executable.clone();
        options.expected_version = expected.into();
        assert_eq!(
            pi_rpc::subprocess::spawn_auto(options).await.err(),
            Some(Error::Version)
        );
        markers(&path, false, false);
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! process_cases {
    ($module:ident, $profile:ident, $version:literal, $patch:literal, $other:literal, $legacy:literal) => {
        mod $module {
            use super::*;
            type Process = PiProcess<pi_rpc::$profile>;
            #[tokio::test]
            async fn exact_legacy_and_default_expectations_work_in_both_startup_modes() {
                for version in [$version, $patch] {
                    let mut expected = vec![pi_rpc::PI_VERSION_REQUIREMENT, version];
                    if version == $version {
                        expected.push($legacy);
                    }
                    for expected in expected {
                        for probed in [false, true] {
                            let (_root, mut options) = prepare(version, !probed, "normal");
                            let path = options.executable.clone();
                            options.expected_version = expected.into();
                            let (client, events, process) = if probed {
                                Process::spawn(options).await
                            } else {
                                Process::spawn_with_version(options, version).await
                            }
                            .unwrap();
                            finish(
                                client,
                                events,
                                process,
                                version,
                                if probed {
                                    VersionSource::Probed
                                } else {
                                    VersionSource::Declared
                                },
                                &path,
                            )
                            .await;
                            markers(&path, probed, true);
                        }
                    }
                }
            }
            #[tokio::test]
            async fn static_profile_mismatch_rejects_before_rpc() {
                for probed in [false, true] {
                    let (_root, options) = prepare($other, !probed, "normal");
                    let path = options.executable.clone();
                    let result = if probed {
                        Process::spawn(options).await
                    } else {
                        Process::spawn_with_version(options, $other).await
                    };
                    assert_eq!(result.err(), Some(Error::Version));
                    markers(&path, probed, false);
                }
            }
            #[tokio::test]
            async fn declared_startup_ignores_zero_probe_deadline_and_probe_control_fails() {
                for deadline in [Duration::from_secs(5), Duration::ZERO] {
                    let (_root, mut options) = prepare($version, true, "normal");
                    let path = options.executable.clone();
                    options.startup_timeout = deadline;
                    let (client, events, process) = Process::spawn_with_version(options, $version)
                        .await
                        .unwrap();
                    finish(
                        client,
                        events,
                        process,
                        $version,
                        VersionSource::Declared,
                        &path,
                    )
                    .await;
                    markers(&path, false, true);
                    let (_root, mut options) = prepare($version, true, "normal");
                    let path = options.executable.clone();
                    options.startup_timeout = deadline;
                    let process = pi_rpc::subprocess::spawn_auto_with_version(options, $version)
                        .await
                        .unwrap();
                    finish_auto(process, $version, VersionSource::Declared, &path).await;
                    markers(&path, false, true);
                }
                for auto in [false, true] {
                    let (_root, options) = prepare($version, true, "normal");
                    let path = options.executable.clone();
                    let error = if auto {
                        pi_rpc::subprocess::spawn_auto(options).await.err()
                    } else {
                        Process::spawn(options).await.err()
                    };
                    assert_eq!(error, Some(Error::ProcessExit(Some(86))));
                    markers(&path, true, false);
                }
            }
            #[tokio::test]
            async fn expectation_failures_are_distinct_from_profile_failures() {
                let rejected_expectation = if $version == $patch {
                    ">=1.1.0, <=1.4.0"
                } else {
                    $version
                };
                assert!(<pi_rpc::$profile as PiProtocol>::matches_version($patch));
                for probed in [false, true] {
                    let (_root, mut options) = prepare($patch, !probed, "normal");
                    let path = options.executable.clone();
                    options.expected_version = rejected_expectation.into();
                    let result = if probed {
                        Process::spawn(options).await
                    } else {
                        Process::spawn_with_version(options, $patch).await
                    };
                    assert_eq!(result.err(), Some(Error::Version));
                    markers(&path, probed, false);
                }
            }
            #[tokio::test]
            async fn declared_context_stderr_and_shutdown_are_shared() {
                let (root, mut options) = prepare($version, true, "stderr");
                let path = options.executable.clone();
                let cwd = root.path().join("owned work");
                std::fs::create_dir(&cwd).unwrap();
                options.cwd = Some(cwd.clone());
                options
                    .args
                    .extend(["--expect-cwd".into(), cwd.into_os_string()]);
                options.stderr_bytes = 31;
                let (client, mut events, mut process) =
                    Process::spawn_with_version(options, $version)
                        .await
                        .unwrap();
                ready(&mut events, &path).await;
                client.get_state().await.unwrap();
                let bytes = process.diagnostics().snapshot();
                assert_eq!(bytes.len(), 31);
                assert!(String::from_utf8_lossy(&bytes).contains("DIAGNOSTIC_ONLY"));
                let reason = process.close().await;
                assert_eq!(reason, Error::ProcessExit(Some(0)));
                assert_eq!(events.recv().await, Some(Err(reason)));
                assert_eq!(events.recv().await, None);
            }
            #[tokio::test]
            async fn declared_drop_and_close_deadline_reap_the_owned_child() {
                for dropped in [false, true] {
                    let (_root, mut options) = prepare($version, true, "hang");
                    let path = options.executable.clone();
                    options.connection.close_timeout = Duration::from_millis(20);
                    let (client, mut events, mut process) =
                        Process::spawn_with_version(options, $version)
                            .await
                            .unwrap();
                    let _pid = ready(&mut events, &path).await;
                    #[cfg(windows)]
                    let child = windows_process::Process::open(_pid);
                    #[cfg(windows)]
                    child.assert_alive();
                    let reason = if dropped {
                        drop(process);
                        Error::Closed
                    } else {
                        assert_eq!(process.close().await, Error::CloseTimeout);
                        assert!(process.process_exit().is_some());
                        Error::CloseTimeout
                    };
                    assert_eq!(client.abort().await, Err(reason.clone()));
                    assert_eq!(events.recv().await, Some(Err(reason)));
                    assert_eq!(events.recv().await, None);
                    #[cfg(windows)]
                    child.wait_terminated().await;
                    #[cfg(unix)]
                    assert_reaped(_pid).await;
                }
            }
            #[tokio::test]
            async fn protocol_exit_and_explicit_termination_keep_single_terminal() {
                for (mode, expected) in [
                    ("partial", Error::Protocol("EOF in partial record".into())),
                    ("exit", Error::ProcessExit(Some(7))),
                    ("hang", Error::Terminated),
                ] {
                    let (_root, options) = prepare($version, true, mode);
                    let path = options.executable.clone();
                    let (client, mut events, mut process) =
                        Process::spawn_with_version(options, $version)
                            .await
                            .unwrap();
                    ready(&mut events, &path).await;
                    let reason = if mode == "hang" {
                        let exit = process.terminate().await;
                        assert!(matches!(exit, Error::ProcessExit(_)));
                        assert_eq!(process.process_exit(), Some(exit));
                        assert_eq!(process.terminal_reason(), Some(Error::Terminated));
                        Error::Terminated
                    } else {
                        process.wait_terminal().await
                    };
                    if mode == "partial" {
                        assert!(matches!(reason, Error::Protocol(_)));
                    } else {
                        assert_eq!(reason, expected);
                    }
                    assert_eq!(client.abort().await, Err(reason.clone()));
                    assert_eq!(process.close().await, reason);
                    assert!(process.process_exit().is_some());
                    assert_eq!(events.recv().await, Some(Err(reason)));
                    assert_eq!(events.recv().await, None);
                }
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
process_cases!(pi13, Pi13, "1.3.0", "1.3.10", "1.4.0", ">=1.1.0, <=1.3.0");
#[cfg(feature = "pi_1_4")]
process_cases!(pi14, Pi14, "1.4.0", "1.4.10", "1.5.0", ">=1.1.0, <=1.4.0");
#[cfg(feature = "pi_1_5")]
process_cases!(pi15, Pi15, "1.5.0", "1.5.2", "1.4.0", ">=1.3.0, <=1.5.0");
#[cfg(feature = "bolt_message_id")]
process_cases!(
    identified_bolt,
    BoltMessageIdV1,
    "1.5.0",
    "1.5.0",
    "1.4.0",
    ">=1.3.0, <=1.5.0"
);

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4"))]
#[tokio::test]
async fn exact_patch_prefix_collisions_reject_in_static_and_auto_startup() {
    for (enabled, short, long) in [
        (cfg!(feature = "pi_1_3"), "1.3.1", "1.3.10"),
        (cfg!(feature = "pi_1_4"), "1.4.1", "1.4.10"),
    ] {
        if !enabled {
            continue;
        }
        assert!(long.starts_with(short));
        for (actual, expected) in [(short, long), (long, short)] {
            for probed in [false, true] {
                let (_root, mut options) = prepare(actual, !probed, "normal");
                let path = options.executable.clone();
                options.expected_version = expected.into();
                let result = if probed {
                    pi_rpc::subprocess::spawn_auto(options).await
                } else {
                    pi_rpc::subprocess::spawn_auto_with_version(options, actual).await
                };
                assert_eq!(result.err(), Some(Error::Version));
                markers(&path, probed, false);
                // Matching locks prove that rejection is not caused by a broken fixture.
                let (_root, mut options) = prepare(actual, !probed, "normal");
                let path = options.executable.clone();
                options.expected_version = actual.into();
                let process = if probed {
                    pi_rpc::subprocess::spawn_auto(options).await
                } else {
                    pi_rpc::subprocess::spawn_auto_with_version(options, actual).await
                }
                .unwrap();
                finish_auto(
                    process,
                    actual,
                    if probed {
                        VersionSource::Probed
                    } else {
                        VersionSource::Declared
                    },
                    &path,
                )
                .await;
            }
        }
    }
}

#[tokio::test]
async fn auto_legacy_upper_bounds_reject_patches_and_later_minors() {
    for (actual, expected) in [
        ("1.3.1", ">=1.1.0, <=1.3.0"),
        ("1.3.10", ">=1.1.0, <=1.3.0"),
        ("1.4.1", ">=1.1.0, <=1.4.0"),
        ("1.4.10", ">=1.1.0, <=1.4.0"),
        ("1.5.0", ">=1.1.0, <=1.4.0"),
        ("1.5.1", ">=1.3.0, <=1.5.0"),
        ("1.5.2", ">=1.3.0, <=1.5.0"),
    ] {
        for probed in [false, true] {
            let (_root, mut options) = prepare(actual, !probed, "normal");
            let path = options.executable.clone();
            options.expected_version = expected.into();
            let result = if probed {
                pi_rpc::subprocess::spawn_auto(options).await
            } else {
                pi_rpc::subprocess::spawn_auto_with_version(options, actual).await
            };
            // Without any compiled profiles, ordinary probing rejects immediately.
            let reason = if probed && none_enabled() {
                Error::NoProtocolEnabled
            } else {
                Error::Version
            };
            assert_eq!(result.err(), Some(reason));
            markers(&path, probed && !none_enabled(), false);
        }
    }
}

#[cfg(feature = "pi_1_5")]
#[tokio::test]
async fn pi15_patch_exact_locks_apply_to_declared_plain_and_decorated_startup() {
    for actual in ["1.5.1", "1.5.2"] {
        for expected in ["1.5.0", "1.5.1", "1.5.2", ">=1.3.0, <=1.5.0"] {
            for mode in ["declared", "plain", "decorated"] {
                let probed = mode != "declared";
                let output = if mode == "decorated" {
                    format!("{actual} (Pi-Bolt 9.10.11, linux-x64, JIT off)")
                } else {
                    actual.into()
                };
                let (_root, mut options) = prepare(&output, !probed, "normal");
                let path = options.executable.clone();
                options.expected_version = expected.into();
                let result = if probed {
                    PiProcess::<pi_rpc::Pi15>::spawn(options).await
                } else {
                    PiProcess::<pi_rpc::Pi15>::spawn_with_version(options, actual).await
                };
                if expected == actual {
                    let (client, events, process) = result.unwrap();
                    finish(
                        client,
                        events,
                        process,
                        actual,
                        if probed {
                            VersionSource::Probed
                        } else {
                            VersionSource::Declared
                        },
                        &path,
                    )
                    .await;
                } else {
                    assert_eq!(result.err(), Some(Error::Version));
                }
                markers(&path, probed, expected == actual);
            }
        }
    }
}

#[cfg(feature = "bolt_message_id")]
#[tokio::test]
async fn identified_profile_rejects_unreviewed_patch_bases_before_rpc_startup() {
    for actual in ["1.5.1", "1.5.2"] {
        for expected in [pi_rpc::PI_VERSION_REQUIREMENT, actual] {
            for mode in ["declared", "plain", "decorated"] {
                let probed = mode != "declared";
                let output = if mode == "decorated" {
                    format!("{actual} (Pi-Bolt 9.10.11, linux-x64, JIT off)")
                } else {
                    actual.into()
                };
                let (_root, mut options) = prepare(&output, !probed, "normal");
                let path = options.executable.clone();
                options.expected_version = expected.into();
                let result = if probed {
                    PiProcess::<pi_rpc::BoltMessageIdV1>::spawn(options).await
                } else {
                    PiProcess::<pi_rpc::BoltMessageIdV1>::spawn_with_version(options, actual).await
                };
                assert_eq!(
                    result.err(),
                    Some(Error::Version),
                    "{actual}, {expected}, {mode}"
                );
                markers(&path, probed, false);
            }
        }
    }
}

#[cfg(feature = "pi_1_5")]
#[tokio::test]
async fn pi15_idle_and_handled_prompt_shutdown_drain_before_one_terminal() {
    use pi_rpc::protocol::PromptDisposition;
    for version in ["1.5.1", "1.5.2"] {
        for mode in ["idle-exit", "prompt-shutdown"] {
            let (_root, options) = prepare(version, true, mode);
            let path = options.executable.clone();
            let (client, mut events, mut process) =
                PiProcess::<pi_rpc::Pi15>::spawn_with_version(options, version)
                    .await
                    .unwrap();
            ready(&mut events, &path).await;
            if mode == "prompt-shutdown" {
                assert_eq!(
                    client.prompt("/synthetic-shutdown").await.unwrap(),
                    PromptDisposition::Handled
                );
            }
            let terminal = tokio::time::timeout(Duration::from_secs(3), process.wait_terminal())
                .await
                .unwrap();
            assert_eq!(terminal, Error::ProcessExit(Some(0)));
            assert_eq!(events.recv().await, Some(Err(terminal.clone())));
            assert_eq!(events.recv().await, None);
            assert_eq!(client.abort().await, Err(terminal.clone()));
            assert_eq!(process.close().await, terminal);
            assert_eq!(process.process_exit(), Some(Error::ProcessExit(Some(0))));
        }
    }
}

#[cfg(feature = "pi_1_5")]
#[tokio::test]
async fn pi15_orderly_close_drains_cleanup_output_after_stdin_eof() {
    for version in ["1.5.1", "1.5.2"] {
        let (_root, options) = prepare(version, true, "close-cleanup");
        let path = options.executable.clone();
        let (client, mut events, mut process) =
            PiProcess::<pi_rpc::Pi15>::spawn_with_version(options, version)
                .await
                .unwrap();
        ready(&mut events, &path).await;
        assert_eq!(client.get_state().await.unwrap().session_id, "fake");
        let terminal = process.close().await;
        assert_eq!(terminal, Error::ProcessExit(Some(0)));
        assert_eq!(
            events.recv().await,
            Some(Ok(pi_rpc::protocol::v1_5::Event::Unknown(
                serde_json::json!({"type":"synthetic_cleanup_complete"})
            )))
        );
        assert_eq!(events.recv().await, Some(Err(terminal.clone())));
        assert_eq!(events.recv().await, None);
        assert_eq!(client.abort().await, Err(terminal));
    }
}

#[cfg(unix)]
#[test]
fn unix_case_aliases_share_only_the_immutable_executable_inode() {
    use std::os::unix::fs::MetadataExt;
    let (_first_root, first) = prepare("1.3.0", false, "normal");
    let (_second_root, second) = prepare("1.5.0", false, "normal");
    let source = std::fs::metadata(fake_process::fixture()).unwrap();
    for path in [&first.executable, &second.executable] {
        let alias = std::fs::metadata(path).unwrap();
        assert_eq!((alias.dev(), alias.ino()), (source.dev(), source.ino()));
        #[cfg(target_os = "macos")]
        {
            assert_eq!(path.file_name(), fake_process::fixture().file_name());
            assert!(
                std::fs::symlink_metadata(path)
                    .unwrap()
                    .file_type()
                    .is_symlink()
            );
            assert_eq!(
                path.canonicalize().unwrap(),
                fake_process::fixture().canonicalize().unwrap()
            );
            assert_eq!(
                source.nlink(),
                1,
                "macOS source must have one stable hard-link identity"
            );
            assert!(
                path.parent()
                    .unwrap()
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .ends_with(".case")
            );
        }
    }
    assert_ne!(first.executable, second.executable);
    assert_eq!(
        std::fs::read_to_string(first.executable.with_extension("version")).unwrap(),
        "1.3.0"
    );
    assert_eq!(
        std::fs::read_to_string(second.executable.with_extension("version")).unwrap(),
        "1.5.0"
    );
    markers(&first.executable, false, false);
    markers(&second.executable, false, false);
}
