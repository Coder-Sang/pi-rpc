use super::*;

#[test]
fn retry_delays_accept_fractional_milliseconds_and_preserve_integer_boundaries() {
    for kind in ["auto_retry_start", "summarization_retry_scheduled"] {
        for (number, expected) in [
            ("0", 0),
            ("-0.0", 0),
            ("0.75", 0),
            ("1500.5", 1500),
            ("1.5e3", 1500),
            ("9007199254740993", 9_007_199_254_740_993),
            ("18446744073709551615", u64::MAX),
            ("18446744073709549568.0", u64::MAX - 2047),
        ] {
            let wire = json!({"type":kind,"attempt":1,"maxAttempts":3,"delayMs":serde_json::from_str::<Value>(number).unwrap(),"errorMessage":"synthetic"});
            for event in [
                serde_json::from_value::<KnownEvent>(wire.clone()).unwrap(),
                match decode_record(wire).unwrap() {
                    Record::Event(Event::Known(event)) => event,
                    _ => panic!("expected known retry event"),
                },
            ] {
                let delay = match &event {
                    KnownEvent::AutoRetryStart { delay_ms, .. }
                    | KnownEvent::SummarizationRetryScheduled { delay_ms, .. } => *delay_ms,
                    _ => panic!("expected retry delay"),
                };
                assert_eq!(delay, expected, "{kind}: {number}");
                assert_eq!(serde_json::to_value(event).unwrap()["delayMs"], expected);
            }
        }
    }
}

#[test]
fn retry_delays_remain_required_and_reject_invalid_durations() {
    for kind in ["auto_retry_start", "summarization_retry_scheduled"] {
        let base = json!({"type":kind,"attempt":1,"maxAttempts":3,"errorMessage":"synthetic"});
        for bad in [
            None,
            Some(Value::Null),
            Some(json!(-0.25)),
            Some(json!(-1)),
            Some(json!("1500.5")),
            Some(json!(true)),
            Some(json!([])),
            Some(json!({})),
            Some(serde_json::from_str("18446744073709551616").unwrap()),
        ] {
            let mut wire = base.clone();
            if let Some(bad) = bad {
                wire["delayMs"] = bad;
            }
            assert!(serde_json::from_value::<KnownEvent>(wire.clone()).is_err());
            assert!(matches!(decode_record(wire), Err(Error::Protocol(_))));
        }
    }
}

