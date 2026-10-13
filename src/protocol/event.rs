//! Known session events and assistant message deltas.
use super::{
    CompactionResult, ContentBlock, Extra, Message, SessionEntry, ThinkingLevel, Usage,
    optional_number,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
