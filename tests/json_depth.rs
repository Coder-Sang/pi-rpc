mod support;
use pi_rpc::{
    Config, Error, PI_VERSION, PiConnection, Transport,
    codec::{self, JsonlReader, MAX_JSON_DEPTH},
    protocol::{Event, Record, Response, Tree, decode_record},
};
use serde_json::{Value, json};
use std::{process::Command, time::Duration};
use tokio::io::AsyncWriteExt;

async fn parse(wire: &str) -> pi_rpc::Result<Value> {
    JsonlReader::new(wire.as_bytes(), 65536)
        .next()
        .await
        .map(|r| r.unwrap().0)
}

#[tokio::test]
async fn container_depth_boundary_and_strict_syntax() {
    assert_eq!(MAX_JSON_DEPTH, 512);
    let value = parse(&(support::nested(512) + "\r\n")).await.unwrap();
    let encoded = codec::encode(&value, 65536).unwrap();
    assert_eq!(
        parse(std::str::from_utf8(&encoded).unwrap()).await.unwrap(),
        value
    );
    let error = parse(&(support::nested(513) + "\n")).await.unwrap_err();
    assert!(matches!(&error, Error::Protocol(s) if s.contains("nesting depth")));
    for wire in ["{} true\n", "[}\n", "{\"a\":\n", "\"unterminated\n"] {
        assert!(matches!(parse(wire).await, Err(Error::Protocol(_))));
    }
    // Brackets and escaped quotes/backslashes inside a string do not count.
    let text = "[{}]\\\"".repeat(600);
    let expected = json!({"type":"future_event","text":text});
    let wire = codec::encode(&expected, 65536).unwrap();
    assert_eq!(
        parse(std::str::from_utf8(&wire).unwrap()).await.unwrap(),
        expected
    );
}

#[tokio::test]
async fn deep_unknown_event_fragmented_input_and_terminal_order() {
    let (local, remote) = tokio::io::duplex(65536);
    let (r, w) = tokio::io::split(local);
    let (_client, mut events, mut owner) =
        PiConnection::connect(Transport::new(r, w, PI_VERSION).unwrap(), Config::default())
            .unwrap();
    let (_r, mut w) = tokio::io::split(remote);
    let wire = format!(
        "{{\"type\":\"future_event\",\"payload\":{}}}\r\n",
        support::nested(511)
    );
    for chunk in wire.as_bytes().chunks(7) {
        w.write_all(chunk).await.unwrap();
    }
    let event = tokio::time::timeout(Duration::from_secs(2), events.recv())
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    assert!(matches!(&event, Event::Unknown(v) if v["type"] == "future_event"));
    let error_wire = format!(
        "{{\"type\":\"future_event\",\"payload\":{}}}\n",
        support::nested(512)
    );
    w.write_all(error_wire.as_bytes()).await.unwrap();
    let error = tokio::time::timeout(Duration::from_secs(2), owner.wait_terminal())
        .await
        .unwrap();
    assert!(matches!(&error, Error::Protocol(s) if s.contains("nesting depth")));
    assert_eq!(events.recv().await, Some(Err(error)));
    assert_eq!(events.recv().await, None);
}

#[tokio::test]
async fn long_tree_commands_complete_without_terminating_connection() {
    for depth in [70, 100, 200] {
        let (local, remote) = tokio::io::duplex(65536);
        let (r, w) = tokio::io::split(local);
        let (client, _events, owner) =
            PiConnection::connect(Transport::new(r, w, PI_VERSION).unwrap(), Config::default())
                .unwrap();
        let (r, mut w) = tokio::io::split(remote);
        let command = tokio::spawn(async move { client.get_tree().await });
        let mut reader = JsonlReader::new(r, 65536);
        let request = reader.next().await.unwrap().unwrap().0;
        w.write_all(support::tree_response(depth, &request["id"].to_string()).as_bytes())
            .await
            .unwrap();
        let tree = tokio::time::timeout(Duration::from_secs(2), command)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        let mut node = &tree.tree[0];
        for _ in 1..depth {
            node = &node.children[0];
        }
        assert_eq!(node.entry.id, (depth - 1).to_string());
        assert_eq!(owner.terminal_reason(), None);
    }
}

#[test]
fn direct_value_entrypoints_enforce_depth() {
    let mut v = Value::Null;
    for _ in 0..10000 {
        v = Value::Array(vec![v]);
    }
    // Owned values must be destroyed iteratively even when much deeper than the cap.
    assert!(matches!(decode_record(v), Err(Error::Protocol(s)) if s.contains("nesting depth")));
    let mut v = Value::Null;
    for _ in 0..513 {
        v = Value::Array(vec![v]);
    }
    let r = Response {
        id: None,
        command: "custom".into(),
        success: true,
        data: Some(v),
        error: None,
    };
    assert!(matches!(r.decode::<Value>(), Err(Error::Protocol(_))));
}

