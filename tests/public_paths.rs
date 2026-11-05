//! Compile as an external consumer, including packaged independent consumers.
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;
fn wire<T: Serialize + DeserializeOwned + Debug + Clone + PartialEq>() {}
fn comparable<T: Debug + Clone + PartialEq>() {}
fn same_type<T>(_: Option<T>, _: Option<T>) {}

#[test]
fn protocol_range_and_latest_reviewed_release_remain_public() {
    assert_eq!(pi_rpc::PI_VERSION, "1.5.0");
    assert_eq!(
        pi_rpc::SOURCE_COMMIT,
        "f9a26365568127461c47a07a37dcb42c140a3ac5"
    );
    assert_eq!(pi_rpc::PI_VERSION_REQUIREMENT, ">=1.3.0, <=1.5.0");
    for version in [
        "1.3.0", "1.3.1", "1.3.10", "1.4.0", "1.4.1", "1.4.10", "1.5.0",
    ] {
        let transport =
            pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), version).unwrap();
        assert_eq!(transport.info().version(), version);
        assert_eq!(transport.info().source(), pi_rpc::VersionSource::Declared);
    }
    for version in [
        "0.99.2",
        "1.1.0",
        "1.2.10",
        "1.5.1",
        "1.10.0",
        "v1.3.0",
        "1.3.0 ",
        "",
        "1.3.0-rc.1",
        "1.5.0+build",
        pi_rpc::PI_VERSION_REQUIREMENT,
        ">=1.1.0, <=1.2.0",
        ">=1.1.0, <=1.3.0",
        ">=1.1.0, <=1.4.0",
    ] {
        assert!(
            matches!(
                pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), version),
                Err(pi_rpc::Error::Version)
            ),
            "{version:?}"
        );
    }
    #[cfg(feature = "subprocess")]
    assert_eq!(
        pi_rpc::ProcessOptions::default().expected_version,
        pi_rpc::PI_VERSION_REQUIREMENT
    );
}

