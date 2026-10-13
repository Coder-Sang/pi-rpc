//! Strict record dispatch and preservation of unknown event records.
use super::{AssistantEvent, ContentBlock, KnownEvent, Response, UiRequest};
use crate::{Error, Result, json};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    Known(KnownEvent),
    Ui(UiRequest),
    Unknown(Value),
    UncorrelatedResponse(Response),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Record {
    Response(Response),
    Event(Event),
}
/// Strict known-record parsing, with raw preservation only for unknown types.
pub fn decode_record(v: Value) -> Result<Record> {
    if let Err(error) = json::check_value_depth(&v) {
        json::drop_value(v);
        return Err(error);
    }
    let kind = v
        .get("type")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Protocol("record must be an object with a string type".into()))?;
    let invalid = || Error::Protocol(format!("malformed known record: {kind}"));
    if kind == "response" {
        let command = v.get("command").and_then(Value::as_str).unwrap_or("");
        if v.get("success") == Some(&Value::Bool(true)) {
            let has_data = matches!(
                command,
                "prompt"
                    | "steer"
                    | "follow_up"
                    | "clear_queue"
                    | "new_session"
                    | "get_state"
                    | "set_model"
                    | "cycle_model"
                    | "get_available_models"
                    | "cycle_thinking_level"
                    | "get_available_thinking_levels"
                    | "compact"
                    | "bash"
                    | "get_session_stats"
                    | "export_html"
                    | "switch_session"
                    | "fork"
                    | "clone"
                    | "get_fork_messages"
                    | "get_entries"
                    | "get_tree"
                    | "get_last_assistant_text"
                    | "get_messages"
                    | "get_commands"
            );
            if has_data && v.get("data").is_none() {
                return Err(invalid());
            }
            let required = match command {
                "get_entries" | "get_tree" => Some("leafId"),
                _ => None,
            };
            if let Some(key) = required {
                if v.get("data").and_then(|d| d.get(key)).is_none() {
                    return Err(invalid());
                }
            }
        }
        let r: Response = json::decode(&v).map_err(|_| invalid())?;
        r.validate()?;
        return Ok(Record::Response(r));
    }
    if kind == "extension_ui_request" {
        let u: UiRequest = json::decode(&v).map_err(|_| invalid())?;
        return Ok(Record::Event(Event::Ui(u)));
    }
    const KNOWN: &[&str] = &[
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
    ];
    if KNOWN.contains(&kind) {
        let required: &[&str] = match kind {
            "tool_execution_start" => &["args"],
            "tool_execution_update" => &["args", "partialResult"],
            "tool_execution_end" => &["result"],
            _ => &[],
        };
        if required.iter().any(|key| v.get(key).is_none()) {
            return Err(invalid());
        }
        let e: KnownEvent = json::decode(&v).map_err(|_| invalid())?;
        if let KnownEvent::MessageUpdate {
            assistant_message_event,
            ..
        } = &e
        {
            let valid = match assistant_message_event {
                AssistantEvent::ToolcallEnd { tool_call, .. } => {
                    matches!(tool_call, ContentBlock::ToolCall { .. })
                }
                AssistantEvent::Done { reason, message } => {
                    message.role == "assistant"
                        && matches!(reason.as_str(), "stop" | "length" | "toolUse" | "deferred")
                }
                AssistantEvent::Error { reason, error } => {
                    error.role == "assistant" && matches!(reason.as_str(), "aborted" | "error")
                }
                _ => true,
            };
            if !valid {
                return Err(invalid());
            }
        }
        return Ok(Record::Event(Event::Known(e)));
    }
    Ok(Record::Event(Event::Unknown(v)))
}
