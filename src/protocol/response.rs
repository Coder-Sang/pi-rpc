//! Response envelopes, typed success payloads and validation.
use super::{
    Entries, Extra, Message, PromptDisposition, QueueMode, QueuedInputDisposition, ThinkingLevel,
    Tree, Usage,
};
use crate::{Error, Result, json};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionState {
    pub model: Option<Value>,
    pub thinking_level: ThinkingLevel,
    pub is_streaming: bool,
    pub is_compacting: bool,
    pub steering_mode: QueueMode,
    pub follow_up_mode: QueueMode,
    pub session_file: Option<String>,
    pub session_id: String,
    pub session_name: Option<String>,
    pub auto_compaction_enabled: bool,
    pub message_count: u64,
    pub pending_message_count: u64,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Disposition<T> {
    pub disposition: T,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Queue {
    pub steering: Vec<String>,
    pub follow_up: Vec<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SessionChange {
    pub cancelled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkResult {
    pub text: String,
    pub cancelled: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct BashResult {
    pub output: String,
    pub exit_code: Option<i32>,
    pub cancelled: bool,
    pub truncated: bool,
    pub full_output_path: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompactionResult {
    pub summary: String,
    pub first_kept_entry_id: String,
    pub tokens_before: f64,
    pub estimated_tokens_after: Option<f64>,
    pub usage: Option<Usage>,
    pub details: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionStats {
    pub session_file: Option<String>,
    pub session_id: String,
    pub user_messages: u64,
    pub assistant_messages: u64,
    pub tool_calls: u64,
    pub tool_results: u64,
    pub total_messages: u64,
    pub tokens: TokenTotals,
    pub cost: f64,
    pub context_usage: Option<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TokenTotals {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub path: String,
    pub source: String,
    pub scope: String,
    pub origin: String,
    pub base_dir: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SlashCommand {
    pub name: String,
    pub description: Option<String>,
    pub source: String,
    pub source_info: SourceInfo,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Models {
    pub models: Vec<Value>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ModelCycle {
    pub model: Value,
    pub thinking_level: ThinkingLevel,
    pub is_scoped: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Levels {
    pub levels: Vec<ThinkingLevel>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Level {
    pub level: ThinkingLevel,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportPath {
    pub path: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Messages {
    pub messages: Vec<Message>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Commands {
    pub commands: Vec<SlashCommand>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForkMessage {
    pub entry_id: String,
    pub text: String,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ForkMessages {
    pub messages: Vec<ForkMessage>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LastAssistantText {
    // rpc-mode omits an undefined result in empty sessions despite the nullable wire type.
    #[serde(default)]
    pub text: Option<String>,
}

/// Complete response envelope. `decode` checks every known command's success data.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Response {
    pub id: Option<String>,
    pub command: String,
    pub success: bool,
    #[serde(default)]
    pub data: Option<Value>,
    pub error: Option<String>,
}
impl Response {
    pub fn decode<T: serde::de::DeserializeOwned>(&self) -> Result<T> {
        json::check_value_depth(self.data.as_ref().unwrap_or(&Value::Null))?;
        json::decode(self.data.as_ref().unwrap_or(&Value::Null))
            .map_err(|_| Error::Protocol(format!("invalid {} response data", self.command)))
    }
    pub fn validate(&self) -> Result<()> {
        if let Some(data) = &self.data {
            json::check_value_depth(data)?;
        }
        if !self.success {
            if self.error.is_none() {
                return Err(Error::Protocol("rejection missing error".into()));
            }
            return Ok(());
        }
        macro_rules! check {
            ($t:ty) => {
                self.decode::<$t>().map(|_| ())
            };
        }
        match self.command.as_str() {
            "prompt" => check!(Disposition<PromptDisposition>),
            "steer" | "follow_up" => check!(Disposition<QueuedInputDisposition>),
            "clear_queue" => check!(Queue),
            "new_session" | "switch_session" | "clone" => check!(SessionChange),
            "fork" => check!(ForkResult),
            "get_state" => check!(SessionState),
            "set_model" => {
                if self.data.as_ref().is_some_and(Value::is_object) {
                    Ok(())
                } else {
                    Err(Error::Protocol("invalid model response".into()))
                }
            }
            "cycle_model" => check!(Option<ModelCycle>),
            "get_available_models" => check!(Models),
            "cycle_thinking_level" => check!(Option<Level>),
            "get_available_thinking_levels" => check!(Levels),
            "compact" => check!(CompactionResult),
            "bash" => check!(BashResult),
            "get_session_stats" => check!(SessionStats),
            "export_html" => check!(ExportPath),
            "get_fork_messages" => check!(ForkMessages),
            "get_entries" => check!(Entries),
            "get_tree" => check!(Tree),
            "get_last_assistant_text" => {
                if self.data.as_ref().is_some_and(Value::is_object) {
                    check!(LastAssistantText)
                } else {
                    Err(Error::Protocol(
                        "invalid last assistant text response".into(),
                    ))
                }
            }
            "get_messages" => check!(Messages),
            "get_commands" => check!(Commands),
            "abort"
            | "set_thinking_level"
            | "set_steering_mode"
            | "set_follow_up_mode"
            | "set_auto_compaction"
            | "set_auto_retry"
            | "abort_retry"
            | "abort_bash"
            | "set_session_name" => Ok(()),
            _ => Ok(()),
        }
    }
}
