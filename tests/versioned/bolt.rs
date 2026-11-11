//! Probe-output compatibility, using only the std-only fake executable.
use super::*;

fn output(version: &str) -> String {
    format!("{version} (Pi-Bolt 1.1.1, linux-x64, JIT off)")
}

#[tokio::test]
async fn auto_bolt_probe_selects_the_base_pi_protocol() {
    for (version, target, jit, enabled, feature) in [
        (
            "1.3.0",
            "linux-x64",
            "off",
            cfg!(feature = "pi_1_3"),
            "pi_1_3",
        ),
        (
            "1.3.1",
            "linux-x64-baseline",
            "on",
            cfg!(feature = "pi_1_3"),
            "pi_1_3",
        ),
        (
            "1.3.10",
            "linux-x64-jit",
            "on",
            cfg!(feature = "pi_1_3"),
            "pi_1_3",
        ),
        (
            "1.4.0",
            "linux-arm64",
            "off",
            cfg!(feature = "pi_1_4"),
            "pi_1_4",
        ),
        (
            "1.4.1",
            "linux-arm64-jit",
            "on",
            cfg!(feature = "pi_1_4"),
            "pi_1_4",
        ),
        (
            "1.4.10",
            "darwin-arm64",
            "off",
            cfg!(feature = "pi_1_4"),
            "pi_1_4",
        ),
        (
            "1.5.0",
            "darwin-arm64-jit",
            "on",
            cfg!(feature = "pi_1_5"),
            "pi_1_5",
        ),
        (
            "1.5.1",
            "win32-x64",
            "off",
            cfg!(feature = "pi_1_5"),
            "pi_1_5",
        ),
        (
            "1.5.2",
            "win32-x64-jit",
            "on",
            cfg!(feature = "pi_1_5"),
            "pi_1_5",
        ),
    ] {
        for expected in [pi_rpc::PI_VERSION_REQUIREMENT, version] {
            let decorated = format!(" \t{version} (Pi-Bolt 1.1.1, {target}, JIT {jit})\r\n ");
            let (_root, mut options) = prepare(&decorated, false, "normal");
            let path = options.executable.clone();
            options.expected_version = expected.into();
            let result = pi_rpc::subprocess::spawn_auto(options).await;
            if enabled {
                finish_auto(result.unwrap(), version, VersionSource::Probed, &path).await;
                markers(&path, true, true);
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
                    }),
                );
                markers(&path, !none_enabled(), false);
            }
        }
    }
}

#[tokio::test]
async fn auto_bolt_probe_rejects_malformed_output_and_outside_versions() {
    for decorated in [
        output("1.2.999"),
        output("1.5.3"),
        output("1.10.0"),
        output("01.5.0"),
        output("1.5.0-rc.1"),
        output("1.5.0+build"),
        "1.5.0 (Pi-Bolt 1.1.1)".into(),
        "1.5.0 (Pi-Bolt 1.1.1, linux-x64, JIT off) extra".into(),
        "1.5.0 (Pi-Bolt 1.1.1, linux-x64, JIT off)\n1.5.0".into(),
        "1.5.0 (Pi-Bolt 1.01.1, linux-x64, JIT off)".into(),
        "1.5.0 (Other 1.1.1, linux-x64, JIT off)".into(),
        "1.5.0 (Pi-Bolt 1.1.1, linux--x64, JIT off)".into(),
        "1.5.0 (Pi-Bolt 1.1.1, linux-x64, JIT true)".into(),
    ] {
        let (_root, options) = prepare(&decorated, false, "normal");
        let path = options.executable.clone();
        assert_eq!(
            pi_rpc::subprocess::spawn_auto(options).await.err(),
            Some(if none_enabled() {
                Error::NoProtocolEnabled
            } else {
                Error::Version
            }),
            "{decorated:?}"
        );
        markers(&path, !none_enabled(), false);
    }
}

