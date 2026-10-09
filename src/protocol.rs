//! Wire types from the pinned Pi sources listed in docs/compatibility.md.
//! Tool, provider and extension payloads remain open JSON. Unknown events retain
//! their complete record. Known event decoding never falls back to Unknown.
use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
pub type Extra = Map<String, Value>;
// Optional TypeScript numbers may be absent, but explicit null is not a number.
fn optional_number<'de, D>(d: D) -> std::result::Result<Option<f64>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    f64::deserialize(d).map(Some)
}
// deserialize_with (without default) makes absence invalid while allowing null.
fn required_nullable<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Option::<T>::deserialize(d)
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ThinkingLevel {
    Off,
    Minimal,
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum QueueMode {
    #[serde(rename = "all")]
    All,
    #[serde(rename = "one-at-a-time")]
    OneAtATime,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum StreamingBehavior {
    #[serde(rename = "steer")]
    Steer,
    #[serde(rename = "followUp")]
    FollowUp,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum PromptDisposition {
    Started,
    Queued,
    Handled,
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum QueuedInputDisposition {
    Queued,
    Handled,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum ContentBlock {
    #[serde(rename = "text", rename_all = "camelCase")]
    Text {
        text: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        text_signature: Option<String>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename = "thinking", rename_all = "camelCase")]
    Thinking {
        thinking: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        thinking_signature: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        redacted: Option<bool>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename = "image", rename_all = "camelCase")]
    Image {
        data: String,
        mime_type: String,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename = "toolCall", rename_all = "camelCase")]
    ToolCall {
        id: String,
        name: String,
        arguments: Extra,
        #[serde(flatten)]
        extra: Extra,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MessageContent {
    Text(String),
    Blocks(Vec<ContentBlock>),
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ImageContent {
    pub r#type: ImageType,
    pub data: String,
    pub mime_type: String,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ImageType {
    #[serde(rename = "image")]
    Image,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Cost {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total: f64,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Usage {
    pub input: f64,
    pub output: f64,
    pub cache_read: f64,
    pub cache_write: f64,
    pub total_tokens: f64,
    pub cost: Cost,
    #[serde(flatten)]
    pub extra: Extra,
}

/// Native extensible AgentMessage. Known roles are validated when decoding.
/// Additional roles are permitted by Pi's CustomAgentMessages extension point.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Message {
    pub role: String,
    #[serde(flatten)]
    pub fields: Extra,
}
impl<'de> Deserialize<'de> for Message {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        let mut fields = Extra::deserialize(d)?;
        let role = fields
            .remove("role")
            .and_then(|v| v.as_str().map(str::to_owned))
            .ok_or_else(|| serde::de::Error::custom("message role missing"))?;
        validate_message(&role, &fields).map_err(serde::de::Error::custom)?;
        Ok(Self { role, fields })
    }
}
fn validate_message(role: &str, f: &Extra) -> std::result::Result<(), String> {
    fn field<T: serde::de::DeserializeOwned>(
        f: &Extra,
        key: &str,
    ) -> std::result::Result<(), String> {
        serde_json::from_value::<T>(
            f.get(key)
                .cloned()
                .ok_or_else(|| format!("missing {key}"))?,
        )
        .map(|_| ())
        .map_err(|_| format!("invalid {key}"))
    }
    if matches!(
        role,
        "system"
            | "user"
            | "assistant"
            | "toolResult"
            | "bashExecution"
            | "custom"
            | "branchSummary"
            | "compactionSummary"
    ) {
        field::<f64>(f, "timestamp")?;
    }
    match role {
        "system" | "user" => {
            field::<MessageContent>(f, "content")?;
            validate_content(f, "content", role == "user", false, false)?;
            if role == "system" {
                if f.contains_key("sections") {
                    field::<std::collections::HashMap<String, Option<String>>>(f, "sections")?;
                }
                for key in ["toolsAdded", "toolsRemoved"] {
                    if f.contains_key(key) {
                        field::<Vec<Extra>>(f, key)?;
                    }
                }
            }
        }
        "assistant" => {
            field::<Vec<ContentBlock>>(f, "content")?;
            for k in ["api", "provider", "model", "stopReason"] {
                field::<String>(f, k)?;
            }
            field::<Usage>(f, "usage")?;
            validate_content(f, "content", false, true, true)?;
            if !matches!(
                f["stopReason"].as_str(),
                Some("pending" | "stop" | "length" | "toolUse" | "error" | "aborted" | "deferred")
            ) {
                return Err("invalid stopReason".into());
            }
        }
        "toolResult" => {
            field::<Vec<ContentBlock>>(f, "content")?;
            field::<String>(f, "toolCallId")?;
            field::<String>(f, "toolName")?;
            field::<bool>(f, "isError")?;
            validate_content(f, "content", true, false, false)?;
        }
        "bashExecution" => {
            field::<String>(f, "command")?;
            field::<String>(f, "output")?;
            field::<bool>(f, "cancelled")?;
            field::<bool>(f, "truncated")?;
            if f.contains_key("exitCode") {
                field::<Option<i32>>(f, "exitCode")?;
            }
        }
        "custom" => {
            field::<String>(f, "customType")?;
            field::<MessageContent>(f, "content")?;
            field::<bool>(f, "display")?;
            validate_content(f, "content", true, false, false)?;
        }
        "branchSummary" => {
            field::<String>(f, "summary")?;
            field::<Option<String>>(f, "fromId")?;
        }
        "compactionSummary" => {
            field::<String>(f, "summary")?;
            field::<f64>(f, "tokensBefore")?;
        }
        _ => {}
    }
    if matches!(role, "assistant" | "toolResult") && f.contains_key("durationMs") {
        field::<f64>(f, "durationMs")?;
    }
    Ok(())
}
fn validate_content(
    f: &Extra,
    key: &str,
    image: bool,
    thinking: bool,
    tool: bool,
) -> std::result::Result<(), String> {
    if let Some(blocks) = f.get(key).and_then(Value::as_array) {
        for block in blocks {
            let allowed = match block.get("type").and_then(Value::as_str) {
                Some("text") => true,
                Some("image") => image,
                Some("thinking") => thinking,
                Some("toolCall") => tool,
                _ => false,
            };
            if !allowed {
                return Err("content block not allowed for native role".into());
            }
        }
    }
    Ok(())
}

/// IDs are assigned by PiClient, never by callers of Command.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    #[serde(rename_all = "camelCase")]
    Prompt {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        streaming_behavior: Option<StreamingBehavior>,
    },
    Steer {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    FollowUp {
        message: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        images: Option<Vec<ImageContent>>,
    },
    Abort,
    ClearQueue,
    #[serde(rename_all = "camelCase")]
    NewSession {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        parent_session: Option<String>,
    },
    GetState,
    #[serde(rename_all = "camelCase")]
    SetModel {
        provider: String,
        model_id: String,
    },
    CycleModel,
    GetAvailableModels,
    SetThinkingLevel {
        level: ThinkingLevel,
    },
    CycleThinkingLevel,
    GetAvailableThinkingLevels,
    SetSteeringMode {
        mode: QueueMode,
    },
    SetFollowUpMode {
        mode: QueueMode,
    },
    #[serde(rename_all = "camelCase")]
    Compact {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        custom_instructions: Option<String>,
    },
    SetAutoCompaction {
        enabled: bool,
    },
    SetAutoRetry {
        enabled: bool,
    },
    AbortRetry,
    #[serde(rename_all = "camelCase")]
    Bash {
        command: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        exclude_from_context: Option<bool>,
    },
    AbortBash,
    GetSessionStats,
    #[serde(rename_all = "camelCase")]
    ExportHtml {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        output_path: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SwitchSession {
        session_path: String,
    },
    #[serde(rename_all = "camelCase")]
    Fork {
        entry_id: String,
    },
    Clone,
    GetForkMessages,
    GetEntries {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        since: Option<String>,
    },
    GetTree,
    GetLastAssistantText,
    SetSessionName {
        name: String,
    },
    GetMessages,
    GetCommands,
}
impl Command {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Prompt { .. } => "prompt",
            Self::Steer { .. } => "steer",
            Self::FollowUp { .. } => "follow_up",
            Self::Abort => "abort",
            Self::ClearQueue => "clear_queue",
            Self::NewSession { .. } => "new_session",
            Self::GetState => "get_state",
            Self::SetModel { .. } => "set_model",
            Self::CycleModel => "cycle_model",
            Self::GetAvailableModels => "get_available_models",
            Self::SetThinkingLevel { .. } => "set_thinking_level",
            Self::CycleThinkingLevel => "cycle_thinking_level",
            Self::GetAvailableThinkingLevels => "get_available_thinking_levels",
            Self::SetSteeringMode { .. } => "set_steering_mode",
            Self::SetFollowUpMode { .. } => "set_follow_up_mode",
            Self::Compact { .. } => "compact",
            Self::SetAutoCompaction { .. } => "set_auto_compaction",
            Self::SetAutoRetry { .. } => "set_auto_retry",
            Self::AbortRetry => "abort_retry",
            Self::Bash { .. } => "bash",
            Self::AbortBash => "abort_bash",
            Self::GetSessionStats => "get_session_stats",
            Self::ExportHtml { .. } => "export_html",
            Self::SwitchSession { .. } => "switch_session",
            Self::Fork { .. } => "fork",
            Self::Clone => "clone",
            Self::GetForkMessages => "get_fork_messages",
            Self::GetEntries { .. } => "get_entries",
            Self::GetTree => "get_tree",
            Self::GetLastAssistantText => "get_last_assistant_text",
            Self::SetSessionName { .. } => "set_session_name",
            Self::GetMessages => "get_messages",
            Self::GetCommands => "get_commands",
        }
    }
}

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
        serde_json::from_value(self.data.clone().unwrap_or(Value::Null))
            .map_err(|_| Error::Protocol(format!("invalid {} response data", self.command)))
    }
    pub fn validate(&self) -> Result<()> {
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Entries {
    pub entries: Vec<SessionEntry>,
    #[serde(deserialize_with = "required_nullable")]
    pub leaf_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Tree {
    pub tree: Vec<SessionTreeNode>,
    #[serde(deserialize_with = "required_nullable")]
    pub leaf_id: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionTreeNode {
    pub entry: SessionEntry,
    pub children: Vec<SessionTreeNode>,
    pub label: Option<String>,
    pub label_timestamp: Option<String>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionEntry {
    pub id: String,
    #[serde(deserialize_with = "required_nullable")]
    pub parent_id: Option<String>,
    pub timestamp: String,
    #[serde(flatten)]
    pub data: EntryData,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryData {
    Message {
        message: Message,
    },
    #[serde(rename_all = "camelCase")]
    ThinkingLevelChange {
        thinking_level: String,
    },
    #[serde(rename_all = "camelCase")]
    ModelChange {
        provider: String,
        model_id: String,
    },
    Usage {
        kind: String,
        provider: String,
        model: String,
        usage: Usage,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    Compaction {
        summary: String,
        first_kept_entry_id: String,
        tokens_before: f64,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    BranchSummary {
        from_id: String,
        summary: String,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    Custom {
        custom_type: String,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    CustomMessage {
        custom_type: String,
        content: MessageContent,
        display: bool,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    ContextEdit {
        target_id: String,
        #[serde(deserialize_with = "required_nullable")]
        replacement: Option<ContextReplacement>,
    },
    #[serde(rename_all = "camelCase")]
    Label {
        target_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<String>,
    },
    SessionInfo {
        #[serde(default, skip_serializing_if = "Option::is_none")]
        name: Option<String>,
    },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextReplacement {
    pub content: MessageContent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum AssistantEvent {
    Start,
    #[serde(rename_all = "camelCase")]
    TextStart {
        content_index: u64,
    },
    #[serde(rename_all = "camelCase")]
    TextDelta {
        content_index: u64,
        delta: String,
    },
    #[serde(rename_all = "camelCase")]
    TextEnd {
        content_index: u64,
        content: String,
    },
    #[serde(rename_all = "camelCase")]
    ThinkingStart {
        content_index: u64,
    },
    #[serde(rename_all = "camelCase")]
    ThinkingDelta {
        content_index: u64,
        delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ThinkingEnd {
        content_index: u64,
        content: String,
    },
    #[serde(rename = "toolcall_start", rename_all = "camelCase")]
    ToolcallStart {
        content_index: u64,
        id: String,
        tool_name: String,
    },
    #[serde(rename = "toolcall_delta", rename_all = "camelCase")]
    ToolcallDelta {
        content_index: u64,
        delta: String,
    },
    #[serde(rename = "toolcall_end", rename_all = "camelCase")]
    ToolcallEnd {
        content_index: u64,
        tool_call: ContentBlock,
    },
    Done {
        reason: String,
        message: Message,
    },
    Error {
        reason: String,
        error: Message,
    },
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum CompactionReason {
    Manual,
    Threshold,
    Overflow,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "source")]
pub enum SummarySource {
    #[serde(rename = "branchSummary")]
    BranchSummary,
    #[serde(rename = "compaction")]
    Compaction { reason: CompactionReason },
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KnownEvent {
    AgentStart,
    #[serde(rename_all = "camelCase")]
    AgentEnd {
        messages: Vec<Message>,
        will_retry: bool,
    },
    AgentSettled {
        aborted: bool,
    },
    TurnStart,
    #[serde(rename_all = "camelCase")]
    TurnEnd {
        message: Message,
        tool_results: Vec<Message>,
    },
    MessageStart {
        message: Message,
    },
    #[serde(rename_all = "camelCase")]
    MessageUpdate {
        usage: Usage,
        assistant_message_event: AssistantEvent,
    },
    MessageEnd {
        message: Message,
    },
    #[serde(rename_all = "camelCase")]
    ToolExecutionStart {
        tool_call_id: String,
        tool_name: String,
        args: Value,
        parent_tool_call_id: Option<String>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    ToolExecutionUpdate {
        tool_call_id: String,
        tool_name: String,
        args: Value,
        partial_result: Value,
        parent_tool_call_id: Option<String>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    ToolExecutionEnd {
        tool_call_id: String,
        tool_name: String,
        result: Value,
        is_error: bool,
        #[serde(
            default,
            deserialize_with = "optional_number",
            skip_serializing_if = "Option::is_none"
        )]
        duration_ms: Option<f64>,
        parent_tool_call_id: Option<String>,
        #[serde(flatten)]
        extra: Extra,
    },
    #[serde(rename_all = "camelCase")]
    QueueUpdate {
        steering: Vec<String>,
        follow_up: Vec<String>,
    },
    EntryAppended {
        entry: SessionEntry,
    },
    SessionInfoChanged {
        name: Option<String>,
    },
    ThinkingLevelChanged {
        level: ThinkingLevel,
    },
    CompactionStart {
        reason: CompactionReason,
    },
    #[serde(rename_all = "camelCase")]
    CompactionEnd {
        reason: CompactionReason,
        result: Option<CompactionResult>,
        aborted: bool,
        will_retry: bool,
        error_message: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    AutoRetryStart {
        attempt: u64,
        max_attempts: u64,
        delay_ms: u64,
        error_message: String,
    },
    #[serde(rename_all = "camelCase")]
    AutoRetryEnd {
        success: bool,
        attempt: u64,
        final_error: Option<String>,
    },
    #[serde(rename_all = "camelCase")]
    SummarizationRetryScheduled {
        attempt: u64,
        max_attempts: u64,
        delay_ms: u64,
        error_message: String,
    },
    SummarizationRetryAttemptStart {
        #[serde(flatten)]
        source: SummarySource,
    },
    SummarizationRetryFinished,
    BashExecutionUpdate {
        id: Option<String>,
        delta: String,
    },
    #[serde(rename_all = "camelCase")]
    ExtensionError {
        extension_path: String,
        event: String,
        error: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UiRequest {
    pub id: String,
    #[serde(flatten)]
    pub method: UiMethod,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "method")]
pub enum UiMethod {
    #[serde(rename = "select")]
    Select {
        title: String,
        options: Vec<String>,
        timeout: Option<u64>,
    },
    #[serde(rename = "confirm")]
    Confirm {
        title: String,
        message: String,
        timeout: Option<u64>,
    },
    #[serde(rename = "input")]
    Input {
        title: String,
        placeholder: Option<String>,
        timeout: Option<u64>,
    },
    #[serde(rename = "editor")]
    Editor {
        title: String,
        prefill: Option<String>,
    },
    #[serde(rename = "notify", rename_all = "camelCase")]
    Notify {
        message: String,
        notify_type: Option<String>,
    },
    #[serde(rename = "setStatus", rename_all = "camelCase")]
    SetStatus {
        status_key: String,
        status_text: Option<String>,
    },
    #[serde(rename = "setWidget", rename_all = "camelCase")]
    SetWidget {
        widget_key: String,
        widget_lines: Option<Vec<String>>,
        widget_placement: Option<String>,
    },
    #[serde(rename = "setTitle")]
    SetTitle { title: String },
    #[serde(rename = "set_editor_text")]
    SetEditorText { text: String },
}
impl UiMethod {
    pub fn is_dialog(&self) -> bool {
        matches!(
            self,
            Self::Select { .. } | Self::Confirm { .. } | Self::Input { .. } | Self::Editor { .. }
        )
    }
}
#[derive(Debug, Clone, PartialEq)]
pub enum UiAnswer {
    Value(String),
    Confirmed(bool),
    Cancel,
}
impl UiAnswer {
    pub fn record(&self, request: &UiRequest) -> Result<Value> {
        let valid = match (&request.method, self) {
            (m, Self::Cancel) => m.is_dialog(),
            (UiMethod::Confirm { .. }, Self::Confirmed(_)) => true,
            (UiMethod::Select { options, .. }, Self::Value(v)) => options.contains(v),
            (UiMethod::Input { .. } | UiMethod::Editor { .. }, Self::Value(_)) => true,
            _ => false,
        };
        if !valid {
            return Err(Error::Protocol("UI response does not match dialog".into()));
        }
        let mut v = serde_json::json!({"type":"extension_ui_response","id":request.id});
        match self {
            Self::Value(s) => v["value"] = s.clone().into(),
            Self::Confirmed(b) => v["confirmed"] = (*b).into(),
            Self::Cancel => v["cancelled"] = true.into(),
        }
        Ok(v)
    }
}
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
        let r: Response = serde_json::from_value(v.clone()).map_err(|_| invalid())?;
        r.validate()?;
        return Ok(Record::Response(r));
    }
    if kind == "extension_ui_request" {
        let u: UiRequest = serde_json::from_value(v.clone()).map_err(|_| invalid())?;
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
        let e: KnownEvent = serde_json::from_value(v.clone()).map_err(|_| invalid())?;
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
