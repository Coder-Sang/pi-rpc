//! Synthetic fork-extension tests. No installed Pi/Bolt or provider is used.
#![cfg(feature = "bolt_message_id")]
use pi_rpc::protocol::bolt_message_id_v1::{
    ClearedInputs, Event, InputSource, MessageId, QueueChange, QueueTrackingV1, QueueUpdate,
    TrackingVersion,
};
use pi_rpc::protocol::{
    self, PromptDisposition, QueuedInputDisposition, Record, StreamingBehavior,
};
use pi_rpc::{BoltMessageIdV1, Config, Error, PiClient, PiConnection, PiEvents, Transport};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncWriteExt, DuplexStream};

fn id(text: &str) -> MessageId {
    MessageId::new(text).unwrap()
}
fn queue(reason: &str, source: &str, ids: Value) -> Value {
    let mut tracking = json!({"version":1,"instanceId":"instance-A","sequence":7,
        "change":{"reason":reason,"source":source,"messageIds":ids},
        "steeringMessageIds":[],"followUpMessageIds":[]});
    if reason == "clear" {
        tracking["change"].as_object_mut().unwrap().remove("source");
    }
    if matches!(reason, "consume" | "prompt_start") {
        tracking["batchId"] = json!("opaque-batch");
    }
    json!({"type":"queue_update","steering":[],"followUp":[],"tracking":tracking})
}
fn decoded(value: Value) -> pi_rpc::Result<QueueUpdate> {
    match protocol::decode_record::<BoltMessageIdV1>(value)? {
        Record::Event(Event::QueueUpdate(queue)) => Ok(queue),
        other => panic!("unexpected record: {other:?}"),
    }
}
type Client = PiClient<BoltMessageIdV1>;
type Events = PiEvents<BoltMessageIdV1>;
type Owner = PiConnection<BoltMessageIdV1>;
fn connection(config: Config) -> (Client, Events, Owner, DuplexStream) {
    let (local, peer) = tokio::io::duplex(4096);
    let (read, write) = tokio::io::split(local);
    let (client, events, owner) =
        Owner::connect(Transport::new(read, write, "1.5.0").unwrap(), config).unwrap();
    (client, events, owner, peer)
}
async fn bounded<F: std::future::Future>(future: F) -> F::Output {
    tokio::time::timeout(Duration::from_secs(3), future)
        .await
        .expect("synthetic peer deadline")
}

#[test]
fn extension_base_stays_exact_when_official_pi15_support_expands() {
    use pi_rpc::PiProtocol;
    assert!(BoltMessageIdV1::matches_version("1.5.0"));
    for version in ["1.5.0", "1.5.1", "1.5.2"] {
        assert!(pi_rpc::Pi15::matches_version(version));
        assert_eq!(
            BoltMessageIdV1::matches_version(version),
            version == "1.5.0"
        );
    }
    for version in [
        "1.3.0",
        "1.4.0",
        "1.5.3",
        "1.5.10",
        "1.6.0",
        "01.5.0",
        "1.5.0 ",
        "1.5.0-rc.1",
        "1.5.0+build",
        "1.5.0 (Pi-Bolt 1.1.1, linux-x64, JIT off)",
    ] {
        assert!(!BoltMessageIdV1::matches_version(version), "{version}");
    }
}

#[test]
fn ids_preserve_bytes_and_clones_share_the_allocation() {
    assert!(MessageId::new("").is_err());
    for text in [
        " ",
        "\t\r\n",
        "input-🦀-中-\"\\\0",
        "same-text-different-id",
    ] {
        let value = id(text);
        let cloned = value.clone();
        assert_eq!(value.as_str().as_ptr(), cloned.as_str().as_ptr());
        assert_eq!(
            serde_json::from_str::<MessageId>(&serde_json::to_string(&value).unwrap()).unwrap(),
            value
        );
        assert_eq!(value.as_ref(), text);
    }
    for invalid in [json!(null), json!(0), json!(""), json!([])] {
        assert!(serde_json::from_value::<MessageId>(invalid).is_err());
    }
}

