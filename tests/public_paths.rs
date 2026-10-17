//! Compile as an external consumer in every feature configuration. Explicit
//! paths keep unreferenced re-exports from disappearing behind private modules.
use serde::{Serialize, de::DeserializeOwned};
use std::fmt::Debug;

fn wire<T: Serialize + DeserializeOwned + Debug + Clone + PartialEq>() {}
fn comparable<T: Debug + Clone + PartialEq>() {}
fn same_type<T>(_: Option<T>, _: Option<T>) {}

#[test]
fn protocol_range_and_latest_reviewed_release_remain_public() {
    assert_eq!(pi_rpc::PI_VERSION, "1.3.0");
    assert_eq!(
        pi_rpc::SOURCE_COMMIT,
        "71023aec1546d6d677a65669c5665c0927ce0dd0"
    );
    assert_eq!(pi_rpc::codec::MAX_JSON_DEPTH, 512);
    assert_eq!(pi_rpc::PI_VERSION_REQUIREMENT, ">=1.1.0, <=1.3.0");
    for version in [
        "1.1.0", "1.1.1", "1.1.10", "1.2.0", "1.2.1", "1.2.10", "1.3.0",
    ] {
        assert!(pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), version).is_ok());
    }
    for version in [
        "0.99.2",
        "1.0.4",
        "1.3.1",
        "1.10.0",
        "v1.3.0",
        "1.3.0 ",
        "",
        "1.3.0-rc.1",
        "1.3.0+build",
        pi_rpc::PI_VERSION_REQUIREMENT,
    ] {
        assert!(matches!(
            pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), version),
            Err(pi_rpc::Error::Version)
        ));
    }
    #[cfg(feature = "subprocess")]
    assert_eq!(
        pi_rpc::ProcessOptions::default().expected_version,
        pi_rpc::PI_VERSION_REQUIREMENT
    );
}

#[test]
fn ui_timeouts_and_retry_delays_keep_the_original_u64_public_fields() {
    use pi_rpc::protocol::{KnownEvent, UiMethod};
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

#[test]
fn all_original_protocol_paths_and_wire_traits_remain_available() {
    macro_rules! wire_types {
        ($($name:ident),+ $(,)?) => {
            $(wire::<pi_rpc::protocol::$name>();)+
        };
    }
    wire_types!(
        Extra,
        Command,
        PromptDisposition,
        QueueMode,
        QueuedInputDisposition,
        StreamingBehavior,
        ThinkingLevel,
        AssistantEvent,
        CompactionReason,
        KnownEvent,
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
    comparable::<pi_rpc::protocol::Event>();
    comparable::<pi_rpc::protocol::Record>();
    comparable::<pi_rpc::protocol::UiAnswer>();
    let _decode: fn(serde_json::Value) -> pi_rpc::Result<pi_rpc::protocol::Record> =
        pi_rpc::protocol::decode_record;
}

#[test]
fn root_aliases_and_client_transport_paths_remain_identical() {
    same_type(None::<pi_rpc::Config>, None::<pi_rpc::client::Config>);
    same_type(None::<pi_rpc::PiClient>, None::<pi_rpc::client::PiClient>);
    same_type(None::<pi_rpc::PiEvents>, None::<pi_rpc::client::PiEvents>);
    same_type(
        None::<pi_rpc::PiConnection>,
        None::<pi_rpc::client::PiConnection>,
    );
    same_type(None::<pi_rpc::UiPolicy>, None::<pi_rpc::client::UiPolicy>);
    same_type(
        None::<pi_rpc::Transport>,
        None::<pi_rpc::transport::Transport>,
    );
    type Connected = (pi_rpc::PiClient, pi_rpc::PiEvents, pi_rpc::PiConnection);
    let _connect: fn(pi_rpc::Transport, pi_rpc::Config) -> pi_rpc::Result<Connected> =
        pi_rpc::client::PiConnection::connect;
    let _depth: usize = pi_rpc::codec::MAX_JSON_DEPTH;
    let _version: &str = pi_rpc::PI_VERSION;
    let _commit: &str = pi_rpc::SOURCE_COMMIT;
    let _reader: pi_rpc::transport::Reader = Box::new(tokio::io::empty());
    let _writer: pi_rpc::transport::Writer = Box::new(tokio::io::sink());
    let _jsonl = pi_rpc::codec::JsonlReader::new(tokio::io::empty(), 1024);
    let _encode: fn(&serde_json::Value, usize) -> pi_rpc::Result<Vec<u8>> = pi_rpc::codec::encode;
    let _error: pi_rpc::Error = pi_rpc::Error::Closed;
}

#[cfg(feature = "subprocess")]
#[test]
fn subprocess_paths_and_root_aliases_remain_available() {
    same_type(
        None::<pi_rpc::PiProcess>,
        None::<pi_rpc::subprocess::PiProcess>,
    );
    same_type(
        None::<pi_rpc::ProcessOptions>,
        None::<pi_rpc::subprocess::ProcessOptions>,
    );
    let _diagnostics: Option<pi_rpc::subprocess::Diagnostics> = None;
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
