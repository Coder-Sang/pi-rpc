use super::*;
use std::{
    pin::Pin,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, ReadBuf};

struct FragmentedReader(ReadHalf<DuplexStream>);
impl AsyncRead for FragmentedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        output: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        if output.remaining() == 0 {
            return Poll::Ready(Ok(()));
        }
        let mut bytes = [0; 3];
        let limit = output.remaining().min(bytes.len());
        let mut fragment = ReadBuf::new(&mut bytes[..limit]);
        match Pin::new(&mut self.0).poll_read(cx, &mut fragment) {
            Poll::Ready(Ok(())) => {
                output.put_slice(fragment.filled());
                Poll::Ready(Ok(()))
            }
            other => other,
        }
    }
}

pub(super) fn setup_fragmented(
    config: Config,
) -> (
    PiClient,
    PiEvents,
    PiConnection,
    PeerReader,
    WriteHalf<DuplexStream>,
) {
    let (local, peer) = tokio::io::duplex(65536);
    let (reader, writer) = tokio::io::split(local);
    let (client, events, owner) = PiConnection::connect(
        Transport::new(FragmentedReader(reader), writer, PI_VERSION).unwrap(),
        config,
    )
    .unwrap();
    let (reader, writer) = tokio::io::split(peer);
    (
        client,
        events,
        owner,
        JsonlReader::new(reader, 65536),
        writer,
    )
}