#[test]
fn tracking_reasons_null_positions_and_opaque_batches_round_trip() {
    for (reason, source, ids) in [
        ("enqueue", "steer", json!(["A", null, "B"])),
        ("enqueue", "followUp", json!(["C"])),
        ("consume", "steer", json!(["A", null, "B"])),
        ("consume", "followUp", json!(["C", "D"])),
        ("prompt_start", "prompt", json!([null, "P", null])),
        ("clear", "", json!({"steering":["A",null],"followUp":["B"]})),
    ] {
        let wire = queue(reason, source, ids);
        let value = decoded(wire.clone()).unwrap();
        assert_eq!(
            value.tracking.as_ref().unwrap().version,
            TrackingVersion::V1
        );
        let mut encoded = serde_json::to_value(value).unwrap();
        encoded["type"] = json!("queue_update");
        assert_eq!(encoded, wire);
    }
    let mut wire = queue("enqueue", "followUp", json!(["A", "B"]));
    wire["steering"] = json!(["unidentified"]);
    wire["followUp"] = json!(["duplicate text", "duplicate text"]);
    wire["tracking"]["steeringMessageIds"] = json!([null]);
    wire["tracking"]["followUpMessageIds"] = json!(["A", "B"]);
    let update = decoded(wire).unwrap();
    assert_eq!(
        update.tracking.unwrap().follow_up_message_ids,
        vec![Some(id("A")), Some(id("B"))]
    );
    assert_eq!(
        decoded(json!({"type":"queue_update","steering":[],"followUp":[]}))
            .unwrap()
            .tracking,
        None
    );
}

#[test]
fn malformed_tracking_is_never_downgraded_to_unknown_or_untracked() {
    let valid = queue("consume", "followUp", json!(["B", "C"]));
    let mut cases = Vec::new();
    for key in [
        "version",
        "instanceId",
        "sequence",
        "change",
        "steeringMessageIds",
        "followUpMessageIds",
        "batchId",
    ] {
        let mut wire = valid.clone();
        wire["tracking"].as_object_mut().unwrap().remove(key);
        cases.push(wire);
    }
    for (pointer, value) in [
        ("/tracking", json!(null)),
        ("/tracking/version", json!(2)),
        ("/tracking/version", json!(1.0)),
        ("/tracking/instanceId", json!("")),
        ("/tracking/sequence", json!(-1)),
        ("/tracking/sequence", json!(0.5)),
        ("/tracking/batchId", json!(null)),
        ("/tracking/batchId", json!("")),
        ("/tracking/change/source", json!("prompt")),
        ("/tracking/change/reason", json!("future")),
        ("/tracking/change/messageIds", json!([""])),
        ("/tracking/change/messageIds", json!([1])),
        ("/tracking/steeringMessageIds", json!([null])),
        ("/tracking/followUpMessageIds", json!(["B"])),
    ] {
        let mut wire = valid.clone();
        *wire.pointer_mut(pointer).unwrap() = value;
        cases.push(wire);
    }
    let mut enqueue = queue("enqueue", "steer", json!(["A"]));
    enqueue["tracking"]["batchId"] = json!("unexpected");
    cases.push(enqueue);
    cases.push(queue("prompt_start", "steer", json!(["A"])));
    cases.push(queue("clear", "", json!(["A"])));
    for wire in cases {
        assert!(
            matches!(
                protocol::decode_record::<BoltMessageIdV1>(wire.clone()),
                Err(Error::Protocol(_))
            ),
            "accepted {wire}"
        );
        // Enabling the opt-in feature must not tighten the official Pi15 schema.
        assert!(matches!(
            protocol::decode_record::<pi_rpc::Pi15>(wire).unwrap(),
            Record::Event(protocol::v1_5::Event::Known(
                protocol::v1_5::KnownEvent::QueueUpdate { .. }
            ))
        ));
    }
}

