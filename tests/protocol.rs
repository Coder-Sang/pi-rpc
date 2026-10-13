use pi_rpc::{Error, protocol::*};
use serde_json::{Value, json};
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

#[path = "protocol/command.rs"]
mod command;
#[path = "protocol/event.rs"]
mod event;
#[path = "protocol/message.rs"]
mod message;
#[path = "protocol/response.rs"]
mod response;
#[path = "protocol/ui.rs"]
mod ui;

#[path = "protocol/response_negative.rs"]
mod response_negative;
