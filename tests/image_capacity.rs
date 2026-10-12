use pi_rpc::{
    Config, Error, PI_VERSION, PiConnection, Transport,
    codec::{self, JsonlReader},
    protocol::*,
};
use serde_json::{Value, json};
use tokio::io::AsyncWriteExt;

fn images(count: usize, bytes: usize) -> Vec<ImageContent> {
    (0..count)
        .map(|_| ImageContent {
            r#type: ImageType::Image,
            data: "A".repeat(bytes),
            mime_type: "image/png".into(),
            extra: Default::default(),
        })
        .collect()
}
fn message(bytes: usize) -> Value {
    json!({"role":"user","timestamp":0,"content":[{"type":"image","data":"A".repeat(bytes),"mimeType":"image/png"}]})
}
fn entry(bytes: usize) -> Value {
    json!({"type":"message","id":"image","parentId":null,"timestamp":"test","message":message(bytes)})
}

#[tokio::test]
async fn four_near_pi_limit_images_and_fifteen_mib_original_are_submitted() {
    for input in [
        images(4, 9 * 1024 * 1024 / 2 - 4096),
        images(1, 20 * 1024 * 1024),
        images(2, 20 * 1024 * 1024),
    ] {
        let expected: Vec<_> = input.iter().map(|i| i.data.len()).collect();
        let accepted = expected.iter().sum::<usize>() < Config::default().max_record_bytes;
        let (local, remote) = tokio::io::duplex(65536);
        let (r, w) = tokio::io::split(local);
        let (client, mut events, mut owner) =
            PiConnection::connect(Transport::new(r, w, PI_VERSION).unwrap(), Config::default())
                .unwrap();
        let peer = tokio::spawn(async move {
            let (r, mut w) = tokio::io::split(remote);
            let mut reader = JsonlReader::new(r, Config::default().max_record_bytes);
            let next = reader.next().await.unwrap();
            if !accepted {
                assert!(next.is_none());
                return;
            }
            let (command, _) = next.unwrap();
            let got: Vec<_> = command["images"]
                .as_array()
                .unwrap()
                .iter()
                .map(|i| i["data"].as_str().unwrap().len())
                .collect();
            assert_eq!(got, expected);
            assert_eq!(command["message"], "text\n\"中文");
            w.write_all(&codec::encode(&json!({"type":"response","id":command["id"],"command":"prompt","success":true,"data":{"disposition":"started"}}),4096).unwrap()).await.unwrap();
            assert!(reader.next().await.unwrap().is_none());
        });
        let result = client.prompt_with("text\n\"中文", Some(input), None).await;
        if accepted {
            assert_eq!(result.unwrap(), PromptDisposition::Started);
        } else {
            assert_eq!(result, Err(Error::Capacity("record bytes")));
            assert_eq!(owner.terminal_reason(), None);
        }
        assert_eq!(owner.close().await, Error::Closed);
        peer.await.unwrap();
        assert_eq!(events.recv().await, Some(Err(Error::Closed)));
        assert_eq!(events.recv().await, None);
    }
}

#[tokio::test]
async fn all_image_input_methods_obey_explicit_small_limits_without_submission() {
    let (local, remote) = tokio::io::duplex(1024);
    let (r, w) = tokio::io::split(local);
    let (client, _events, mut owner) = PiConnection::connect(
        Transport::new(r, w, PI_VERSION).unwrap(),
        Config {
            max_record_bytes: 256,
            ..Config::default()
        },
    )
    .unwrap();
    assert_eq!(
        client.prompt_with("text", Some(images(2, 256)), None).await,
        Err(Error::Capacity("record bytes"))
    );
    assert_eq!(
        client.steer("text", Some(images(2, 256))).await,
        Err(Error::Capacity("record bytes"))
    );
    assert_eq!(
        client.follow_up("text", Some(images(2, 256))).await,
        Err(Error::Capacity("record bytes"))
    );
    assert_eq!(owner.terminal_reason(), None);
    let peer = tokio::spawn(async move {
        let (r, mut w) = tokio::io::split(remote);
        let mut reader = JsonlReader::new(r, 1024);
        let command = reader.next().await.unwrap().unwrap().0;
        assert_eq!(command["type"], "get_messages");
        w.write_all(&codec::encode(&json!({"type":"response","id":command["id"],"command":"get_messages","success":true,"data":{"messages":[]}}),1024).unwrap()).await.unwrap();
        assert!(reader.next().await.unwrap().is_none());
    });
    assert!(client.get_messages().await.unwrap().messages.is_empty());
    assert_eq!(owner.close().await, Error::Closed);
    peer.await.unwrap();
}