#[tokio::test]
async fn identified_inputs_use_separate_rpc_ids_and_preserve_existing_payloads() {
    bounded(async {
        let (client, mut events, mut owner, peer) = connection(Config::default());
        let peer = tokio::spawn(async move {
            let (read, mut write) = tokio::io::split(peer);
            let mut reader = pi_rpc::codec::JsonlReader::new(read,4096);
            let mut rpc_ids = Vec::new();
            for command in ["prompt","steer","follow_up","prompt","steer","follow_up"] {
                let (request,_) = reader.next().await.unwrap().unwrap();
                assert_eq!(request["type"], command);
                assert_eq!(request["message"], "same text");
                assert!(request["id"].as_str().unwrap().starts_with("pi-rpc-"));
                assert!(!rpc_ids.contains(&request["id"]));
                rpc_ids.push(request["id"].clone());
                let mut data = json!({"disposition":"handled"});
                if rpc_ids.len() <= 3 {
                    assert_eq!(request["message_id"], "input-\"🦀\\\n");
                    assert_ne!(request["message_id"],request["id"]);
                    assert_eq!(request["images"][0]["data"], "AA==");
                    assert_eq!(request["images"][0]["mimeType"],"image/png");
                    if command == "prompt" { assert_eq!(request["streamingBehavior"],"followUp"); }
                    data["message_id"] = request["message_id"].clone();
                } else {
                    assert!(request.get("message_id").is_none());
                    assert!(request.get("images").is_none());
                }
                let response = json!({"type":"response","id":request["id"],"command":command,"success":true,"data":data});
                write.write_all(&pi_rpc::codec::encode(&response,4096).unwrap()).await.unwrap();
            }
            assert!(reader.next().await.unwrap().is_none());
        });
        let image = || Some(vec![serde_json::from_value(json!({"type":"image","data":"AA==","mimeType":"image/png"})).unwrap()]);
        let input = id("input-\"🦀\\\n");
        assert_eq!(client.prompt_with_message_id("same text",image(),Some(StreamingBehavior::FollowUp),input.clone()).await.unwrap().disposition,PromptDisposition::Handled);
        assert_eq!(client.steer_with_message_id("same text",image(),input.clone()).await.unwrap().message_id,input);
        assert_eq!(client.follow_up_with_message_id("same text",image(),input).await.unwrap().disposition,QueuedInputDisposition::Handled);
        client.prompt("same text").await.unwrap();
        client.steer("same text",None).await.unwrap();
        client.follow_up("same text",None).await.unwrap();
        assert_eq!(owner.close().await, Error::Closed);
        peer.await.unwrap();
        assert_eq!(events.recv().await,Some(Err(Error::Closed)));
        assert_eq!(events.recv().await,None);
    }).await;
}

