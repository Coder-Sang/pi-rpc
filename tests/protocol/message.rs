use super::*;

#[test]
fn native_custom_messages_require_their_fields_and_preserve_details() {
    let base = json!({"role":"custom","customType":"extension.review","content":"中文😀","display":false,"timestamp":123,"details":{"opaque":[null,42.25]}});
    let message: Message = serde_json::from_value(base.clone()).unwrap();
    assert_eq!(serde_json::to_value(message).unwrap(), base);
    for key in ["customType", "content", "display", "timestamp"] {
        for missing in [false, true] {
            let mut invalid = base.clone();
            if missing {
                invalid.as_object_mut().unwrap().remove(key);
            } else {
                invalid[key] = Value::Null;
            }
            assert!(serde_json::from_value::<Message>(invalid.clone()).is_err());
            assert!(matches!(
                decode_record(json!({"type":"message_end","message":invalid})),
                Err(Error::Protocol(_))
            ));
        }
    }
}

#[test]
fn native_message_durations_survive_rpc_and_session_entries_but_reject_bad_types() {
    for base in [assistant(), tool_result()] {
        for duration in [None, Some(json!(0)), Some(json!(42.25))] {
            let mut wire = base.clone();
            if let Some(duration) = duration {
                wire["durationMs"] = duration;
            }
            let message: Message = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(serde_json::to_value(message).unwrap(), wire);
            assert!(decode_record(json!({"type":"response","command":"get_messages","success":true,"data":{"messages":[wire.clone()]}})).is_ok());
            let entry =
                json!({"type":"message","id":"e","parentId":null,"timestamp":"iso","message":wire});
            let decoded: SessionEntry = serde_json::from_value(entry.clone()).unwrap();
            assert_eq!(serde_json::to_value(decoded).unwrap(), entry);
        }
        for bad in [Value::Null, json!("42"), json!(false), json!([]), json!({})] {
            let mut wire = base.clone();
            wire["durationMs"] = bad;
            assert!(serde_json::from_value::<Message>(wire.clone()).is_err());
            assert!(decode_record(json!({"type":"message_end","message":wire.clone()})).is_err());
            assert!(decode_record(json!({"type":"response","command":"get_messages","success":true,"data":{"messages":[wire.clone()]}})).is_err());
            assert!(serde_json::from_value::<SessionEntry>(json!({"type":"message","id":"e","parentId":null,"timestamp":"iso","message":wire})).is_err());
        }
    }
    let extension = json!({"role":"futureRole","durationMs":"opaque"});
    assert!(serde_json::from_value::<Message>(extension).is_ok());
}
#[test]
fn known_native_message_validation_and_extension_fields() {
    let v = assistant();
    let m: Message = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(m).unwrap(), v);
    let custom = json!({"role":"extensionFutureRole","arbitrary":{"payload":true}});
    let m: Message = serde_json::from_value(custom.clone()).unwrap();
    assert_eq!(serde_json::to_value(m).unwrap(), custom);
    for v in [
        json!({"role":"assistant","content":[]}),
        json!({"role":"user","timestamp":1}),
        json!({"role":"toolResult","timestamp":1,"content":[]}),
    ] {
        assert!(serde_json::from_value::<Message>(v).is_err());
    }
}

#[test]
fn pi_1_3_tool_argument_errors_survive_messages_streams_and_history() {
    let wire_usage =
        serde_json::to_value(serde_json::from_value::<Usage>(usage()).unwrap()).unwrap();
    let tool = json!({"type":"toolCall","id":"call","name":"read","arguments":{},
        "argumentsError":"Invalid JSON object: {\"path\":\"\\usepackage\"", "namespace":"tools"});
    let decoded: ContentBlock = serde_json::from_value(tool.clone()).unwrap();
    let ContentBlock::ToolCall {
        arguments, extra, ..
    } = &decoded
    else {
        panic!("expected tool call")
    };
    assert!(arguments.is_empty());
    assert_eq!(extra["argumentsError"], tool["argumentsError"]);
    assert_eq!(serde_json::to_value(decoded).unwrap(), tool);
    let mut message = assistant();
    message["content"] = json!([tool.clone()]);
    let update = json!({"type":"message_update","usage":wire_usage,
        "assistantMessageEvent":{"type":"toolcall_end","contentIndex":0,"toolCall":tool}});
    for event in [
        update,
        json!({"type":"message_end","message":message.clone()}),
    ] {
        let Record::Event(Event::Known(decoded)) = decode_record(event.clone()).unwrap() else {
            panic!("expected event")
        };
        assert_eq!(serde_json::to_value(decoded).unwrap(), event);
    }
    let Record::Response(response) =
        decode_record(json!({"type":"response","command":"get_messages",
        "success":true,"data":{"messages":[message.clone()]}}))
        .unwrap()
    else {
        panic!("expected response")
    };
    assert_eq!(
        serde_json::to_value(response.decode::<Messages>().unwrap().messages.remove(0)).unwrap(),
        message
    );
    for bad in [Value::Null, json!("partial"), json!([])] {
        let mut malformed = tool.clone();
        malformed["arguments"] = bad;
        assert!(serde_json::from_value::<ContentBlock>(malformed.clone()).is_err());
        assert!(
            decode_record(json!({"type":"message_update","usage":wire_usage,
            "assistantMessageEvent":{"type":"toolcall_end","contentIndex":0,"toolCall":malformed}}))
            .is_err()
        );
    }
}

#[tokio::test]
async fn pi_1_3_argument_error_survives_fragmented_crlf_jsonl() {
    let wire_usage =
        serde_json::to_value(serde_json::from_value::<Usage>(usage()).unwrap()).unwrap();
    use tokio::io::AsyncWriteExt;
    let event = json!({"type":"message_update","usage":wire_usage,"assistantMessageEvent":{
        "type":"toolcall_end","contentIndex":1,"toolCall":{"type":"toolCall","id":"call",
        "name":"read","arguments":{},"argumentsError":"Malformed: \\usepackage\n\"quoted\" 中文"}}});
    let mut bytes = pi_rpc::codec::encode(&event, 4096).unwrap();
    bytes.pop();
    bytes.extend_from_slice(b"\r\n");
    let (mut writer, reader) = tokio::io::duplex(32);
    let sender = tokio::spawn(async move {
        for chunk in bytes.chunks(3) {
            writer.write_all(chunk).await.unwrap();
        }
    });
    let mut reader = pi_rpc::codec::JsonlReader::new(reader, 4096);
    let (wire, _) = reader.next().await.unwrap().unwrap();
    let Record::Event(Event::Known(decoded)) = decode_record(wire).unwrap() else {
        panic!("expected event")
    };
    assert_eq!(serde_json::to_value(decoded).unwrap(), event);
    assert!(reader.next().await.unwrap().is_none());
    sender.await.unwrap();
}