#[tokio::test]
async fn image_start_end_are_complete_ordered_and_independently_budgeted() {
    for overflow in [false, true] {
        let first = json!({"type":"message_start","message":message(9 * 1024 * 1024 / 2 - 4096)});
        let last = json!({"type":"message_end","message":message(9 * 1024 * 1024 / 2 - 4096)});
        let first_wire = codec::encode(&first, Config::default().max_record_bytes).unwrap();
        let last_wire = codec::encode(&last, Config::default().max_record_bytes).unwrap();
        let bytes = first_wire.len() + last_wire.len() - 2;
        let (local, remote) = tokio::io::duplex(65536);
        let (r, w) = tokio::io::split(local);
        let (_, mut events, mut owner) = PiConnection::connect(
            Transport::new(r, w, PI_VERSION).unwrap(),
            Config {
                max_event_bytes: bytes - usize::from(overflow),
                ..Config::default()
            },
        )
        .unwrap();
        let (_, mut writer) = tokio::io::split(remote);
        let mut batch = first_wire;
        batch.extend(last_wire);
        writer.write_all(&batch).await.unwrap();
        if overflow {
            assert_eq!(owner.wait_terminal().await, Error::EventOverflow);
            assert!(matches!(
                events.recv().await,
                Some(Ok(Event::Known(KnownEvent::MessageStart { .. })))
            ));
            assert_eq!(events.recv().await, Some(Err(Error::EventOverflow)));
            assert_eq!(events.recv().await, None);
        } else {
            for start in [true, false] {
                let got = events.recv().await.unwrap().unwrap();
                let m = match got {
                    Event::Known(KnownEvent::MessageStart { message }) if start => message,
                    Event::Known(KnownEvent::MessageEnd { message }) if !start => message,
                    _ => panic!("event order"),
                };
                assert_eq!(
                    m.fields["content"][0]["data"],
                    "A".repeat(9 * 1024 * 1024 / 2 - 4096)
                );
            }
            // A subsequent pair fits only if dequeue returned both byte permits.
            writer.write_all(&batch).await.unwrap();
            assert!(events.recv().await.unwrap().is_ok());
            assert!(events.recv().await.unwrap().is_ok());
            drop(writer);
            assert_eq!(owner.wait_terminal().await, Error::Eof);
            assert_eq!(events.recv().await, Some(Err(Error::Eof)));
            assert_eq!(events.recv().await, None);
        }
    }
}

#[tokio::test]
async fn complete_image_history_responses_accept_exact_limit_and_reject_one_over() {
    for (command, data) in [
        (
            Command::GetMessages,
            json!({"messages":[message(9 * 1024 * 1024 / 4),message(9 * 1024 * 1024 / 4)]}),
        ),
        (
            Command::GetEntries { since: None },
            json!({"entries":[entry(9 * 1024 * 1024 / 2 - 4096)],"leafId":"image"}),
        ),
        (
            Command::GetTree,
            json!({"tree":[{"entry":entry(9 * 1024 * 1024 / 2 - 4096),"children":[]}],"leafId":"image"}),
        ),
    ] {
        for over in [false, true] {
            let name = command.name();
            let response = json!({"type":"response","id":"pi-rpc-1","command":name,"success":true,"data":data});
            let wire = codec::encode(&response, Config::default().max_record_bytes).unwrap();
            let limit = wire.len() - 1 - usize::from(over);
            let (local, remote) = tokio::io::duplex(65536);
            let (r, w) = tokio::io::split(local);
            let (client, mut events, mut owner) = PiConnection::connect(
                Transport::new(r, w, PI_VERSION).unwrap(),
                Config {
                    max_record_bytes: limit,
                    ..Config::default()
                },
            )
            .unwrap();
            let peer = tokio::spawn(async move {
                let (r, mut w) = tokio::io::split(remote);
                let mut reader = JsonlReader::new(r, 4096);
                assert!(reader.next().await.unwrap().is_some());
                w.write_all(&wire).await.unwrap();
                let _ = reader.next().await;
            });
            let result = client.command(command.clone()).await;
            if over {
                let err = result.unwrap_err();
                assert!(matches!(&err,Error::Protocol(s) if s.contains("record byte")));
                assert_eq!(owner.wait_terminal().await, err);
                assert_eq!(events.recv().await, Some(Err(err)));
                assert_eq!(events.recv().await, None);
            } else {
                assert_eq!(result.unwrap().data.unwrap(), data);
                assert_eq!(owner.close().await, Error::Closed);
            }
            peer.await.unwrap();
        }
    }
}