#[tokio::test]
async fn fragmented_crlf_tracking_precedes_acceptance_and_keeps_wire_order() {
    // Build the golden values directly from the wire contract. Never decode
    // or serialize the fixture to derive an expected value with the same bug.
    fn expected(
        instance: &str,
        sequence: u64,
        batch: Option<&str>,
        change: QueueChange,
        steering: &[(&str, Option<&str>)],
        follow_up: &[(&str, Option<&str>)],
    ) -> Event {
        Event::QueueUpdate(QueueUpdate {
            steering: steering.iter().map(|(text, _)| (*text).into()).collect(),
            follow_up: follow_up.iter().map(|(text, _)| (*text).into()).collect(),
            tracking: Some(QueueTrackingV1 {
                version: TrackingVersion::V1,
                instance_id: instance.into(),
                sequence,
                batch_id: batch.map(String::from),
                change,
                steering_message_ids: steering.iter().map(|(_, value)| value.map(id)).collect(),
                follow_up_message_ids: follow_up.iter().map(|(_, value)| value.map(id)).collect(),
            }),
        })
    }
    bounded(async {
        let (client, mut events, mut owner, peer) = connection(Config::default());
        let mut before = vec![queue("prompt_start","prompt",json!([null,"P"])), queue("enqueue","steer",json!(["A",null,"B"])),queue("consume","steer",json!(["A",null,"B"])), queue("consume","followUp",json!(["C","D"])),queue("clear","",json!({"steering":[],"followUp":["E"]}))];
        for (index, wire) in before.iter_mut().enumerate() { wire["tracking"]["sequence"] = json!(41+index); }
        before[0]["tracking"]["batchId"] = json!("initial-handoff");
        before[1]["steering"] = json!(["same text","same text","same text"]);
        before[1]["tracking"]["steeringMessageIds"] = json!(["A",null,"B"]);
        for wire in &mut before[1..3] {
            wire["followUp"] = json!(["later-1","later-2","cancelled"]);
            wire["tracking"]["followUpMessageIds"] = json!(["C","D","E"]);
        }
        before[2]["tracking"]["batchId"] = json!("steering-drain");
        before[3]["tracking"]["batchId"] = json!("follow-up-drain");
        before[3]["followUp"] = json!(["cancelled"]);
        before[3]["tracking"]["followUpMessageIds"] = json!(["E"]);
        // Session instance replacement resets sequence; the client must preserve it.
        let mut restarted = queue("enqueue","followUp",json!(["new"]));
        restarted["tracking"]["instanceId"] = json!("instance-B");
        restarted["tracking"]["sequence"] = json!(0);
        restarted["followUp"] = json!(["new input"]);
        restarted["tracking"]["followUpMessageIds"] = json!(["new"]);
        before.push(restarted);
        let expected = [
            expected("instance-A",41,Some("initial-handoff"),QueueChange::PromptStart {
                source:InputSource::Prompt,message_ids:vec![None,Some(id("P"))],
            },&[],&[]),
            expected("instance-A",42,None,QueueChange::Enqueue {
                source:InputSource::Steer,message_ids:vec![Some(id("A")),None,Some(id("B"))],
            },&[("same text",Some("A")),("same text",None),("same text",Some("B"))],
              &[("later-1",Some("C")),("later-2",Some("D")),("cancelled",Some("E"))]),
            expected("instance-A",43,Some("steering-drain"),QueueChange::Consume {
                source:InputSource::Steer,message_ids:vec![Some(id("A")),None,Some(id("B"))],
            },&[],&[("later-1",Some("C")),("later-2",Some("D")),("cancelled",Some("E"))]),
            expected("instance-A",44,Some("follow-up-drain"),QueueChange::Consume {
                source:InputSource::FollowUp,message_ids:vec![Some(id("C")),Some(id("D"))],
            },&[],&[("cancelled",Some("E"))]),
            expected("instance-A",45,None,QueueChange::Clear {
                message_ids:ClearedInputs {steering:vec![],follow_up:vec![Some(id("E"))]},
            },&[],&[]),
            expected("instance-B",0,None,QueueChange::Enqueue {
                source:InputSource::FollowUp,message_ids:vec![Some(id("new"))],
            },&[],&[("new input",Some("new"))]),
        ];
        let (seen_tx,seen_rx) = tokio::sync::oneshot::channel();
        let (release_tx,release_rx) = tokio::sync::oneshot::channel();
        let peer = tokio::spawn(async move {
            let (read,mut write)=tokio::io::split(peer);
            let mut reader=pi_rpc::codec::JsonlReader::new(read,4096);
            let (request,_) = reader.next().await.unwrap().unwrap();
            for wire in before {
                let mut bytes=pi_rpc::codec::encode(&wire,4096).unwrap(); bytes.pop(); bytes.extend_from_slice(b"\r\n");
                for fragment in bytes.chunks(3) { write.write_all(fragment).await.unwrap(); }
            }
            seen_tx.send(()).unwrap();
            release_rx.await.unwrap();
            let response=json!({"type":"response","command":"prompt","id":request["id"],"success":true,"data":{"disposition":"started","message_id":"P"}});
            write.write_all(&pi_rpc::codec::encode(&response,4096).unwrap()).await.unwrap();
            assert!(reader.next().await.unwrap().is_none());
        });
        let submission=tokio::spawn({let client=client.clone();async move {client.prompt_with_message_id("input",None,None,id("P")).await}});
        seen_rx.await.unwrap();
        for event in expected {assert_eq!(events.recv().await,Some(Ok(event)));}
        assert!(!submission.is_finished(),"peer has not sent acceptance yet");
        release_tx.send(()).unwrap();
        assert_eq!(submission.await.unwrap().unwrap().disposition,PromptDisposition::Started);
        assert_eq!(owner.close().await,Error::Closed);
        peer.await.unwrap();
        assert_eq!(events.recv().await,Some(Err(Error::Closed)));
        assert_eq!(events.recv().await,None);
    }).await;
}

async fn check_echo(command: &'static str, requested: &str, echoed: Option<&str>, accepted: bool) {
    let (client, mut events, mut owner, peer) = connection(Config::default());
    let echo = echoed.map(String::from);
    let expected_request = requested.to_owned();
    let peer = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(peer);
        let mut reader = pi_rpc::codec::JsonlReader::new(read, 4096);
        let (request, _) = reader.next().await.unwrap().unwrap();
        assert_eq!(request["type"], command);
        assert_eq!(request["message_id"], expected_request);
        let mut data = json!({"disposition":"queued"});
        if let Some(id) = echo {
            data["message_id"] = json!(id);
        }
        let response = json!({"type":"response","id":request["id"],"command":command,"success":true,"data":data});
        write
            .write_all(&pi_rpc::codec::encode(&response, 4096).unwrap())
            .await
            .unwrap();
        let (request, _) = reader.next().await.unwrap().unwrap();
        assert_eq!(request["type"], "abort", "identified input was retried");
        let response =
            json!({"type":"response","id":request["id"],"command":"abort","success":true});
        write
            .write_all(&pi_rpc::codec::encode(&response, 4096).unwrap())
            .await
            .unwrap();
        assert!(reader.next().await.unwrap().is_none());
    });
    let result = match command {
        "prompt" => client
            .prompt_with_message_id("input", None, None, id(requested))
            .await
            .map(|value| value.message_id),
        "steer" => client
            .steer_with_message_id("input", None, id(requested))
            .await
            .map(|value| value.message_id),
        "follow_up" => client
            .follow_up_with_message_id("input", None, id(requested))
            .await
            .map(|value| value.message_id),
        _ => panic!("unexpected command"),
    };
    if accepted {
        assert_eq!(result.unwrap().as_str(), requested);
    } else {
        assert!(
            matches!(result, Err(Error::Protocol(_))),
            "accepted echo {echoed:?} for {requested:?}"
        );
    }
    // Well-formed missing/mismatched echoes affect this call, not connection
    // liveness. Reading the next command also proves no automatic resubmission.
    client.abort().await.unwrap();
    assert_eq!(owner.close().await, Error::Closed);
    peer.await.unwrap();
    assert_eq!(events.recv().await, Some(Err(Error::Closed)));
    assert_eq!(events.recv().await, None);
}

