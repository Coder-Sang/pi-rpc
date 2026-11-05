//! Version admission and automatic dispatch never inspect event contents.
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use pi_rpc::{AutoConnection, VersionSource};
use pi_rpc::{Config, Error, Transport, client::connect_auto};
use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
};

struct ObservedReader(Arc<AtomicUsize>, Arc<AtomicUsize>);
impl tokio::io::AsyncRead for ObservedReader {
    fn poll_read(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        _: &mut tokio::io::ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.0.fetch_add(1, Ordering::SeqCst);
        Poll::Pending
    }
}
impl Drop for ObservedReader {
    fn drop(&mut self) {
        self.1.fetch_add(1, Ordering::SeqCst);
    }
}

#[tokio::test]
async fn auto_selection_obeys_each_compiled_protocol_without_fallback() {
    for (version, feature, enabled) in [
        ("1.3.0", "pi_1_3", cfg!(feature = "pi_1_3")),
        ("1.3.10", "pi_1_3", cfg!(feature = "pi_1_3")),
        ("1.4.0", "pi_1_4", cfg!(feature = "pi_1_4")),
        ("1.4.10", "pi_1_4", cfg!(feature = "pi_1_4")),
        ("1.5.0", "pi_1_5", cfg!(feature = "pi_1_5")),
    ] {
        let polls = Arc::new(AtomicUsize::new(0));
        let drops = Arc::new(AtomicUsize::new(0));
        let transport = Transport::new(
            ObservedReader(polls.clone(), drops.clone()),
            tokio::io::sink(),
            version,
        )
        .unwrap();
        let result = connect_auto(transport, Config::default());
        if enabled {
            #[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
            {
                let owner = result.unwrap_or_else(|error| panic!("{version}: {error}"));
                assert_eq!(owner.info().version(), version);
                assert_eq!(owner.info().source(), VersionSource::Declared);
                match &owner {
                    #[cfg(feature = "pi_1_3")]
                    AutoConnection::V1_3 {
                        client,
                        events,
                        connection,
                    } => {
                        assert_eq!(feature, "pi_1_3");
                        assert_eq!(client.info(), events.info());
                        assert_eq!(client.info(), connection.info());
                    }
                    #[cfg(feature = "pi_1_4")]
                    AutoConnection::V1_4 {
                        client,
                        events,
                        connection,
                    } => {
                        assert_eq!(feature, "pi_1_4");
                        assert_eq!(client.info(), events.info());
                        assert_eq!(client.info(), connection.info());
                    }
                    #[cfg(feature = "pi_1_5")]
                    AutoConnection::V1_5 {
                        client,
                        events,
                        connection,
                    } => {
                        assert_eq!(feature, "pi_1_5");
                        assert_eq!(client.info(), events.info());
                        assert_eq!(client.info(), connection.info());
                    }
                    _ => panic!("unexpected protocol variant"),
                }
                drop(owner);
            }
            #[cfg(not(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5")))]
            panic!("an enabled profile was reported in a no-protocol build");
        } else {
            let expected = if cfg!(any(
                feature = "pi_1_3",
                feature = "pi_1_4",
                feature = "pi_1_5"
            )) {
                Error::VersionNotEnabled {
                    version: version.into(),
                    feature,
                }
            } else {
                Error::NoProtocolEnabled
            };
            assert_eq!(result.err(), Some(expected));
            assert_eq!(
                polls.load(Ordering::SeqCst),
                0,
                "rejection started a reader"
            );
            assert_eq!(
                drops.load(Ordering::SeqCst),
                1,
                "rejection retained the transport"
            );
        }
    }
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
async fn command_event_close<P: pi_rpc::PiProtocol>(version: &str, expected: Vec<P::Event>) {
    use tokio::io::AsyncWriteExt;
    let (local, remote) = tokio::io::duplex(128);
    let (reader, writer) = tokio::io::split(local);
    let (client, mut events, mut owner) = pi_rpc::PiConnection::<P>::connect(
        Transport::new(reader, writer, version).unwrap(),
        Config::default(),
    )
    .unwrap();
    let original_info = owner.info().clone();
    assert_eq!(client.info(), &original_info);
    assert_eq!(events.info(), &original_info);
    let peer = tokio::spawn(async move {
        let (reader, mut writer) = tokio::io::split(remote);
        let mut reader = pi_rpc::codec::JsonlReader::new(reader, 4096);
        let (command, _) = reader.next().await.unwrap().unwrap();
        assert_eq!(command["type"], "abort");
        let records = [
            serde_json::json!({"type":"agent_start"}),
            serde_json::json!({"type":"extension_ui_request","id":"ui","method":"confirm","title":"title","message":"message","timeout":12.5}),
            serde_json::json!({"type":"future_event","opaque":{"escaped":"中\\\"[]{}"}}),
            serde_json::json!({"type":"response","id":command["id"],"command":"abort","success":true}),
        ];
        for record in records {
            let mut bytes = pi_rpc::codec::encode(&record, 4096).unwrap();
            bytes.pop();
            bytes.extend_from_slice(b"\r\n");
            for fragment in bytes.chunks(3) {
                writer.write_all(fragment).await.unwrap();
            }
        }
        let (reply, _) = reader.next().await.unwrap().unwrap();
        assert_eq!(
            reply,
            serde_json::json!({"type":"extension_ui_response","id":"ui","cancelled":true})
        );
        assert!(reader.next().await.unwrap().is_none());
    });
    client.abort().await.unwrap();
    for event in expected {
        assert_eq!(events.recv().await, Some(Ok(event)));
    }
    assert_eq!(owner.close().await, Error::Closed);
    peer.await.unwrap();
    assert_eq!(owner.info(), &original_info);
    assert_eq!(events.recv().await, Some(Err(Error::Closed)));
    assert_eq!(events.recv().await, None);
    assert_eq!(client.abort().await, Err(Error::Closed));
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! profile_cases {
    ($name:ident, $module:ident, $profile:ident, $version:literal, $patch:literal, $other:literal) => {
        mod $name {
            use super::*;
            use pi_rpc::protocol::$module::{Event, KnownEvent};
            #[tokio::test]
            async fn commands_events_ui_and_close_keep_the_selected_type() {
                for version in [$version, $patch] {
                    let expected = vec![
                        Event::Known(KnownEvent::AgentStart),
                        Event::Ui(serde_json::from_value(serde_json::json!({"type":"extension_ui_request","id":"ui","method":"confirm","title":"title","message":"message","timeout":12.5})).unwrap()),
                        Event::Unknown(serde_json::json!({"type":"future_event","opaque":{"escaped":"中\\\"[]{}"}})),
                    ];
                    tokio::time::timeout(std::time::Duration::from_secs(3), command_event_close::<pi_rpc::$profile>(version, expected)).await.unwrap();
                }
            }
            #[tokio::test]
            async fn mismatched_profile_rejects_before_io_and_releases_transport() {
                let polls = Arc::new(AtomicUsize::new(0));
                let drops = Arc::new(AtomicUsize::new(0));
                let transport = Transport::new(ObservedReader(polls.clone(), drops.clone()), tokio::io::sink(), $other).unwrap();
                let result = pi_rpc::PiConnection::<pi_rpc::$profile>::connect(transport, Config::default());
                assert_eq!(result.err(), Some(Error::Version));
                assert_eq!(polls.load(Ordering::SeqCst), 0);
                assert_eq!(drops.load(Ordering::SeqCst), 1);
            }
        }
    };
}
#[cfg(feature = "pi_1_3")]
profile_cases!(pi13, v1_3, Pi13, "1.3.0", "1.3.10", "1.4.0");
#[cfg(feature = "pi_1_4")]
profile_cases!(pi14, v1_4, Pi14, "1.4.0", "1.4.10", "1.5.0");
#[cfg(feature = "pi_1_5")]
profile_cases!(pi15, v1_5, Pi15, "1.5.0", "1.5.0", "1.4.0");

#[cfg(feature = "subprocess")]
#[path = "versioned/process.rs"]
mod process;
