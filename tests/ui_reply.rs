use pi_rpc::{
    Config, Error, PI_VERSION, PiConnection, Transport, UiPolicy,
    codec::{self, JsonlReader},
    protocol::{UiAnswer, UiRequest},
};
use serde_json::json;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

fn request(method: serde_json::Value) -> UiRequest {
    let mut method = method;
    method["id"] = json!("ui-\n\"中");
    serde_json::from_value(method).unwrap()
}

#[tokio::test]
async fn borrowed_replies_preserve_wire_fields_and_exact_record_limits() {
    for (request, answer) in [
        (
            json!({"method":"confirm","title":"t","message":"m"}),
            UiAnswer::Confirmed(true),
        ),
        (
            json!({"method":"confirm","title":"t","message":"m"}),
            UiAnswer::Confirmed(false),
        ),
        (
            json!({"method":"confirm","title":"t","message":"m"}),
            UiAnswer::Cancel,
        ),
        (
            json!({"method":"select","title":"t","options":["a"]}),
            UiAnswer::Value("a".into()),
        ),
        (
            json!({"method":"select","title":"t","options":["a"]}),
            UiAnswer::Cancel,
        ),
        (
            json!({"method":"input","title":"t"}),
            UiAnswer::Value(String::new()),
        ),
        (
            json!({"method":"input","title":"t"}),
            UiAnswer::Value("你好\n\"\\".into()),
        ),
        (json!({"method":"input","title":"t"}), UiAnswer::Cancel),
        (
            json!({"method":"editor","title":"t"}),
            UiAnswer::Value("你好\n\"\\".into()),
        ),
        (json!({"method":"editor","title":"t"}), UiAnswer::Cancel),
    ] {
        let request = self::request(request);
        let expected = answer.record(&request).unwrap();
        let size = codec::encode(&expected, 65536).unwrap().len() - 1;
        for limit in [size - 1, size] {
            let (local, peer) = tokio::io::duplex(4096);
            let (r, w) = tokio::io::split(local);
            let (client, _events, owner) = PiConnection::connect(
                Transport::new(r, w, PI_VERSION).unwrap(),
                Config {
                    max_record_bytes: limit,
                    ui_policy: UiPolicy::Manual,
                    ..Default::default()
                },
            )
            .unwrap();
            let mut reader = JsonlReader::new(peer, 65536);
            if limit < size {
                assert_eq!(
                    client.respond_ui(&request, answer.clone()).await,
                    Err(Error::Capacity("record bytes"))
                );
                drop(owner);
                assert!(
                    tokio::time::timeout(Duration::from_secs(2), reader.next())
                        .await
                        .unwrap()
                        .unwrap()
                        .is_none()
                );
            } else {
                let c = client.clone();
                let request = request.clone();
                let answer = answer.clone();
                let reply = tokio::spawn(async move { c.respond_ui(&request, answer).await });
                let (value, wire_size) =
                    tokio::time::timeout(Duration::from_secs(2), reader.next())
                        .await
                        .unwrap()
                        .unwrap()
                        .unwrap();
                assert_eq!(value, expected);
                assert_eq!(wire_size, size);
                assert_eq!(reply.await.unwrap(), Ok(()));
                drop(owner);
            }
        }
    }
}

#[tokio::test]
async fn oversized_manual_text_is_not_submitted_and_connection_stays_usable() {
    for method in ["input", "editor"] {
        let request = request(json!({"method":method,"title":"t"}));
        let (local, peer) = tokio::io::duplex(4096);
        let (r, w) = tokio::io::split(local);
        let (client, _events, owner) = PiConnection::connect(
            Transport::new(r, w, PI_VERSION).unwrap(),
            Config {
                max_record_bytes: 256,
                ui_policy: UiPolicy::Manual,
                ..Default::default()
            },
        )
        .unwrap();
        let (r, mut w) = tokio::io::split(peer);
        let mut reader = JsonlReader::new(r, 65536);
        assert_eq!(
            client
                .respond_ui(&request, UiAnswer::Value("\"\\中".repeat(256 * 1024)))
                .await,
            Err(Error::Capacity("record bytes"))
        );
        assert_eq!(owner.terminal_reason(), None);
        let c = client.clone();
        let command = tokio::spawn(async move { c.abort().await });
        let (record, _) = tokio::time::timeout(Duration::from_secs(2), reader.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(record["type"], "abort"); // No rejected UI record precedes it.
        let response =
            json!({"type":"response","command":"abort","id":record["id"],"success":true});
        w.write_all(&codec::encode(&response, 65536).unwrap())
            .await
            .unwrap();
        assert_eq!(command.await.unwrap(), Ok(()));
        let c = client.clone();
        let reply =
            tokio::spawn(async move { c.respond_ui(&request, UiAnswer::Value("ok".into())).await });
        let (record, _) = tokio::time::timeout(Duration::from_secs(2), reader.next())
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            record,
            json!({"type":"extension_ui_response","id":"ui-\n\"中","value":"ok"})
        );
        assert_eq!(reply.await.unwrap(), Ok(()));
        drop(owner);
    }
}

#[tokio::test]
async fn terminal_reason_precedes_ui_validation_and_record_capacity() {
    for eof in [false, true] {
        let (local, mut peer) = tokio::io::duplex(4096);
        let (r, w) = tokio::io::split(local);
        let (client, mut events, owner) = PiConnection::connect(
            Transport::new(r, w, PI_VERSION).unwrap(),
            Config {
                max_record_bytes: 256,
                ui_policy: UiPolicy::Manual,
                ..Default::default()
            },
        )
        .unwrap();
        let mut owner = Some(owner);
        let expected = if eof {
            peer.shutdown().await.unwrap();
            assert_eq!(
                tokio::time::timeout(
                    Duration::from_secs(2),
                    owner.as_mut().unwrap().wait_terminal()
                )
                .await
                .unwrap(),
                Error::Eof
            );
            Error::Eof
        } else {
            drop(owner.take());
            Error::Closed
        };
        for method in [
            json!({"method":"input","title":"t"}),
            json!({"method":"editor","title":"t"}),
            json!({"method":"confirm","title":"t","message":"m"}),
            json!({"method":"select","title":"t","options":["allowed"]}),
        ] {
            let request = request(method);
            assert_eq!(
                client
                    .respond_ui(&request, UiAnswer::Value("x".repeat(1024 * 1024)))
                    .await,
                Err(expected.clone())
            );
        }
        assert_eq!(events.recv().await, Some(Err(expected)));
        assert_eq!(events.recv().await, None);
        drop(owner);
    }
}