#[test]
fn shared_protocol_paths_and_wire_traits_remain_available() {
    macro_rules! wire_types { ($($name:ident),+ $(,)?) => { $(wire::<pi_rpc::protocol::$name>();)+ }; }
    wire_types!(
        Extra,
        Command,
        PromptDisposition,
        QueueMode,
        QueuedInputDisposition,
        StreamingBehavior,
        ThinkingLevel,
        CompactionReason,
        SummarySource,
        ContentBlock,
        Cost,
        ImageContent,
        ImageType,
        Message,
        MessageContent,
        Usage,
        BashResult,
        Commands,
        CompactionResult,
        ExportPath,
        ForkMessage,
        ForkMessages,
        ForkResult,
        LastAssistantText,
        Level,
        Levels,
        Messages,
        ModelCycle,
        Models,
        Queue,
        Response,
        SessionChange,
        SessionState,
        SessionStats,
        SlashCommand,
        SourceInfo,
        TokenTotals,
        ContextReplacement,
        Entries,
        EntryData,
        SessionEntry,
        SessionTreeNode,
        Tree,
        UiMethod,
        UiRequest,
    );
    wire::<pi_rpc::protocol::Disposition<pi_rpc::protocol::PromptDisposition>>();
    comparable::<pi_rpc::protocol::UiAnswer>();
    same_type(None::<pi_rpc::Config>, None::<pi_rpc::client::Config>);
    same_type(None::<pi_rpc::UiPolicy>, None::<pi_rpc::client::UiPolicy>);
    same_type(
        None::<pi_rpc::Transport>,
        None::<pi_rpc::transport::Transport>,
    );
    same_type(
        None::<pi_rpc::ConnectionInfo>,
        None::<pi_rpc::transport::ConnectionInfo>,
    );
    same_type(
        None::<pi_rpc::VersionSource>,
        None::<pi_rpc::transport::VersionSource>,
    );
    same_type(
        None::<pi_rpc::AutoConnection>,
        None::<pi_rpc::client::AutoConnection>,
    );
    let _reader: pi_rpc::transport::Reader = Box::new(tokio::io::empty());
    let _writer: pi_rpc::transport::Writer = Box::new(tokio::io::sink());
    let _jsonl = pi_rpc::codec::JsonlReader::new(tokio::io::empty(), 1024);
    let _depth: usize = pi_rpc::codec::MAX_JSON_DEPTH;
    let _encode: fn(&serde_json::Value, usize) -> pi_rpc::Result<Vec<u8>> = pi_rpc::codec::encode;
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
fn profile_paths<P: pi_rpc::PiProtocol + pi_rpc::protocol::PiProtocol>() {
    fn send<T: Send>() {}
    comparable::<pi_rpc::protocol::Event<P>>();
    comparable::<pi_rpc::protocol::Record<P>>();
    send::<pi_rpc::PiClient<P>>();
    send::<pi_rpc::PiEvents<P>>();
    send::<pi_rpc::PiConnection<P>>();
    same_type(
        None::<pi_rpc::PiClient<P>>,
        None::<pi_rpc::client::PiClient<P>>,
    );
    same_type(
        None::<pi_rpc::PiEvents<P>>,
        None::<pi_rpc::client::PiEvents<P>>,
    );
    same_type(
        None::<pi_rpc::PiConnection<P>>,
        None::<pi_rpc::client::PiConnection<P>>,
    );
    type Connected<P> = (
        pi_rpc::PiClient<P>,
        pi_rpc::PiEvents<P>,
        pi_rpc::PiConnection<P>,
    );
    let _connect: fn(pi_rpc::Transport, pi_rpc::Config) -> pi_rpc::Result<Connected<P>> =
        pi_rpc::PiConnection::<P>::connect;
    let _decode: fn(serde_json::Value) -> pi_rpc::Result<pi_rpc::protocol::Record<P>> =
        pi_rpc::protocol::decode_record::<P>;
    #[cfg(feature = "subprocess")]
    {
        same_type(
            None::<pi_rpc::PiProcess<P>>,
            None::<pi_rpc::subprocess::PiProcess<P>>,
        );
        same_type(
            None::<pi_rpc::AutoProcess>,
            None::<pi_rpc::subprocess::AutoProcess>,
        );
        same_type(
            None::<pi_rpc::ProcessOptions>,
            None::<pi_rpc::subprocess::ProcessOptions>,
        );
        fn future<P: pi_rpc::PiProtocol>(
            _: impl std::future::Future<
                Output = pi_rpc::Result<(
                    pi_rpc::PiClient<P>,
                    pi_rpc::PiEvents<P>,
                    pi_rpc::PiProcess<P>,
                )>,
            > + Send,
        ) {
        }
        let options = || pi_rpc::ProcessOptions {
            executable: "compile-only-never-run-pi".into(),
            ..Default::default()
        };
        future::<P>(pi_rpc::PiProcess::<P>::spawn(options()));
        future::<P>(pi_rpc::subprocess::PiProcess::<P>::spawn_with_version(
            options(),
            pi_rpc::PI_VERSION,
        ));
        fn auto_future(
            _: impl std::future::Future<Output = pi_rpc::Result<pi_rpc::AutoProcess>> + Send,
        ) {
        }
        auto_future(pi_rpc::subprocess::spawn_auto(options()));
        auto_future(pi_rpc::subprocess::spawn_auto_with_version(
            options(),
            pi_rpc::PI_VERSION,
        ));
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! profile_test {
    ($test:ident, $module:ident, $profile:ident) => {
        #[test]
        fn $test() {
            use pi_rpc::protocol::$module::{AssistantEvent, Event, KnownEvent};
            same_type(None::<pi_rpc::$profile>, None::<pi_rpc::protocol::$profile>);
            same_type(
                None::<pi_rpc::$profile>,
                None::<pi_rpc::protocol::$module::$profile>,
            );
            same_type(
                None::<Event>,
                None::<pi_rpc::protocol::Event<pi_rpc::$profile>>,
            );
            wire::<KnownEvent>();
            wire::<AssistantEvent>();
            profile_paths::<pi_rpc::$profile>();
            let delay = 1500_u64;
            for event in [
                KnownEvent::AutoRetryStart {
                    attempt: 1,
                    max_attempts: 3,
                    delay_ms: delay,
                    error_message: "synthetic".into(),
                },
                KnownEvent::SummarizationRetryScheduled {
                    attempt: 1,
                    max_attempts: 3,
                    delay_ms: delay,
                    error_message: "synthetic".into(),
                },
            ] {
                let value: u64 = match event {
                    KnownEvent::AutoRetryStart { delay_ms, .. }
                    | KnownEvent::SummarizationRetryScheduled { delay_ms, .. } => delay_ms,
                    _ => unreachable!(),
                };
                assert_eq!(value, delay);
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
profile_test!(pi13_public_paths, v1_3, Pi13);
#[cfg(feature = "pi_1_4")]
profile_test!(pi14_public_paths, v1_4, Pi14);
#[cfg(feature = "pi_1_5")]
profile_test!(pi15_public_paths, v1_5, Pi15);

#[test]
fn ui_timeouts_keep_the_u64_public_fields() {
    use pi_rpc::protocol::UiMethod;
    let timeout = Some(1500_u64);
    for method in [
        UiMethod::Select {
            title: "t".into(),
            options: vec![],
            timeout,
        },
        UiMethod::Confirm {
            title: "t".into(),
            message: "m".into(),
            timeout,
        },
        UiMethod::Input {
            title: "t".into(),
            placeholder: None,
            timeout,
        },
    ] {
        let value: Option<u64> = match method {
            UiMethod::Select { timeout, .. }
            | UiMethod::Confirm { timeout, .. }
            | UiMethod::Input { timeout, .. } => timeout,
            _ => unreachable!(),
        };
        assert_eq!(value, timeout);
    }
}

#[cfg(feature = "session_file")]
#[test]
fn native_session_paths_remain_available() {
    let _file: Option<pi_rpc::session_file::SessionFile> = None;
    let _header: Option<pi_rpc::session_file::SessionHeader> = None;
    let _kind = pi_rpc::session_file::HeaderType::Session;
    let _limits = pi_rpc::session_file::SessionLimits::default();
    let _builder: Option<pi_rpc::session_file::SessionFileBuilder> = None;
    let _message: fn(
        pi_rpc::protocol::MessageContent,
        u64,
    ) -> pi_rpc::Result<pi_rpc::protocol::Message> = pi_rpc::session_file::user_message;
}
