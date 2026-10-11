//! Reject flattened extensions that shadow typed wire fields before serialization.
use crate::{
    Error, Result,
    protocol::{ContentBlock, EntryData, Extra, MessageContent, Usage},
};

pub(super) fn reserved(fields: &Extra, names: &[&str]) -> Result<()> {
    if names.iter().any(|name| fields.contains_key(*name)) {
        return Err(Error::Protocol(
            "extension field conflicts with reserved native field".into(),
        ));
    }
    Ok(())
}
fn usage(usage: &Usage) -> Result<()> {
    reserved(
        &usage.extra,
        &[
            "input",
            "output",
            "cacheRead",
            "cacheWrite",
            "totalTokens",
            "cost",
        ],
    )?;
    reserved(
        &usage.cost.extra,
        &["input", "output", "cacheRead", "cacheWrite", "total"],
    )
}
pub(super) fn content(content: &MessageContent) -> Result<()> {
    if let MessageContent::Blocks(blocks) = content {
        for block in blocks {
            match block {
                ContentBlock::Text { extra, .. } => {
                    reserved(extra, &["type", "text", "textSignature"])
                }
                ContentBlock::Thinking { extra, .. } => reserved(
                    extra,
                    &["type", "thinking", "thinkingSignature", "redacted"],
                ),
                ContentBlock::Image { extra, .. } => reserved(extra, &["type", "data", "mimeType"]),
                ContentBlock::ToolCall { extra, .. } => {
                    reserved(extra, &["type", "id", "name", "arguments"])
                }
            }?;
        }
    }
    Ok(())
}
pub(super) fn entry(data: &EntryData) -> Result<()> {
    let fields = match data {
        EntryData::Usage {
            extra,
            usage: value,
            ..
        } => {
            usage(value)?;
            reserved(extra, &["kind", "provider", "model", "usage"])?;
            Some(extra)
        }
        EntryData::Compaction { extra, .. } => {
            reserved(extra, &["summary", "firstKeptEntryId", "tokensBefore"])?;
            Some(extra)
        }
        EntryData::BranchSummary { extra, .. } => {
            reserved(extra, &["fromId", "summary"])?;
            Some(extra)
        }
        EntryData::Custom { extra, .. } => {
            reserved(extra, &["customType"])?;
            Some(extra)
        }
        EntryData::CustomMessage {
            extra,
            content: value,
            ..
        } => {
            content(value)?;
            reserved(extra, &["customType", "content", "display"])?;
            Some(extra)
        }
        EntryData::ContextEdit {
            replacement: Some(value),
            ..
        } => {
            content(&value.content)?;
            None
        }
        _ => None,
    };
    if let Some(fields) = fields {
        reserved(fields, &["type", "id", "parentId", "timestamp"])?;
    }
    Ok(())
}
