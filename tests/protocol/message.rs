use super::*;

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