#[test]
fn required_open_tool_payloads_allow_null_but_reject_absence_and_bad_nested_updates() {
    for (kind, key) in [
        ("tool_execution_start", "args"),
        ("tool_execution_update", "args"),
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
fn known_event_fixtures() -> Vec<Value> {
    let entry = json!({"type":"custom","id":"e","parentId":null,"timestamp":"2026-10-02T00:00:00Z","customType":"x","data":{"arbitrary":true}});
    vec![
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
    ]
}

#[test]
fn all_known_event_shapes_decode_and_required_fields_are_not_unknown() {
    let fixtures = known_event_fixtures();
    // Golden set from the pinned Pi 1.2.0 AgentSessionEvent and RPC mode.
    assert_fixture_names(
        &fixtures,
        "type",
        &[
            "agent_start",
            "agent_end",
            "agent_settled",
            "turn_start",
            "turn_end",
            "message_start",
            "message_update",
            "message_end",
            "tool_execution_start",
            "tool_execution_update",
            "tool_execution_end",
            "queue_update",
            "entry_appended",
            "session_info_changed",
            "thinking_level_changed",
            "compaction_start",
            "compaction_end",
            "auto_retry_start",
            "auto_retry_end",
            "summarization_retry_scheduled",
            "summarization_retry_attempt_start",
            "summarization_retry_finished",
            "bash_execution_update",
            "extension_error",
        ],
    );
    for v in fixtures {
        let kind = v["type"].clone();
        let Record::Event(Event::Known(event)) = decode_record(v).unwrap() else {
            panic!("expected known event");
        };
        assert_eq!(serde_json::to_value(event).unwrap()["type"], kind);
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
fn assistant_update_fixtures() -> Vec<Value> {
    let mut aborted = assistant();
    aborted["stopReason"] = "aborted".into();
    vec![
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
        json!({"type":"error","reason":"aborted","error":aborted}),
    ]
}

#[test]
fn every_delta_only_assistant_update_decodes_without_partial_snapshot() {
    let fixtures = assistant_update_fixtures();
    assert_fixture_names(
        &fixtures,
        "type",
        &[
            "start",
            "text_start",
            "text_delta",
            "text_end",
            "thinking_start",
            "thinking_delta",
            "thinking_end",
            "toolcall_start",
            "toolcall_delta",
            "toolcall_end",
            "done",
            "error",
        ],
    );
    for v in fixtures {
        let wire = json!({"type":"message_update","usage":usage(),"assistantMessageEvent":v});
        let Record::Event(Event::Known(KnownEvent::MessageUpdate {
            assistant_message_event,
            ..
        })) = decode_record(wire).unwrap()
        else {
            panic!("expected assistant update");
        };
        assert_eq!(serde_json::to_value(assistant_message_event).unwrap(), v);
    }
}

#[test]
fn known_event_required_fields_reject_absence_and_explicit_null() {
    for fixture in known_event_fixtures() {
        let kind = fixture["type"].as_str().unwrap();
        let keys: &[&str] = match kind {
            "agent_start"
            | "turn_start"
            | "session_info_changed"
            | "summarization_retry_finished" => &[],
            "agent_end" => &["messages", "willRetry"],
            "agent_settled" => &["aborted"],
            "turn_end" => &["message", "toolResults"],
            "message_start" | "message_end" => &["message"],
            "message_update" => &["usage", "assistantMessageEvent"],
            "tool_execution_start" => &["toolCallId", "toolName", "args"],
            "tool_execution_update" => &["toolCallId", "toolName", "args", "partialResult"],
            "tool_execution_end" => &["toolCallId", "toolName", "result", "isError"],
            "queue_update" => &["steering", "followUp"],
            "entry_appended" => &["entry"],
            "thinking_level_changed" => &["level"],
            "compaction_start" => &["reason"],
            "compaction_end" => &["reason", "aborted", "willRetry"],
            "auto_retry_start" | "summarization_retry_scheduled" => {
                &["attempt", "maxAttempts", "delayMs", "errorMessage"]
            }
            "auto_retry_end" => &["success", "attempt"],
            "summarization_retry_attempt_start" => &["source", "reason"],
            "bash_execution_update" => &["delta"],
            "extension_error" => &["extensionPath", "event", "error"],
            other => panic!("missing negative-field coverage: {other}"),
        };
        for key in keys {
            let mut missing = fixture.clone();
            missing.as_object_mut().unwrap().remove(*key);
            assert!(
                matches!(decode_record(missing), Err(Error::Protocol(_))),
                "{kind}: missing {key}"
            );
            let mut null = fixture.clone();
            null[*key] = Value::Null;
            let result = decode_record(null);
            if matches!(*key, "args" | "partialResult" | "result") {
                assert!(result.is_ok(), "{kind}: opaque {key} must accept null");
            } else {
                assert!(
                    matches!(result, Err(Error::Protocol(_))),
                    "{kind}: null {key}"
                );
            }
        }
    }
}

#[test]
fn assistant_updates_reject_missing_required_properties() {
    for fixture in assistant_update_fixtures() {
        for key in fixture
            .as_object()
            .unwrap()
            .keys()
            .filter(|key| key.as_str() != "type")
        {
            let mut missing = fixture.clone();
            missing.as_object_mut().unwrap().remove(key);
            let wire =
                json!({"type":"message_update","usage":usage(),"assistantMessageEvent":missing});
            assert!(
                matches!(decode_record(wire), Err(Error::Protocol(_))),
                "{}: missing {key}",
                fixture["type"]
            );
        }
    }
}
