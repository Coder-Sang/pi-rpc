use super::*;

#[test]
fn pi_1_2_model_schema_metadata_remains_open_and_lossless() {
    // Pi 1.2 extracts these definitions to model-schema.ts/compat-schema.ts.
    // The RPC still returns open model objects, including key-scoped limits.
    let model = json!({
        "id":"synthetic", "name":"Synthetic", "api":"openai-completions",
        "provider":"openrouter", "baseUrl":"https://example.invalid/v1",
        "input":["text","image"], "contextWindow":8192, "maxTokens":1024,
        "inputLimits":{"maxRequestBytes":33554432,"images":{"resize":{"maxWidth":2000,"maxHeight":2000,"maxBytes":4718592,"jpegQuality":0.8},"maxPerMessage":4,"maxPerRequest":8}},
        "cost":{"input":0.1,"output":0.2,"cacheRead":0.0,"cacheWrite":0.0,"tiers":[{"inputTokensAbove":4096,"input":0.3,"output":0.4,"cacheRead":0.0,"cacheWrite":0.0}]},
        "thinkingLevelMap":{"off":null,"max":"high"},
        "samplingParamsByThinkingLevel":{"high":{"temperature":0.75}},
        "compat":{"openRouterRouting":{"only":["synthetic-region"],"require_parameters":true}}
    });
    for (command, data) in [
        ("set_model", model.clone()),
        ("get_available_models", json!({"models":[model.clone()]})),
        (
            "cycle_model",
            json!({"model":model.clone(),"thinkingLevel":"max","isScoped":true}),
        ),
    ] {
        let Record::Response(response) =
            decode_record(json!({"type":"response","command":command,"success":true,"data":data}))
                .unwrap()
        else {
            panic!("expected model response");
        };
        assert_eq!(response.decode::<Value>().unwrap(), data);
        let decoded = match command {
            "set_model" => response.decode::<Value>().unwrap(),
            "get_available_models" => response.decode::<Models>().unwrap().models.remove(0),
            "cycle_model" => {
                response
                    .decode::<Option<ModelCycle>>()
                    .unwrap()
                    .unwrap()
                    .model
            }
            _ => unreachable!(),
        };
        assert_eq!(decoded, model);
    }
}

#[test]
fn required_nullable_rpc_and_native_fields_distinguish_absence_from_null() {
    for (command, data, key) in [
        ("cycle_model", Value::Null, None),
        ("cycle_thinking_level", Value::Null, None),
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
    assert_eq!(
        serde_json::from_value::<LastAssistantText>(json!({}))
            .unwrap()
            .text,
        None
    );
}
#[test]
fn last_assistant_text_accepts_empty_session_omission_but_rejects_bad_types() {
    for data in [json!({}), json!({"text":null}), json!({"text":"answer"})] {
        let record = decode_record(json!({"type":"response","command":"get_last_assistant_text","success":true,"data":data})).unwrap();
        let Record::Response(response) = record else {
            panic!("expected response")
        };
        assert_eq!(
            response.decode::<LastAssistantText>().unwrap().text,
            data.get("text").and_then(Value::as_str).map(str::to_owned)
        );
    }
    for data in [
        Value::Null,
        json!([]),
        json!([null]),
        json!(42),
        json!("answer"),
        json!({"text":42}),
        json!({"text":false}),
        json!({"text":[]}),
        json!({"text":{}}),
    ] {
        assert!(decode_record(json!({"type":"response","command":"get_last_assistant_text","success":true,"data":data})).is_err());
    }
    assert!(
        decode_record(
            json!({"type":"response","command":"get_last_assistant_text","success":true})
        )
        .is_err()
    );
}
pub(super) fn success_fixtures() -> Vec<(&'static str, Option<Value>)> {
    let state = json!({"thinkingLevel":"off","isStreaming":false,"isCompacting":false,"steeringMode":"all","followUpMode":"all","sessionId":"s","autoCompactionEnabled":true,"messageCount":0,"pendingMessageCount":0});
    vec![
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
    ]
}

#[test]
fn all_33_success_response_data_schemas_are_covered() {
    let fixtures = success_fixtures();
    assert_eq!(fixtures.len(), 33);
    for (command, data) in fixtures {
        let mut wire = json!({"type":"response","command":command,"success":true});
        if let Some(data) = data {
            wire["data"] = data;
        }
        assert!(decode_record(wire).is_ok(), "{command}");
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