fn assistant_message(stop: &str, content: Value) -> Value {
    json!({"role":"assistant","content":content,"api":"test","provider":"test","model":"test",
        "usage":{"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,
            "cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}},
        "stopReason":stop,"timestamp":123,"responseId":"opaque-response"})
}

async fn send_records(writer: &mut WriteHalf<DuplexStream>, records: &[Value]) {
    let mut wire = Vec::new();
    for record in records {
        let mut bytes = codec::encode(record, 65536).unwrap();
        bytes.pop();
        wire.extend(bytes);
        wire.extend_from_slice(b"\r\n");
    }
    // A single coalesced write, but the transport must return at most 3 bytes
    // per poll. UTF-8, escapes and CRLF are split regardless of scheduling.
    writer.write_all(&wire).await.unwrap();
}

async fn eof_once(
    events: &mut PiEvents,
    owner: &mut PiConnection,
    writer: &mut WriteHalf<DuplexStream>,
) {
    writer.shutdown().await.unwrap();
    assert_eq!(bounded_event(events).await, Some(Err(Error::Eof)));
    assert_eq!(bounded_event(events).await, None);
    assert_eq!(bounded_event(events).await, None);
    assert_eq!(owner.wait_terminal().await, Error::Eof);
}

#[tokio::test]
async fn fragmented_multiblock_updates_keep_order_final_content_and_tool_metadata() {
    let (_client, mut events, mut owner, _reader, mut writer) = setup_fragmented(Config::default());
    let updates = [
        json!({"type":"thinking_start","contentIndex":0}),
        json!({"type":"thinking_delta","contentIndex":0,"delta":"思考\n😀"}),
        json!({"type":"thinking_end","contentIndex":0,"content":"最终思考"}),
        json!({"type":"text_start","contentIndex":1}),
        json!({"type":"text_delta","contentIndex":1,"delta":"答案\"😀"}),
        json!({"type":"text_end","contentIndex":1,"content":"最终答案"}),
        json!({"type":"toolcall_start","contentIndex":2,"id":"call","toolName":"lookup"}),
        json!({"type":"toolcall_delta","contentIndex":2,"delta":"{\"q\":"}),
        json!({"type":"toolcall_delta","contentIndex":2,"delta":"\"中文\"}"}),
        json!({"type":"toolcall_end","contentIndex":2,"toolCall":{"type":"toolCall","id":"call","name":"lookup","arguments":{"q":"中文"},"thoughtSignature":"opaque","namespace":"tools"}}),
        json!({"type":"text_start","contentIndex":3}),
        json!({"type":"text_delta","contentIndex":3,"delta":"另一个文本块"}),
        json!({"type":"text_end","contentIndex":3,"content":"最终第二块"}),
    ];
    let initial = assistant_message("pending", json!([]));
    let final_message = assistant_message(
        "toolUse",
        json!([
            {"type":"thinking","thinking":"最终思考","thinkingSignature":"opaque","redacted":false},
            {"type":"text","text":"最终答案","textSignature":"{\"v\":1,\"phase\":\"final_answer\"}"},
            {"type":"toolCall","id":"call","name":"lookup","arguments":{"q":"中文"},"thoughtSignature":"opaque","namespace":"tools"},
            {"type":"text","text":"最终第二块"},
        ]),
    );
    let tools = [
        json!({"type":"tool_execution_start","toolCallId":"call","toolName":"lookup","args":{"q":"中文"},"parentToolCallId":"parent","extensionField":true}),
        json!({"type":"tool_execution_update","toolCallId":"call","toolName":"lookup","args":{"q":"中文"},"partialResult":{"content":[{"type":"text","text":"部分结果"}],"details":{"opaque":true}},"parentToolCallId":"parent"}),
        json!({"type":"tool_execution_end","toolCallId":"call","toolName":"lookup","result":{"content":[{"type":"image","mimeType":"image/png","data":"AAAA"}],"details":{"opaque":true}},"isError":false,"durationMs":42.25,"parentToolCallId":"parent","extensionField":true}),
    ];
    let mut records = vec![json!({"type":"message_start","message":initial})];
    for update in &updates {
        records.push(json!({"type":"message_update","usage":initial["usage"],"assistantMessageEvent":update}));
    }
    records.push(json!({"type":"message_end","message":final_message}));
    records.extend(tools.iter().cloned());
    send_records(&mut writer, &records).await;

    let Some(Ok(Event::Known(KnownEvent::MessageStart { message }))) =
        bounded_event(&mut events).await
    else {
        panic!("expected message start");
    };
    assert_eq!(serde_json::to_value(message).unwrap(), initial);
    for update in &updates {
        let Some(Ok(Event::Known(KnownEvent::MessageUpdate {
            usage,
            assistant_message_event,
        }))) = bounded_event(&mut events).await
        else {
            panic!("expected message update");
        };
        assert_eq!(usage.total_tokens, 3.0);
        assert_eq!(
            serde_json::to_value(assistant_message_event).unwrap(),
            *update
        );
    }
    let Some(Ok(Event::Known(KnownEvent::MessageEnd { message }))) =
        bounded_event(&mut events).await
    else {
        panic!("expected final message");
    };
    assert_eq!(serde_json::to_value(message).unwrap(), final_message);
    for tool in &tools {
        let Some(Ok(Event::Known(event))) = bounded_event(&mut events).await else {
            panic!("expected tool event");
        };
        assert_eq!(serde_json::to_value(event).unwrap(), *tool);
    }
    assert!(owner.terminal_reason().is_none());
    eof_once(&mut events, &mut owner, &mut writer).await;
}

#[tokio::test]
async fn custom_messages_entries_and_unknown_events_keep_native_payloads() {
    let (_client, mut events, mut owner, _reader, mut writer) = setup_fragmented(Config::default());
    for display in [false, true] {
        for content in [
            json!("中文\n😀"),
            json!([]),
            json!([
                {"type":"text","text":"中文\"😀","textSignature":"opaque"},
                {"type":"image","data":"AAAA","mimeType":"image/png"},
            ]),
        ] {
            let custom = json!({"role":"custom","customType":"extension.review","content":content,"display":display,"details":{"opaque":[null,42.25]},"timestamp":123,"extensionField":true});
            send_records(
                &mut writer,
                &[
                    json!({"type":"message_start","message":custom}),
                    json!({"type":"message_end","message":custom}),
                ],
            )
            .await;
            for start in [true, false] {
                let message = match bounded_event(&mut events).await.unwrap().unwrap() {
                    Event::Known(KnownEvent::MessageStart { message }) if start => message,
                    Event::Known(KnownEvent::MessageEnd { message }) if !start => message,
                    other => panic!("unexpected {other:?}"),
                };
                assert_eq!(serde_json::to_value(message).unwrap(), custom);
            }
        }
    }
    for entry in [
        json!({"type":"custom","id":"custom","parentId":null,"timestamp":"iso","customType":"review","data":{"opaque":true}}),
        json!({"type":"custom_message","id":"message","parentId":"custom","timestamp":"iso","customType":"review","content":"中文","display":false,"details":{"opaque":true}}),
    ] {
        send_records(
            &mut writer,
            &[json!({"type":"entry_appended","entry":entry})],
        )
        .await;
        let Some(Ok(Event::Known(KnownEvent::EntryAppended { entry: decoded }))) =
            bounded_event(&mut events).await
        else {
            panic!("expected appended entry");
        };
        assert_eq!(serde_json::to_value(decoded).unwrap(), entry);
    }
    let future = json!({"role":"extensionFutureRole","opaque":[null,"中文😀"]});
    let unknown = json!({"type":"extension_future_event","opaque":{"payload":[null,"中文😀"]}});
    send_records(
        &mut writer,
        &[
            json!({"type":"message_end","message":future}),
            unknown.clone(),
        ],
    )
    .await;
    let Some(Ok(Event::Known(KnownEvent::MessageEnd { message }))) =
        bounded_event(&mut events).await
    else {
        panic!("expected extension message");
    };
    assert_eq!(serde_json::to_value(message).unwrap(), future);
    assert_eq!(
        bounded_event(&mut events).await,
        Some(Ok(Event::Unknown(unknown)))
    );
    eof_once(&mut events, &mut owner, &mut writer).await;
}

#[tokio::test]
async fn model_error_abort_and_deferred_messages_are_not_connection_terminal_errors() {
    for stop in ["error", "aborted", "deferred"] {
        let (client, mut events, mut owner, mut reader, mut writer) =
            setup_fragmented(Config::default());
        let command = tokio::spawn(async move { client.abort().await });
        let request = next(&mut reader).await;
        let mut message = assistant_message(stop, json!([]));
        message["errorMessage"] = "synthetic model outcome".into();
        message["diagnostics"] = json!([{"opaque":true}]);
        let records = [
            // A provider may fail before its first delta; Pi emits both native
            // message boundaries rather than a nested AssistantEvent::Error.
            json!({"type":"message_start","message":message}),
            json!({"type":"message_end","message":message}),
            json!({"type":"turn_end","message":message,"toolResults":[]}),
            json!({"type":"agent_end","messages":[message],"willRetry":stop == "error"}),
            json!({"type":"extension_error","extensionPath":"test","event":"test","error":"synthetic extension failure"}),
            json!({"type":"agent_settled","aborted":stop == "aborted"}),
        ];
        let mut batch = records.to_vec();
        batch.push(json!({"type":"response","id":request["id"],"command":"abort","success":true}));
        batch.push(json!({"type":"agent_start"}));
        send_records(&mut writer, &batch).await;
        for record in &records {
            let Some(Ok(Event::Known(event))) = bounded_event(&mut events).await else {
                panic!("model outcome must remain an event");
            };
            match &event {
                KnownEvent::MessageStart { message: native }
                | KnownEvent::MessageEnd { message: native } => {
                    assert_eq!(serde_json::to_value(native).unwrap(), message)
                }
                KnownEvent::AgentEnd { will_retry, .. } => assert_eq!(*will_retry, stop == "error"),
                KnownEvent::AgentSettled { aborted } => assert_eq!(*aborted, stop == "aborted"),
                _ => {}
            }
            assert_eq!(serde_json::to_value(event).unwrap()["type"], record["type"]);
        }
        tokio::time::timeout(Duration::from_secs(2), command)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        assert_eq!(
            bounded_event(&mut events).await,
            Some(Ok(Event::Known(KnownEvent::AgentStart)))
        );
        assert!(owner.terminal_reason().is_none());
        eof_once(&mut events, &mut owner, &mut writer).await;
    }
}