#[tokio::test]
async fn missing_or_mismatched_echo_has_uncertain_outcome_and_is_not_retried() {
    for command in ["prompt", "steer", "follow_up"] {
        for echo in [None, Some("other")] {
            bounded(check_echo(command, "expected", echo, false)).await;
        }
    }
}

#[tokio::test]
async fn identified_echoes_require_exact_bytes_without_retry() {
    for command in ["prompt", "steer", "follow_up"] {
        for (requested, echoed, accepted) in [
            ("input-1", "input-1", true),
            ("input-10", "input-10", true),
            (" input-1 \t\r\n", " input-1 \t\r\n", true),
            (" \t", " \t", true),
            ("🦀", "🦀", true),
            ("input-1", "input-10", false),
            ("input-10", "input-1", false),
            ("input-1", "input-1 ", false),
            ("input-1 ", "input-1", false),
            ("input-1", " input-1", false),
            (" input-1", "input-1", false),
            (" \t", " ", false),
            (" ", " \t", false),
            ("input-A", "input-a", false),
            ("input-a", "input-A", false),
            ("\u{e9}", "e\u{301}", false),
            ("e\u{301}", "\u{e9}", false),
        ] {
            bounded(check_echo(command, requested, Some(echoed), accepted)).await;
        }
    }
}

#[tokio::test]
async fn malformed_echo_or_tracking_terminates_once_and_rejects_later_inputs() {
    bounded(async {
        for malformed in [json!(""),json!(null),json!(123),json!({"queue":true})] {
            let (client,mut events,mut owner,peer)=connection(Config::default());
            let peer=tokio::spawn(async move {
                let (read,mut write)=tokio::io::split(peer);let mut reader=pi_rpc::codec::JsonlReader::new(read,4096);
                let (request,_)=reader.next().await.unwrap().unwrap();
                let response=if malformed.is_object(){json!({"type":"queue_update","steering":[],"followUp":[],"tracking":null})}else{json!({"type":"response","id":request["id"],"command":"follow_up","success":true,"data":{"disposition":"queued","message_id":malformed}})};
                write.write_all(&pi_rpc::codec::encode(&response,4096).unwrap()).await.unwrap();
            });
            let error=client.follow_up_with_message_id("input",None,id("A")).await.unwrap_err();
            assert!(matches!(error,Error::Protocol(_)));
            assert_eq!(owner.wait_terminal().await,error);
            assert_eq!(client.steer_with_message_id("later",None,id("B")).await.unwrap_err(),error);
            assert_eq!(events.recv().await,Some(Err(error)));assert_eq!(events.recv().await,None);
            peer.await.unwrap();
        }
    }).await;
}