#[tokio::test]
async fn bolt_strings_are_not_declarations_or_expectations() {
    let decorated = output("1.5.0");
    let (_root, options) = prepare("1.5.0", true, "normal");
    let path = options.executable.clone();
    assert_eq!(
        pi_rpc::subprocess::spawn_auto_with_version(options, &decorated)
            .await
            .err(),
        Some(Error::Version)
    );
    markers(&path, false, false);
    for declared in [false, true] {
        let (_root, mut options) = prepare(&decorated, true, "normal");
        let path = options.executable.clone();
        options.expected_version = decorated.clone();
        let result = if declared {
            pi_rpc::subprocess::spawn_auto_with_version(options, "1.5.0").await
        } else {
            pi_rpc::subprocess::spawn_auto(options).await
        };
        assert_eq!(result.err(), Some(Error::Version));
        markers(&path, false, false);
    }
}

#[tokio::test]
async fn auto_bolt_probe_keeps_legacy_upper_bounds() {
    for (actual, expected) in [
        ("1.3.1", ">=1.1.0, <=1.3.0"),
        ("1.3.10", ">=1.1.0, <=1.3.0"),
        ("1.4.0", ">=1.1.0, <=1.3.0"),
        ("1.4.1", ">=1.1.0, <=1.4.0"),
        ("1.4.10", ">=1.1.0, <=1.4.0"),
        ("1.5.0", ">=1.1.0, <=1.4.0"),
        ("1.5.1", ">=1.3.0, <=1.5.0"),
        ("1.5.2", ">=1.3.0, <=1.5.0"),
    ] {
        let (_root, mut options) = prepare(&output(actual), false, "normal");
        let path = options.executable.clone();
        options.expected_version = expected.into();
        assert_eq!(
            pi_rpc::subprocess::spawn_auto(options).await.err(),
            Some(if none_enabled() {
                Error::NoProtocolEnabled
            } else {
                Error::Version
            })
        );
        markers(&path, !none_enabled(), false);
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! typed_bolt_cases {
    ($module:ident, $profile:ident, $version:literal, $patch:literal, $other:literal, $legacy:literal) => {
        mod $module {
            use super::*;
            type Process = PiProcess<pi_rpc::$profile>;

            #[tokio::test]
            async fn typed_bolt_probe_uses_pi_expectations_and_profile() {
                for version in [$version, $patch] {
                    let mut expectations = vec![pi_rpc::PI_VERSION_REQUIREMENT, version];
                    if version == $version {
                        expectations.push($legacy);
                    }
                    for expected in expectations {
                        let (_root, mut options) = prepare(&output(version), false, "normal");
                        let path = options.executable.clone();
                        options.expected_version = expected.into();
                        let (client, events, process) = Process::spawn(options).await.unwrap();
                        finish(
                            client,
                            events,
                            process,
                            version,
                            VersionSource::Probed,
                            &path,
                        )
                        .await;
                        markers(&path, true, true);
                    }
                }
                // Separate expectation failures from a matching expectation with the wrong profile.
                for (actual, expected) in [($version, $other), ($other, $other)] {
                    let (_root, mut options) = prepare(&output(actual), false, "normal");
                    let path = options.executable.clone();
                    options.expected_version = expected.into();
                    assert_eq!(Process::spawn(options).await.err(), Some(Error::Version));
                    markers(&path, true, false);
                }
            }

            #[tokio::test]
            async fn typed_bolt_strings_are_not_declarations_or_expectations() {
                let decorated = output($version);
                let (_root, options) = prepare($version, true, "normal");
                let path = options.executable.clone();
                assert_eq!(
                    Process::spawn_with_version(options, &decorated).await.err(),
                    Some(Error::Version)
                );
                markers(&path, false, false);
                for declared in [false, true] {
                    let (_root, mut options) = prepare(&decorated, true, "normal");
                    let path = options.executable.clone();
                    options.expected_version = decorated.clone();
                    let result = if declared {
                        Process::spawn_with_version(options, $version).await
                    } else {
                        Process::spawn(options).await
                    };
                    assert_eq!(result.err(), Some(Error::Version));
                    markers(&path, false, false);
                }
            }

            #[tokio::test]
            async fn typed_bolt_probe_preserves_the_inclusive_output_limit() {
                let template = format!("{} (Pi-Bolt 1.1.1, linux-, JIT off)", $version);
                for bytes in [4096, 4097] {
                    // The fake adds LF; all other bytes form a syntactically valid build tag.
                    let target = "a".repeat(bytes - template.len() - 1);
                    let decorated =
                        format!("{} (Pi-Bolt 1.1.1, linux-{target}, JIT off)", $version);
                    assert_eq!(decorated.len() + 1, bytes);
                    let (_root, options) = prepare(&decorated, false, "normal");
                    let path = options.executable.clone();
                    let result = Process::spawn(options).await;
                    if bytes == 4096 {
                        let (client, events, process) = result.unwrap();
                        finish(
                            client,
                            events,
                            process,
                            $version,
                            VersionSource::Probed,
                            &path,
                        )
                        .await;
                    } else {
                        assert_eq!(
                            result.err(),
                            Some(Error::Protocol(
                                "version probe output limit exceeded".into()
                            ))
                        );
                    }
                    markers(&path, true, bytes == 4096);
                }
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
typed_bolt_cases!(pi13, Pi13, "1.3.0", "1.3.10", "1.4.0", ">=1.1.0, <=1.3.0");
#[cfg(feature = "pi_1_4")]
typed_bolt_cases!(pi14, Pi14, "1.4.0", "1.4.10", "1.5.0", ">=1.1.0, <=1.4.0");
#[cfg(feature = "pi_1_5")]
typed_bolt_cases!(pi15, Pi15, "1.5.0", "1.5.2", "1.4.0", ">=1.3.0, <=1.5.0");
#[cfg(feature = "bolt_message_id")]
typed_bolt_cases!(
    identified_bolt,
    BoltMessageIdV1,
    "1.5.0",
    "1.5.0",
    "1.4.0",
    ">=1.3.0, <=1.5.0"
);

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4"))]
async fn prefix_collisions<P: PiProtocol>(short: &str, long: &str)
where
    P::Event: ReadyEvent,
{
    for (actual, expected) in [(short, long), (long, short)] {
        for auto in [false, true] {
            let (_root, mut options) = prepare(&output(actual), false, "normal");
            let path = options.executable.clone();
            options.expected_version = expected.into();
            let error = if auto {
                pi_rpc::subprocess::spawn_auto(options).await.err()
            } else {
                PiProcess::<P>::spawn(options).await.err()
            };
            assert_eq!(
                error,
                Some(Error::Version),
                "expected={expected}, actual={actual}"
            );
            markers(&path, true, false);
            // A matching-lock control must complete RPC, not just pass parsing.
            let (_root, mut options) = prepare(&output(actual), false, "normal");
            let path = options.executable.clone();
            options.expected_version = actual.into();
            if auto {
                let process = pi_rpc::subprocess::spawn_auto(options).await.unwrap();
                finish_auto(process, actual, VersionSource::Probed, &path).await;
            } else {
                let (client, events, process) = PiProcess::<P>::spawn(options).await.unwrap();
                finish(
                    client,
                    events,
                    process,
                    actual,
                    VersionSource::Probed,
                    &path,
                )
                .await;
            }
            markers(&path, true, true);
        }
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4"))]
#[tokio::test]
async fn bolt_exact_patch_prefix_collisions_reject_before_rpc() {
    #[cfg(feature = "pi_1_3")]
    prefix_collisions::<pi_rpc::Pi13>("1.3.1", "1.3.10").await;
    #[cfg(feature = "pi_1_4")]
    prefix_collisions::<pi_rpc::Pi14>("1.4.1", "1.4.10").await;
}