#[test]
fn stack_safety_in_isolated_process() {
    const CHILD: &str = "PI_RPC_JSON_STACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async {
                let wire = format!("{{\"type\":\"future_event\",\"payload\":{}}}\n", support::nested(511));
                let value = parse(&wire).await.unwrap();
                let cloned = value.clone();
                let record = decode_record(value).unwrap();
                let record_clone = record.clone();
                codec::encode(&cloned, 65536).unwrap();
                drop((record, record_clone, cloned));
                let objects = parse(&(support::nested_objects(512) + "\n"))
                    .await
                    .unwrap();
                let clone = objects.clone();
                codec::encode(&objects, 65536).unwrap();
                drop((objects, clone));
                let value = parse(&support::tree_response(250, "\"test\"")).await.unwrap();
                let Record::Response(response) = decode_record(value).unwrap() else { panic!("response") };
                let tree: Tree = response.decode().unwrap();
                let clone = tree.clone();
                codec::encode(&tree, 65536).unwrap();
                serde_json::to_vec(&clone).unwrap();
                drop((tree, clone, response.clone(), response));
                // Exercise partial typed conversion cleanup and trailing-data rejection.
                let mut invalid = parse(&support::tree_response(200, "\"test\"")).await.unwrap();
                let mut node = &mut invalid["data"]["tree"][0];
                for _ in 1..200 {
                    node = &mut node["children"][0];
                }
                node["entry"]["id"] = json!(false);
                assert!(decode_record(invalid).is_err());
                assert!(parse(&(support::nested(512)+" true\n")).await.is_err());
                #[cfg(feature="session_file")]
                {
                    use pi_rpc::session_file::{SessionFile, SessionLimits};
                    let wire = format!("{{\"type\":\"session\",\"version\":3,\"id\":\"s\",\"timestamp\":\"t\",\"cwd\":\"/p\"}}\n{{\"type\":\"message\",\"id\":\"e\",\"parentId\":null,\"timestamp\":\"t\",\"message\":{{\"role\":\"user\",\"content\":\"test\",\"timestamp\":1,\"opaque\":{}}}}}\n", support::nested(510));
                    let file = SessionFile::parse(wire.as_bytes(), SessionLimits::default()).unwrap();
                    let cloned = file.clone();
                    let bytes = file.encode(SessionLimits::default()).unwrap();
                    assert_eq!(SessionFile::parse(&bytes, SessionLimits::default()).unwrap(), cloned);
                    drop((file, cloned));
                }
            });
        }).unwrap().join().unwrap();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "stack_safety_in_isolated_process", "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stack child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn owned_rejection_and_unpolled_futures_are_stack_safe() {
    const CHILD: &str = "PI_RPC_OWNED_STACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::thread::Builder::new()
            .stack_size(2 * 1024 * 1024)
            .spawn(|| {
                fn deep() -> Value {
                    let mut value = Value::Null;
                    for _ in 0..100_000 {
                        value = Value::Array(vec![value]);
                    }
                    value
                }
                fn images() -> Option<Vec<pi_rpc::protocol::ImageContent>> {
                    let mut image: pi_rpc::protocol::ImageContent = serde_json::from_value(
                        json!({"type":"image", "data":"", "mimeType":"image/png"}),
                    )
                    .unwrap();
                    image.extra.insert("opaque".into(), deep());
                    Some(vec![image])
                }
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                runtime.block_on(async {
                    use pi_rpc::protocol::Command as RpcCommand;
                    let (local, _remote) = tokio::io::duplex(1024);
                    let (r, w) = tokio::io::split(local);
                    let (client, _events, _owner) = PiConnection::connect(
                        Transport::new(r, w, PI_VERSION).unwrap(),
                        Config::default(),
                    )
                    .unwrap();
                    let command = || RpcCommand::Prompt {
                        message: "test".into(),
                        images: images(),
                        streaming_behavior: None,
                    };
                    assert!(matches!(
                        client.command(command()).await,
                        Err(Error::Protocol(_))
                    ));
                    drop(client.command(command()));
                    drop(client.command_with_timeout(command(), Duration::from_secs(1)));
                    drop(client.typed::<Value>(command()));
                    drop(client.prompt_with("test", images(), None));
                    drop(client.steer("test", images()));
                    drop(client.follow_up("test", images()));
                    #[cfg(feature = "session_file")]
                    {
                        use pi_rpc::{
                            protocol::{EntryData, Extra, Message, SessionEntry},
                            session_file::{SessionFile, SessionFileBuilder},
                        };
                        let message = || {
                            let mut fields = Extra::new();
                            fields.insert("content".into(), json!("test"));
                            fields.insert("timestamp".into(), json!(1));
                            fields.insert("opaque".into(), deep());
                            Message {
                                role: "user".into(),
                                fields,
                            }
                        };
                        let mut builder = SessionFileBuilder::new("s", "t", "/p").unwrap();
                        assert!(builder.append_message("a", None, "t", message()).is_err());
                        assert!(builder.append_message("", None, "t", message()).is_err());
                        assert!(
                            builder
                                .append_message("a", Some("missing".into()), "t", message())
                                .is_err()
                        );
                        assert!(builder.finish().unwrap().entries.is_empty());
                        let mut file = SessionFile::new("s", "t", "/p");
                        assert!(file.append_message("a", None, "t", message()).is_err());
                        assert!(file.entries.is_empty());
                        file.entries.push(SessionEntry {
                            id: "a".into(),
                            parent_id: None,
                            timestamp: "t".into(),
                            data: EntryData::Message { message: message() },
                        });
                        assert!(SessionFileBuilder::from_session_file(file).is_err());
                        let mut file = SessionFile::new("s", "t", "/p");
                        file.header.extra.insert("opaque".into(), deep());
                        assert!(SessionFileBuilder::from_session_file(file).is_err());
                    }
                });
            })
            .unwrap()
            .join()
            .unwrap();
        return;
    }
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "owned_rejection_and_unpolled_futures_are_stack_safe",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "owned stack child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}
