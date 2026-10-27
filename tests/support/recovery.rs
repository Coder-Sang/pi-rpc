//! Synthetic native-v3 records for Pi 1.4 recovery-shaped transport tests.
//! These records exercise preservation, not Pi's recovery/projection algorithm.
use serde_json::{Value, json};

pub(super) fn entries(summary: &str) -> Vec<Value> {
    let timestamp = "2026-10-27T00:00:00.000Z";
    let usage = json!({"input":1.0,"output":2.0,"cacheRead":0.0,"cacheWrite":0.0,
        "totalTokens":3.0,"cost":{"input":0.0,"output":0.0,"cacheRead":0.0,
        "cacheWrite":0.0,"total":0.0}});
    vec![
        json!({"type":"message","id":"user","parentId":null,"timestamp":timestamp,
            "message":{"role":"user","timestamp":1,"content":[
                {"type":"text","text":"synthetic image input"},
                {"type":"image","mimeType":"image/png","data":"aGVsbG8=","opaque":{"image":true}}]}}),
        json!({"type":"message","id":"attempt","parentId":"user","timestamp":timestamp,
            "message":{"role":"assistant","timestamp":2,"api":"openai-completions",
                "provider":"synthetic","model":"synthetic","stopReason":"length","usage":usage,
                "content":[{"type":"toolCall","id":"call","name":"read","arguments":{},
                    "argumentsError":"Truncated synthetic arguments"}]}}),
        json!({"type":"message","id":"result","parentId":"attempt","timestamp":timestamp,
            "message":{"role":"toolResult","timestamp":3,"toolCallId":"call","toolName":"read",
                "isError":true,"content":[{"type":"text","text":"not executed"}],
                "details":{"opaque":true}}}),
        json!({"type":"context_edit","id":"omit-attempt","parentId":"result","timestamp":timestamp,
            "targetId":"attempt","replacement":null}),
        json!({"type":"context_edit","id":"omit-result","parentId":"omit-attempt","timestamp":timestamp,
            "targetId":"result","replacement":null}),
        json!({"type":"compaction","id":"compact","parentId":"omit-result","timestamp":timestamp,
            "summary":summary,"firstKeptEntryId":"user","tokensBefore":100000.0,
            "usage":usage,"details":{"opaque":[true,null,"preserved"]}}),
    ]
}
