//! Shared native session entries and RPC session-tree types.
use super::{Extra, Message, MessageContent, Usage, required_nullable};
use serde::{Deserialize, Serialize};

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
