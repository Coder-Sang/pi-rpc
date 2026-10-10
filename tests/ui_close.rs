use pi_rpc::{
    Config, Error, PI_VERSION, PiConnection, Transport, UiPolicy,
    codec::JsonlReader,
    protocol::{Event, KnownEvent, UiAnswer},
};
use serde_json::{Value, json};
use std::{future::Future, task::Poll, time::Duration};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};

#[tokio::test]
async fn dialogs_after_stdin_shutdown_preserve_responses_events_and_terminal() {
    for policy in [UiPolicy::Cancel, UiPolicy::Manual] {
        let (local, remote) = tokio::io::duplex(65536);
        let (r, w) = tokio::io::split(local);
        let (client, mut events, mut owner) = PiConnection::connect(
            Transport::new(r, w, PI_VERSION).unwrap(),
            Config {
                ui_policy: policy,
                ..Default::default()
            },
        )
        .unwrap();
        let (r, mut w) = tokio::io::split(remote);
        let mut peer_reader = JsonlReader::new(r, 65536);
        let c = client.clone();
        let command = tokio::spawn(async move { c.abort().await });
        let request = peer_reader.next().await.unwrap().unwrap().0;
        let closing = tokio::spawn(async move { owner.close().await });
        assert!(
            tokio::time::timeout(Duration::from_secs(2), peer_reader.next())
                .await
                .unwrap()
                .unwrap()
                .is_none()
        );
        let mut wire = Vec::new();
        for (index, mut dialog) in [
            json!({"method":"confirm","title":"t","message":"m"}),
            json!({"method":"select","title":"t","options":["a"]}),
            json!({"method":"input","title":"t"}),
            json!({"method":"editor","title":"t"}),
        ]
        .into_iter()
        .enumerate()
        {
            dialog["type"] = json!("extension_ui_request");
            dialog["id"] = json!(format!("ui-{index}"));
            wire.extend(pi_rpc::codec::encode(&dialog, 65536).unwrap());
        }
        wire.extend(
            pi_rpc::codec::encode(
                &json!({"type":"response","id":request["id"],"command":"abort","success":true}),
                65536,
            )
            .unwrap(),
        );
        wire.extend_from_slice(b"{\"type\":\"agent_start\"}\n");
        w.write_all(&wire).await.unwrap();
        w.shutdown().await.unwrap();
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), command)
                .await
                .unwrap()
                .unwrap(),
            Ok(())
        );
        for index in 0..4 {
            let Some(Ok(Event::Ui(dialog))) = events.recv().await else {
                panic!("dialog must survive close")
            };
            assert_eq!(dialog.id, format!("ui-{index}"));
            assert_eq!(
                client.respond_ui(&dialog, UiAnswer::Cancel).await,
                Err(Error::Closed)
            );
        }
        assert_eq!(
            events.recv().await,
            Some(Ok(Event::Known(KnownEvent::AgentStart)))
        );
        assert_eq!(events.recv().await, Some(Err(Error::Closed)));
        assert_eq!(events.recv().await, None);
        assert_eq!(
            tokio::time::timeout(Duration::from_secs(2), closing)
                .await
                .unwrap()
                .unwrap(),
            Error::Closed
        );
    }
}

#[tokio::test]
async fn late_dialog_while_admitted_writer_is_stalled_does_not_abort_drain() {
    let (local, remote) = tokio::io::duplex(1);
    let (r, w) = tokio::io::split(local);
    let (client, mut events, mut owner) =
        PiConnection::connect(Transport::new(r, w, PI_VERSION).unwrap(), Config::default())
            .unwrap();
    let (mut r, mut w) = tokio::io::split(remote);
    let c = client.clone();
    let command = tokio::spawn(async move { c.abort().await });
    let mut bytes = vec![0];
    r.read_exact(&mut bytes).await.unwrap(); // Writer is now blocked on the rest.
    let mut closing = Box::pin(owner.close());
    std::future::poll_fn(|cx| {
        assert!(closing.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await; // Establish closing admission without relying on scheduling/sleep.
    assert_eq!(client.abort().await, Err(Error::Closed));
    w.write_all(b"{\"type\":\"extension_ui_request\",\"id\":\"u\",\"method\":\"confirm\",\"title\":\"t\",\"message\":\"m\"}\n").await.unwrap();
    assert!(matches!(
        tokio::time::timeout(Duration::from_secs(2), events.recv())
            .await
            .unwrap(),
        Some(Ok(Event::Ui(_)))
    ));
    assert!(!command.is_finished());
    let mut reader = BufReader::new(r);
    reader.read_until(b'\n', &mut bytes).await.unwrap();
    let request: Value = serde_json::from_slice(&bytes).unwrap();
    let mut extra = Vec::new();
    assert_eq!(reader.read_to_end(&mut extra).await.unwrap(), 0); // No late UI reply.
    let response = json!({"type":"response","id":request["id"],"command":"abort","success":true});
    w.write_all(&pi_rpc::codec::encode(&response, 65536).unwrap())
        .await
        .unwrap();
    w.shutdown().await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), command)
            .await
            .unwrap()
            .unwrap(),
        Ok(())
    );
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), closing)
            .await
            .unwrap(),
        Error::Closed
    );
    assert_eq!(events.recv().await, Some(Err(Error::Closed)));
    assert_eq!(events.recv().await, None);
}

#[tokio::test]
async fn open_connection_auto_cancel_capacity_error_remains_terminal() {
    let (local, remote) = tokio::io::duplex(4096);
    let (r, w) = tokio::io::split(local);
    let (client, mut events, mut owner) = PiConnection::connect(
        Transport::new(r, w, PI_VERSION).unwrap(),
        Config {
            max_writer_bytes: 1,
            ..Default::default()
        },
    )
    .unwrap();
    let (_r, mut w) = tokio::io::split(remote);
    w.write_all(b"{\"type\":\"extension_ui_request\",\"id\":\"u\",\"method\":\"confirm\",\"title\":\"t\",\"message\":\"m\"}\n").await.unwrap();
    assert_eq!(
        tokio::time::timeout(Duration::from_secs(2), owner.wait_terminal())
            .await
            .unwrap(),
        Error::Capacity("writer bytes")
    );
    assert_eq!(
        events.recv().await,
        Some(Err(Error::Capacity("writer bytes")))
    );
    assert_eq!(events.recv().await, None);
    assert_eq!(client.abort().await, Err(Error::Capacity("writer bytes")));
}
