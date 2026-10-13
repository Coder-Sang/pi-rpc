use super::*;
use std::collections::BTreeSet;

fn envelope(command: &str, data: Value) -> Value {
    json!({"type":"response","command":command,"success":true,"data":data})
}

fn reject_data(command: &str, data: Value) {
    // Check both entry points: envelope checks alone cannot prove that the
    // command-specific Response::validate branch is still installed.
    let response = Response {
        id: None,
        command: command.into(),
        success: true,
        data: Some(data.clone()),
        error: None,
    };
    assert!(
        matches!(response.validate(), Err(Error::Protocol(_))),
        "{command}: {data}"
    );
    assert!(
        matches!(
            decode_record(envelope(command, data)),
            Err(Error::Protocol(_))
        ),
        "{command}"
    );
}

#[test]
fn command_and_response_fixture_names_are_unique_and_match() {
    let commands = super::command::command_fixtures();
    let responses = super::response::success_fixtures();
    let command_names: BTreeSet<_> = commands
        .iter()
        .map(|v| v["type"].as_str().unwrap())
        .collect();
    let response_names: BTreeSet<_> = responses.iter().map(|(name, _)| *name).collect();
    assert_eq!(commands.len(), command_names.len());
    assert_eq!(responses.len(), response_names.len());
    assert_eq!(command_names, response_names);
}

#[test]
fn every_data_response_rejects_absence_and_wrong_outer_types() {
    let fixtures = super::response::success_fixtures();
    let mut checked = 0;
    for (command, data) in fixtures {
        if data.is_none() {
            continue;
        }
        checked += 1;
        assert!(
            matches!(
                decode_record(json!({"type":"response","command":command,"success":true})),
                Err(Error::Protocol(_))
            ),
            "{command}: missing data"
        );
        for bad in [
            json!(true),
            json!(42),
            json!("invalid"),
            json!([]),
            json!([null]),
        ] {
            reject_data(command, bad);
        }
        if matches!(command, "cycle_model" | "cycle_thinking_level") {
            assert!(decode_record(envelope(command, Value::Null)).is_ok());
        } else {
            reject_data(command, Value::Null);
        }
    }
    assert_eq!(checked, 24);
}

fn required_fields(command: &str) -> &'static [&'static str] {
    match command {
        "prompt" | "steer" | "follow_up" => &["disposition"],
        "clear_queue" => &["steering", "followUp"],
        "new_session" | "switch_session" | "clone" => &["cancelled"],
        "fork" => &["text", "cancelled"],
        "get_state" => &[
            "thinkingLevel",
            "isStreaming",
            "isCompacting",
            "steeringMode",
            "followUpMode",
            "sessionId",
            "autoCompactionEnabled",
            "messageCount",
            "pendingMessageCount",
        ],
        "get_available_models" => &["models"],
        "cycle_model" => &["thinkingLevel", "isScoped"],
        "cycle_thinking_level" => &["level"],
        "get_available_thinking_levels" => &["levels"],
        "compact" => &["summary", "firstKeptEntryId", "tokensBefore"],
        "bash" => &["output", "cancelled", "truncated"],
        "get_session_stats" => &[
            "sessionId",
            "userMessages",
            "assistantMessages",
            "toolCalls",
            "toolResults",
            "totalMessages",
            "tokens",
            "cost",
        ],
        "export_html" => &["path"],
        "get_fork_messages" | "get_messages" => &["messages"],
        "get_entries" => &["entries", "leafId"],
        "get_tree" => &["tree", "leafId"],
        "get_commands" => &["commands"],
        // Model objects remain opaque; empty-session text may be absent.
        "set_model" | "get_last_assistant_text" => &[],
        _ => panic!("uncovered data command: {command}"),
    }
}

#[test]
fn typed_responses_reject_missing_required_fields_and_invalid_field_types() {
    for (command, data) in super::response::success_fixtures() {
        let Some(mut data) = data else {
            continue;
        };
        if command == "cycle_model" {
            data = json!({"model":null,"thinkingLevel":"max","isScoped":true});
        } else if command == "cycle_thinking_level" {
            data = json!({"level":"max"});
        }
        assert!(decode_record(envelope(command, data.clone())).is_ok());
        for &field in required_fields(command) {
            let mut missing = data.clone();
            missing.as_object_mut().unwrap().remove(field);
            reject_data(command, missing);
            let mut wrong = data.clone();
            // leafId is required but nullable; booleans remain invalid IDs.
            wrong[field] = if field == "leafId" {
                json!(false)
            } else {
                Value::Null
            };
            reject_data(command, wrong);
        }
    }
}

#[test]
fn response_validation_rejects_malformed_nested_data_and_enum_values() {
    let message = user();
    let entry =
        json!({"type":"message","id":"e","parentId":null,"timestamp":"iso","message":message});
    let cases = [
        ("prompt", json!({"disposition":"unknown"})),
        ("steer", json!({"disposition":"started"})),
        ("follow_up", json!({"disposition":"started"})),
        ("clear_queue", json!({"steering":[42],"followUp":[]})),
        (
            "cycle_model",
            json!({"model":null,"thinkingLevel":"unknown","isScoped":true}),
        ),
        ("cycle_thinking_level", json!({"level":"unknown"})),
        (
            "get_available_thinking_levels",
            json!({"levels":["unknown"]}),
        ),
        (
            "get_fork_messages",
            json!({"messages":[{"entryId":"e","text":42}]}),
        ),
        (
            "get_messages",
            json!({"messages":[{"role":"user","content":"x","timestamp":"bad"}]}),
        ),
        (
            "get_entries",
            json!({"entries":[{"type":"message","id":"e","parentId":null,"timestamp":"iso","message":{"role":"user","content":"x"}}],"leafId":null}),
        ),
        (
            "get_tree",
            json!({"tree":[{"entry":entry,"children":[42]}],"leafId":null}),
        ),
        (
            "get_commands",
            json!({"commands":[{"name":"x","source":"extension","sourceInfo":{"path":42,"source":"x","scope":"x","origin":"x"}}]}),
        ),
        (
            "get_session_stats",
            json!({"sessionId":"s","userMessages":0,"assistantMessages":0,"toolCalls":0,"toolResults":0,"totalMessages":0,"tokens":{"input":"bad","output":0,"cacheRead":0,"cacheWrite":0,"total":0},"cost":0}),
        ),
        (
            "compact",
            json!({"summary":"x","firstKeptEntryId":"e","tokensBefore":1,"usage":{}}),
        ),
        (
            "bash",
            json!({"output":"x","cancelled":false,"truncated":false,"exitCode":"bad"}),
        ),
    ];
    for (command, data) in cases {
        reject_data(command, data);
    }
}

#[test]
fn unconstrained_responses_and_open_model_data_remain_compatible() {
    for (command, data) in super::response::success_fixtures() {
        if data.is_none() {
            // Commands with no success payload retain their permissive data rule.
            for data in [Value::Null, json!(42), json!({"future":true})] {
                assert!(decode_record(envelope(command, data)).is_ok(), "{command}");
            }
        }
    }
    for data in [json!({}), json!({"future":[null,42,{"opaque":true}]})] {
        assert!(decode_record(envelope("set_model", data)).is_ok());
    }
    for data in [json!({}), json!({"text":null}), json!({"text":"answer"})] {
        assert!(decode_record(envelope("get_last_assistant_text", data)).is_ok());
    }
    assert!(decode_record(envelope("future_command", json!({"arbitrary":true}))).is_ok());
    assert!(
        decode_record(json!({"type":"response","command":"future_command","success":true})).is_ok()
    );
}
