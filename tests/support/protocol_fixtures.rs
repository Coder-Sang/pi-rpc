fn assert_fixture_names(fixtures: &[Value], key: &str, expected: &[&str]) {
    let names: std::collections::BTreeSet<_> = fixtures
        .iter()
        .map(|fixture| fixture[key].as_str().unwrap())
        .collect();
    assert_eq!(names.len(), fixtures.len(), "duplicate {key} fixture");
    assert_eq!(names, expected.iter().copied().collect());
}
fn usage() -> Value {
    json!({"input":1,"output":2,"cacheRead":0,"cacheWrite":0,"totalTokens":3,"reasoning":1,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}})
}
fn user() -> Value {
    json!({"role":"user","content":"native","timestamp":123})
}
fn assistant() -> Value {
    json!({"role":"assistant","content":[],"api":"provider-native","provider":"future-provider","model":"future-model","usage":usage(),"stopReason":"stop","timestamp":123,"opaque":{"provider":true}})
}
fn tool_result() -> Value {
    json!({"role":"toolResult","content":[],"toolCallId":"t","toolName":"open","isError":false,"timestamp":123,"details":{"opaque":true}})
}
