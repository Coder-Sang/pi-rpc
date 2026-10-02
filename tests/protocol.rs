use pi_rpc::{Error, protocol::*};
use serde_json::{Value, json};
#[test]
fn required_nullable_rpc_and_native_fields_distinguish_absence_from_null() {
    for (command, data, key) in [
        ("cycle_model", Value::Null, None),
        ("cycle_thinking_level", Value::Null, None),
        (
            "get_last_assistant_text",
            json!({"text":null}),
            Some("text"),
        ),
        (
            "get_entries",
            json!({"entries":[],"leafId":null}),
            Some("leafId"),
        ),
        ("get_tree", json!({"tree":[],"leafId":null}), Some("leafId")),
    ] {
        let mut v = json!({"type":"response","command":command,"success":true,"data":data});
        assert!(decode_record(v.clone()).is_ok());
        if let Some(key) = key {
            v["data"].as_object_mut().unwrap().remove(key);
        } else {
            v.as_object_mut().unwrap().remove("data");
        }
        assert!(matches!(decode_record(v), Err(Error::Protocol(_))));
    }
    let mut entry = json!({"type":"context_edit","id":"e","parentId":null,"timestamp":"iso","targetId":"target","replacement":null});
    assert!(serde_json::from_value::<SessionEntry>(entry.clone()).is_ok());
    entry.as_object_mut().unwrap().remove("parentId");
    assert!(serde_json::from_value::<SessionEntry>(entry.clone()).is_err());
    entry["parentId"] = Value::Null;
    entry.as_object_mut().unwrap().remove("replacement");
    assert!(serde_json::from_value::<SessionEntry>(entry).is_err());
    assert!(serde_json::from_value::<Entries>(json!({"entries":[]})).is_err());
    assert!(serde_json::from_value::<Tree>(json!({"tree":[]})).is_err());
    assert!(serde_json::from_value::<LastAssistantText>(json!({})).is_err());
}
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
fn usage() -> Value {
    json!({"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,"reasoning":1,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}})
}
#[test]
fn all_33_success_response_data_schemas_are_covered() {
    let state = json!({"thinkingLevel":"off","isStreaming":false,"isCompacting":false,"steeringMode":"all","followUpMode":"all","sessionId":"s","autoCompactionEnabled":true,"messageCount":0,"pendingMessageCount":0});
    let fixtures = vec![
        ("prompt", Some(json!({"disposition":"started"}))),
        ("steer", Some(json!({"disposition":"queued"}))),
        ("follow_up", Some(json!({"disposition":"handled"}))),
        ("abort", None),
        ("clear_queue", Some(json!({"steering":[],"followUp":[]}))),
        ("new_session", Some(json!({"cancelled":false}))),
        ("get_state", Some(state)),
        ("set_model", Some(json!({"id":"m","providerSpecific":true}))),
        ("cycle_model", Some(Value::Null)),
        ("get_available_models", Some(json!({"models":[]}))),
        ("set_thinking_level", None),
        ("cycle_thinking_level", Some(Value::Null)),
        (
            "get_available_thinking_levels",
            Some(json!({"levels":["max"]})),
        ),
        ("set_steering_mode", None),
        ("set_follow_up_mode", None),
        (
            "compact",
            Some(json!({"summary":"s","firstKeptEntryId":"e","tokensBefore":10})),
        ),
        ("set_auto_compaction", None),
        ("set_auto_retry", None),
        ("abort_retry", None),
        (
            "bash",
            Some(json!({"output":"x","cancelled":false,"truncated":false})),
        ),
        ("abort_bash", None),
        (
            "get_session_stats",
            Some(
                json!({"sessionId":"s","userMessages":1,"assistantMessages":0,"toolCalls":0,"toolResults":0,"totalMessages":1,"tokens":{"input":1,"output":0,"cacheRead":0,"cacheWrite":0,"total":1},"cost":0}),
            ),
        ),
        ("export_html", Some(json!({"path":"x"}))),
        ("switch_session", Some(json!({"cancelled":true}))),
        ("fork", Some(json!({"text":"x","cancelled":true}))),
        ("clone", Some(json!({"cancelled":true}))),
        ("get_fork_messages", Some(json!({"messages":[]}))),
        ("get_entries", Some(json!({"entries":[],"leafId":null}))),
        ("get_tree", Some(json!({"tree":[],"leafId":null}))),
        ("get_last_assistant_text", Some(json!({"text":null}))),
        ("set_session_name", None),
        ("get_messages", Some(json!({"messages":[]}))),
        ("get_commands", Some(json!({"commands":[]}))),
    ];
    assert_eq!(fixtures.len(), 33);
    for (command, data) in fixtures {
        let mut wire = json!({"type":"response","command":command,"success":true});
        if let Some(data) = data {
            wire["data"] = data;
        }
        assert!(decode_record(wire).is_ok(), "{command}");
    }
}
fn user() -> Value {
    json!({"role":"user","content":"native","timestamp":123})
}
fn assistant() -> Value {
    json!({"role":"assistant","content":[],"api":"provider-native","provider":"future-provider","model":"future-model","usage":usage(),"stopReason":"stop","timestamp":123,"opaque":{"provider":true}})
}
#[test]
fn every_command_round_trips_with_correct_wire_name_and_casing() {
    let fixtures = vec![
        json!({"type":"prompt","message":"x","streamingBehavior":"followUp","images":[{"type":"image","data":"AA==","mimeType":"image/png"}]}),
        json!({"type":"steer","message":"x"}),
        json!({"type":"follow_up","message":"x"}),
        json!({"type":"abort"}),
        json!({"type":"clear_queue"}),
        json!({"type":"new_session","parentSession":"p"}),
        json!({"type":"get_state"}),
        json!({"type":"set_model","provider":"p","modelId":"m"}),
        json!({"type":"cycle_model"}),
        json!({"type":"get_available_models"}),
        json!({"type":"set_thinking_level","level":"max"}),
        json!({"type":"cycle_thinking_level"}),
        json!({"type":"get_available_thinking_levels"}),
        json!({"type":"set_steering_mode","mode":"one-at-a-time"}),
        json!({"type":"set_follow_up_mode","mode":"all"}),
        json!({"type":"compact","customInstructions":"x"}),
        json!({"type":"set_auto_compaction","enabled":true}),
        json!({"type":"set_auto_retry","enabled":false}),
        json!({"type":"abort_retry"}),
        json!({"type":"bash","command":"x","excludeFromContext":true}),
        json!({"type":"abort_bash"}),
        json!({"type":"get_session_stats"}),
        json!({"type":"export_html","outputPath":"p"}),
        json!({"type":"switch_session","sessionPath":"p"}),
        json!({"type":"fork","entryId":"e"}),
        json!({"type":"clone"}),
        json!({"type":"get_fork_messages"}),
        json!({"type":"get_entries","since":"s"}),
        json!({"type":"get_tree"}),
        json!({"type":"get_last_assistant_text"}),
        json!({"type":"set_session_name","name":"n"}),
        json!({"type":"get_messages"}),
        json!({"type":"get_commands"}),
    ];
    assert_eq!(fixtures.len(), 33);
    for v in fixtures {
        let c: Command = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(c.name(), v["type"].as_str().unwrap());
        assert_eq!(serde_json::to_value(c).unwrap(), v);
    }
}
#[test]
fn all_known_event_shapes_decode_and_required_fields_are_not_unknown() {
    let entry = json!({"type":"custom","id":"e","parentId":null,"timestamp":"2026-10-02T00:00:00Z","customType":"x","data":{"arbitrary":true}});
    let fixtures = vec![
        json!({"type":"agent_start"}),
        json!({"type":"agent_end","messages":[user()],"willRetry":false}),
        json!({"type":"agent_settled"}),
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
fn strict_response_shapes_and_cancelled_values() {
    for (cmd, data) in [
        ("prompt", json!({"disposition":"started"})),
        ("steer", json!({"disposition":"handled"})),
        ("follow_up", json!({"disposition":"queued"})),
        ("new_session", json!({"cancelled":true})),
        ("switch_session", json!({"cancelled":true})),
        ("clone", json!({"cancelled":true})),
        ("fork", json!({"cancelled":true,"text":"x"})),
        ("cycle_model", Value::Null),
        ("cycle_thinking_level", Value::Null),
        ("get_messages", json!({"messages":[user()]})),
        ("get_entries", json!({"entries":[],"leafId":null})),
        ("get_tree", json!({"tree":[],"leafId":null})),
    ] {
        assert!(matches!(
            decode_record(
                json!({"type":"response","id":"id","command":cmd,"success":true,"data":data})
            )
            .unwrap(),
            Record::Response(_)
        ));
    }
    for v in [
        json!({"type":"response","command":"prompt","success":true}),
        json!({"type":"response","command":"steer","success":true,"data":{"disposition":"started"}}),
        json!({"type":"response","command":"fork","success":true,"data":{"cancelled":true}}),
        json!({"type":"response","command":"abort","success":false}),
    ] {
        assert!(matches!(decode_record(v), Err(Error::Protocol(_))));
    }
}
#[test]
fn select_choices_and_one_way_ui_responses_are_checked() {
    let u: UiRequest = serde_json::from_value(
        json!({"id":"u","method":"select","title":"t","options":["allow","deny"]}),
    )
    .unwrap();
    assert!(UiAnswer::Value("other".into()).record(&u).is_err());
    assert_eq!(
        UiAnswer::Value("deny".into()).record(&u).unwrap()["id"],
        "u"
    );
    let u: UiRequest =
        serde_json::from_value(json!({"id":"n","method":"notify","message":"x"})).unwrap();
    assert!(UiAnswer::Cancel.record(&u).is_err());
}
