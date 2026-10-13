//! Native messages, content blocks and role-specific validation.
use super::Extra;
use crate::json;
use serde::{Deserialize, Serialize};
use serde_json::Value;

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
        json::decode::<T>(f.get(key).ok_or_else(|| format!("missing {key}"))?)
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
