use super::*;

#[test]
fn required_open_tool_payloads_allow_null_but_reject_absence_and_bad_nested_updates() {
    for (kind, key) in [
        ("tool_execution_start", "args"),
        ("tool_execution_update", "partialResult"),
        ("tool_execution_end", "result"),
    ] {
        let mut v = json!({"type":kind,"toolCallId":"t","toolName":"n","args":null,"partialResult":null,"result":null,"isError":false});
        assert!(decode_record(v.clone()).is_ok());
        v.as_object_mut().unwrap().remove(key);
        assert!(matches!(decode_record(v), Err(Error::Protocol(_))));
    }
    for nested in [
        json!({"type":"text_delta","delta":"x"}),
        json!({"type":"toolcall_start","contentIndex":0,"id":"i"}),
        json!({"type":"toolcall_end","contentIndex":0,"toolCall":null}),
        json!({"type":"toolcall_end","contentIndex":0,"toolCall":{"type":"text","text":"not a tool"}}),
        json!({"type":"done","reason":"invalid","message":assistant()}),
    ] {
        assert!(matches!(
            decode_record(
                json!({"type":"message_update","usage":usage(),"assistantMessageEvent":nested})
            ),
            Err(Error::Protocol(_))
        ));
    }
}
#[test]
fn settled_requires_boolean_aborted() {
    for aborted in [false, true] {
        let wire = json!({"type":"agent_settled","aborted":aborted});
        assert_eq!(
            decode_record(wire.clone()).unwrap(),
            Record::Event(Event::Known(KnownEvent::AgentSettled { aborted }))
        );
        assert_eq!(
            serde_json::to_value(KnownEvent::AgentSettled { aborted }).unwrap(),
            wire
        );
    }
    for wire in [
        json!({"type":"agent_settled"}),
        json!({"type":"agent_settled","aborted":null}),
        json!({"type":"agent_settled","aborted":"false"}),
        json!({"type":"agent_settled","aborted":0}),
    ] {
        assert!(matches!(decode_record(wire), Err(Error::Protocol(_))));
    }
}
#[test]
fn tool_duration_is_optional_numeric_and_keeps_open_payloads() {
    let base = json!({"type":"tool_execution_end","toolCallId":"t","toolName":"open","result":{"durationMs":"opaque"},"isError":false,"parentToolCallId":"p","extensionField":{"arbitrary":true}});
    for duration in [None, Some(json!(0)), Some(json!(12.5))] {
        let mut wire = base.clone();
        if let Some(duration) = duration {
            wire["durationMs"] = duration;
        }
        let Record::Event(Event::Known(event)) = decode_record(wire.clone()).unwrap() else {
            panic!("expected known event")
        };
        assert!(
            matches!(&event, KnownEvent::ToolExecutionEnd { duration_ms, extra, .. } if *duration_ms == wire.get("durationMs").and_then(Value::as_f64) && extra["extensionField"] == wire["extensionField"] && !extra.contains_key("durationMs"))
        );
        let encoded = serde_json::to_value(event).unwrap();
        if let Some(duration) = wire.get("durationMs").and_then(Value::as_f64) {
            wire["durationMs"] = json!(duration);
        }
        assert_eq!(encoded, wire);
    }
    for bad in [Value::Null, json!("12"), json!(true), json!([]), json!({})] {
        let mut wire = base.clone();
        wire["durationMs"] = bad;
        assert!(serde_json::from_value::<KnownEvent>(wire.clone()).is_err());
        assert!(matches!(decode_record(wire), Err(Error::Protocol(_))));
    }
}
#[test]
fn all_known_event_shapes_decode_and_required_fields_are_not_unknown() {
    let entry = json!({"type":"custom","id":"e","parentId":null,"timestamp":"2026-10-02T00:00:00Z","customType":"x","data":{"arbitrary":true}});
    let fixtures = vec![
        json!({"type":"agent_start"}),
        json!({"type":"agent_end","messages":[user()],"willRetry":false}),
        json!({"type":"agent_settled","aborted":false}),
        json!({"type":"turn_start"}),
        json!({"type":"turn_end","message":assistant(),"toolResults":[]}),
        json!({"type":"message_start","message":user()}),
        json!({"type":"message_update","usage":usage(),"assistantMessageEvent":{"type":"text_delta","contentIndex":0,"delta":"a"}}),
        json!({"type":"message_end","message":assistant()}),
        json!({"type":"tool_execution_start","toolCallId":"t","toolName":"open","args":{"a":1},"parentToolCallId":"parent","extensionField":true}),
        json!({"type":"tool_execution_update","toolCallId":"t","toolName":"open","args":{},"partialResult":{"opaque":true}}),
        json!({"type":"tool_execution_end","toolCallId":"t","toolName":"open","result":{},"isError":false}),
        json!({"type":"queue_update","steering":[],"followUp":["x"]}),
        json!({"type":"entry_appended","entry":entry}),
        json!({"type":"session_info_changed"}),
        json!({"type":"thinking_level_changed","level":"max"}),
        json!({"type":"compaction_start","reason":"overflow"}),
        json!({"type":"compaction_end","reason":"overflow","aborted":true,"willRetry":false}),
        json!({"type":"auto_retry_start","attempt":1,"maxAttempts":3,"delayMs":50,"errorMessage":"x"}),
        json!({"type":"auto_retry_end","success":false,"attempt":3,"finalError":"x"}),
        json!({"type":"summarization_retry_scheduled","attempt":1,"maxAttempts":3,"delayMs":50,"errorMessage":"x"}),
        json!({"type":"summarization_retry_attempt_start","source":"compaction","reason":"threshold"}),
        json!({"type":"summarization_retry_finished"}),
        json!({"type":"bash_execution_update","id":"cmd","delta":"output"}),
        json!({"type":"extension_error","extensionPath":"path","event":"tool_call","error":"x"}),
    ];
    // There are 24 wire event kinds, counting all three tool lifecycle kinds.
    assert_eq!(fixtures.len(), 24);
    for v in fixtures {
        assert!(matches!(
            decode_record(v).unwrap(),
            Record::Event(Event::Known(_))
        ));
    }
    assert!(
        decode_record(json!({"type":"summarization_retry_attempt_start","source":"compaction"}))
            .is_err()
    );
    assert!(decode_record(json!({"type":"tool_execution_end","toolCallId":"x"})).is_err());
    let raw = json!({"type":"new_future_kind","opaque":[1,2,3]});
    assert_eq!(
        decode_record(raw.clone()).unwrap(),
        Record::Event(Event::Unknown(raw))
    );
}
#[test]
fn every_delta_only_assistant_update_decodes_without_partial_snapshot() {
    let fixtures = vec![
        json!({"type":"start"}),
        json!({"type":"text_start","contentIndex":0}),
        json!({"type":"text_delta","contentIndex":0,"delta":"x"}),
        json!({"type":"text_end","contentIndex":0,"content":"x"}),
        json!({"type":"thinking_start","contentIndex":1}),
        json!({"type":"thinking_delta","contentIndex":1,"delta":"x"}),
        json!({"type":"thinking_end","contentIndex":1,"content":"x"}),
        json!({"type":"toolcall_start","contentIndex":2,"id":"call","toolName":"futureTool"}),
        json!({"type":"toolcall_delta","contentIndex":2,"delta":"{}"}),
        json!({"type":"toolcall_end","contentIndex":2,"toolCall":{"type":"toolCall","id":"call","name":"futureTool","arguments":{"anything":true}}}),
        json!({"type":"done","reason":"stop","message":assistant()}),
        json!({"type":"error","reason":"aborted","error":assistant()}),
    ];
    assert_eq!(fixtures.len(), 12);
    for v in fixtures {
        let wire = json!({"type":"message_update","usage":usage(),"assistantMessageEvent":v});
        assert!(matches!(
            decode_record(wire).unwrap(),
            Record::Event(Event::Known(KnownEvent::MessageUpdate { .. }))
        ));
    }
}