#[tokio::test]
async fn image_commands_include_envelope_escaping_and_lf_in_exact_budgets() {
    for command in [
        Command::Prompt {
            message: "text\n\"中文".into(),
            images: Some(images(2, 256)),
            streaming_behavior: None,
        },
        Command::Steer {
            message: "text\n\"中文".into(),
            images: Some(images(2, 256)),
        },
        Command::FollowUp {
            message: "text\n\"中文".into(),
            images: Some(images(2, 256)),
        },
    ] {
        let mut envelope = serde_json::to_value(&command).unwrap();
        envelope["id"] = json!("pi-rpc-1");
        let framed = codec::encode(&envelope, 4096).unwrap().len();
        for over in [false, true] {
            let (local, remote) = tokio::io::duplex(4096);
            let (r, w) = tokio::io::split(local);
            let (client, mut events, mut owner) = PiConnection::connect(
                Transport::new(r, w, PI_VERSION).unwrap(),
                Config {
                    max_record_bytes: framed - 1 - usize::from(over),
                    max_writer_bytes: framed,
                    max_writer_records: 1,
                    ..Config::default()
                },
            )
            .unwrap();
            let expected = envelope.clone();
            let name = command.name();
            let peer = tokio::spawn(async move {
                let (r, mut w) = tokio::io::split(remote);
                let mut reader = JsonlReader::new(r, 4096);
                let next = reader.next().await.unwrap();
                if over {
                    assert!(next.is_none());
                    return;
                }
                assert_eq!(next.unwrap().0, expected);
                let disposition = if name == "prompt" {
                    "started"
                } else {
                    "queued"
                };
                w.write_all(&codec::encode(&json!({"type":"response","id":"pi-rpc-1","command":name,"success":true,"data":{"disposition":disposition}}),4096).unwrap()).await.unwrap();
                assert!(reader.next().await.unwrap().is_none());
            });
            let result = client.command(command.clone()).await;
            if over {
                assert_eq!(result, Err(Error::Capacity("record bytes")));
            } else {
                assert!(result.unwrap().success);
            }
            assert_eq!(owner.close().await, Error::Closed);
            peer.await.unwrap();
            assert_eq!(events.recv().await, Some(Err(Error::Closed)));
            assert_eq!(events.recv().await, None);
        }
    }
}

#[cfg(feature = "session_file")]
#[test]
fn native_image_record_exceeding_old_limit_roundtrips_with_new_default() {
    use pi_rpc::session_file::{SessionFile, SessionLimits};
    let mut file = SessionFile::new("images", "timestamp", "/scratch");
    file.entries
        .push(serde_json::from_value(entry(9 * 1024 * 1024 / 2 - 4096)).unwrap());
    let wire = file.encode(SessionLimits::default()).unwrap();
    let parsed = SessionFile::parse(&wire, SessionLimits::default()).unwrap();
    assert_eq!(parsed, file);
    assert!(
        SessionFile::parse(
            &wire,
            SessionLimits {
                max_record_bytes: 4 * 1024 * 1024,
                ..SessionLimits::default()
            }
        )
        .is_err()
    );
}
