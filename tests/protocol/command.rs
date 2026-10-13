use super::*;

pub(super) fn command_fixtures() -> Vec<Value> {
    vec![
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
    ]
}

#[test]
fn every_command_round_trips_with_correct_wire_name_and_casing() {
    let fixtures = command_fixtures();
    assert_eq!(fixtures.len(), 33);
    for v in fixtures {
        let c: Command = serde_json::from_value(v.clone()).unwrap();
        assert_eq!(c.name(), v["type"].as_str().unwrap());
        assert_eq!(serde_json::to_value(c).unwrap(), v);
    }
}