#[tokio::test]
async fn extension_keeps_official_events_ui_unknown_fields_and_auto_dispatch() {
    bounded(async {
        let (client,mut events,mut owner,peer)=connection(Config::default());
        let peer=tokio::spawn(async move {
            let (read,mut write)=tokio::io::split(peer);let mut reader=pi_rpc::codec::JsonlReader::new(read,4096);
            let (request,_)=reader.next().await.unwrap().unwrap();assert_eq!(request["type"],"abort");
            for wire in [json!({"type":"agent_start"}),json!({"type":"extension_ui_request","id":"ui","method":"confirm","title":"T","message":"M","timeout":0.25}),json!({"type":"future_event","opaque":{"message_id":"not-an-input","number":f64::from_bits(1)}}),json!({"type":"response","id":request["id"],"command":"abort","success":true})] {
                write.write_all(&pi_rpc::codec::encode(&wire,4096).unwrap()).await.unwrap();
            }
            let (reply,_)=reader.next().await.unwrap().unwrap();assert_eq!(reply,json!({"type":"extension_ui_response","id":"ui","cancelled":true}));
            assert!(reader.next().await.unwrap().is_none());
        });
        client.abort().await.unwrap();
        assert_eq!(events.recv().await,Some(Ok(Event::Pi(protocol::v1_5::Event::Known(protocol::v1_5::KnownEvent::AgentStart)))));
        assert!(matches!(events.recv().await,Some(Ok(Event::Pi(protocol::v1_5::Event::Ui(_))))));
        let Some(Ok(Event::Pi(protocol::v1_5::Event::Unknown(value))))=events.recv().await else{panic!("unknown event lost")};
        assert_eq!(value["opaque"]["number"].as_f64().unwrap().to_bits(),1);
        assert_eq!(owner.close().await,Error::Closed);peer.await.unwrap();
        assert_eq!(events.recv().await,Some(Err(Error::Closed)));assert_eq!(events.recv().await,None);
        let (_peer,local)=tokio::io::duplex(16);let (read,write)=tokio::io::split(local);
        let auto=pi_rpc::client::connect_auto(Transport::new(read,write,"1.5.0").unwrap(),Config::default()).unwrap();
        assert!(matches!(auto,pi_rpc::AutoConnection::V1_5 {..}));drop(auto);
    }).await;
}

#[test]
fn extension_only_accepts_pi15_and_uses_its_own_event_type() {
    for version in ["1.3.0", "1.3.10", "1.4.0", "1.4.10"] {
        let transport = Transport::new(tokio::io::empty(), tokio::io::sink(), version).unwrap();
        assert_eq!(
            Owner::connect(transport, Config::default()).err(),
            Some(Error::Version)
        );
    }
    let queue = decoded(queue("consume", "followUp", json!(["A", "B"]))).unwrap();
    assert!(matches!(
        queue.tracking.unwrap().change,
        QueueChange::Consume { .. }
    ));
}

#[test]
fn extension_stack_safety_in_isolated_process() {
    const CHILD: &str = "PI_RPC_BOLT_STACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::thread::Builder::new().stack_size(2*1024*1024).spawn(|| {
            let runtime=tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap();
            runtime.block_on(async {
                for kind in ["future_event","queue_update"] {
                    let wire=format!("{{\"type\":\"{kind}\",\"steering\":[],\"followUp\":[],\"payload\":{}null{}}}\n","[".repeat(511),"]".repeat(511));
                    let (value,_)=pi_rpc::codec::JsonlReader::new(wire.as_bytes(),65536).next().await.unwrap().unwrap();
                    assert_eq!(pi_rpc::codec::encode(&value,65536).unwrap().len(),wire.len());
                    let record=protocol::decode_record::<BoltMessageIdV1>(value).unwrap();
                    drop(record.clone());drop(record);
                }
                let too_deep=format!("{{\"type\":\"queue_update\",\"steering\":[],\"followUp\":[],\"payload\":{}null{}}}\n","[".repeat(512),"]".repeat(512));
                assert!(matches!(pi_rpc::codec::JsonlReader::new(too_deep.as_bytes(),65536).next().await,Err(Error::Protocol(_))));
                let (client,_events,owner,_peer)=connection(Config::default());
                let images=|| {
                    let mut deep=Value::Null;
                    for _ in 0..10000 { deep=Value::Array(vec![deep]); }
                    Some(vec![protocol::ImageContent {r#type:protocol::ImageType::Image,data:"AA==".into(),mime_type:"image/png".into(),extra:[("deep".into(),deep)].into_iter().collect()}])
                };
                // Constructing and dropping an unpolled future must retain the
                // ordinary Owned cleanup; no image extras may unwind recursively.
                drop(client.prompt_with_message_id("input",images(),None,id("A")));
                drop(client.steer_with_message_id("input",images(),id("B")));
                drop(client.follow_up_with_message_id("input",images(),id("C")));
                assert!(matches!(client.prompt_with_message_id("input",images(),None,id("D")).await,Err(Error::Protocol(_))));
                drop(owner);
            });
        }).unwrap().join().unwrap();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "extension_stack_safety_in_isolated_process",
            "--nocapture",
        ])
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
