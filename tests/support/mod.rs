#![allow(dead_code)]
/// Generate a deep tree without recursively building or serializing Values.
pub fn tree_response(depth: usize, id: &str) -> String {
    let mut wire = format!(
        r#"{{"type":"response","id":{id},"command":"get_tree","success":true,"data":{{"leafId":"{}","tree":["#,
        depth - 1
    );
    for i in 0..depth {
        let parent = if i == 0 {
            "null".into()
        } else {
            format!("\"{}\"", i - 1)
        };
        wire.push_str(&format!(r#"{{"entry":{{"type":"session_info","id":"{i}","parentId":{parent},"timestamp":"2026-10-11T00:00:00Z","name":"test"}},"children":["#));
    }
    wire.push_str(&"]}".repeat(depth));
    wire.push_str("]}}\n");
    wire
}

pub fn nested(depth: usize) -> String {
    format!("{}null{}", "[".repeat(depth), "]".repeat(depth))
}

pub fn nested_objects(depth: usize) -> String {
    format!("{}null{}", "{\"child\":".repeat(depth), "}".repeat(depth))
}
