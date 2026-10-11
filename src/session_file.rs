//! Pi-native v3 JSONL sessions only. No history RPC, vendor conversion, prompt
//! injection, filesystem discovery, delivery, or model-context replay is provided.
use crate::{
    Error, Result, codec, json,
    protocol::{EntryData, Extra, Message, MessageContent, SessionEntry},
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

mod builder;
mod fields;
mod index;
pub use builder::SessionFileBuilder;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SessionHeader {
    pub r#type: HeaderType,
    pub version: u32,
    pub id: String,
    pub timestamp: String,
    pub cwd: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_session: Option<String>,
    #[serde(flatten)]
    pub extra: Extra,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum HeaderType {
    #[serde(rename = "session")]
    Session,
}
#[derive(Debug, Clone, PartialEq)]
pub struct SessionFile {
    pub header: SessionHeader,
    pub entries: Vec<SessionEntry>,
}
#[derive(Debug, Clone, Copy)]
pub struct SessionLimits {
    pub max_record_bytes: usize,
    pub max_file_bytes: usize,
    pub max_entries: usize,
}
impl Default for SessionLimits {
    fn default() -> Self {
        Self {
            max_record_bytes: 4 * 1024 * 1024,
            max_file_bytes: 64 * 1024 * 1024,
            max_entries: 100_000,
        }
    }
}
impl SessionFile {
    /// New native v3 header. IDs, ISO entry timestamp and cwd are caller-owned.
    pub fn new(
        id: impl Into<String>,
        timestamp: impl Into<String>,
        cwd: impl Into<String>,
    ) -> Self {
        Self {
            header: SessionHeader {
                r#type: HeaderType::Session,
                version: 3,
                id: id.into(),
                timestamp: timestamp.into(),
                cwd: cwd.into(),
                parent_session: None,
                extra: Extra::new(),
            },
            entries: Vec::new(),
        }
    }
    /// Parses v3 only. Legacy v1/v2 migration belongs to Pi, not this crate.
    /// CRLF is accepted, but all records must be LF terminated.
    pub fn parse(bytes: &[u8], limits: SessionLimits) -> Result<Self> {
        if bytes.len() > limits.max_file_bytes {
            return Err(Error::Capacity("session file bytes"));
        }
        if !bytes.ends_with(b"\n") {
            return Err(Error::Protocol("session file must end with LF".into()));
        }
        let mut records = bytes[..bytes.len() - 1].split(|b| *b == b'\n');
        let header = records
            .next()
            .ok_or_else(|| Error::Protocol("missing session header".into()))?;
        fn parse<T: serde::de::DeserializeOwned>(bytes: &[u8], limit: usize) -> Result<T> {
            let bytes = bytes.strip_suffix(b"\r").unwrap_or(bytes);
            if bytes.len() > limit {
                return Err(Error::Capacity("session record bytes"));
            }
            json::parse(bytes)
        }
        let header: SessionHeader = parse(header, limits.max_record_bytes)?;
        if header.version != 3 {
            return Err(Error::Protocol(
                "only native session version 3 is supported".into(),
            ));
        }
        let mut entries = Vec::new();
        for record in records {
            if entries.len() >= limits.max_entries {
                return Err(Error::Capacity("session entries"));
            }
            entries.push(parse(record, limits.max_record_bytes)?);
        }
        let file = Self { header, entries };
        file.validate()?;
        Ok(file)
    }
    pub fn encode(&self, limits: SessionLimits) -> Result<Vec<u8>> {
        // Reject over-budget output before validation can clone message fields.
        if self.entries.len() > limits.max_entries {
            return Err(Error::Capacity("session entries"));
        }
        fn record<T: Serialize>(
            value: &T,
            limits: SessionLimits,
            remaining: usize,
        ) -> Result<Vec<u8>> {
            let body_budget = remaining
                .checked_sub(1)
                .ok_or(Error::Capacity("session file bytes"))?;
            let limit = limits.max_record_bytes.min(body_budget);
            codec::encode(value, limit).map_err(|error| {
                if error == Error::Capacity("record bytes") && body_budget < limits.max_record_bytes
                {
                    Error::Capacity("session file bytes")
                } else {
                    error
                }
            })
        }
        let mut bytes = record(&self.header, limits, limits.max_file_bytes)?;
        for entry in &self.entries {
            let next = record(entry, limits, limits.max_file_bytes - bytes.len())?;
            bytes.extend(next);
        }
        self.validate()?;
        Ok(bytes)
    }
    /// Appends a native message entry without changing or converting its role.
    /// Message timestamps are Unix milliseconds, entry timestamps are ISO strings.
    pub fn append_message(
        &mut self,
        id: impl Into<String>,
        parent_id: Option<String>,
        timestamp: impl Into<String>,
        message: Message,
    ) -> Result<()> {
        let message = crate::owned::Owned::new(message);
        let entry = SessionEntry {
            id: id.into(),
            parent_id,
            timestamp: timestamp.into(),
            data: EntryData::Message {
                message: message.take(),
            },
        };
        self.entries.push(entry);
        if let Err(e) = self.validate() {
            if let Some(entry) = self.entries.pop() {
                drop(crate::owned::Owned::new(entry));
            }
            return Err(e);
        }
        Ok(())
    }
    /// Checks uniqueness, append-order parent links and branch-relative references.
    /// Multiple roots are valid. This does not rebuild Pi's compacted LLM context.
    pub fn validate(&self) -> Result<()> {
        if self.header.version != 3
            || self.header.id.is_empty()
            || self.header.timestamp.is_empty()
            || self.header.cwd.is_empty()
        {
            return Err(Error::Protocol("invalid native session header".into()));
        }
        fields::reserved(
            &self.header.extra,
            &["type", "version", "id", "timestamp", "cwd", "parentSession"],
        )?;
        json::check_serialized_depth(&self.header)?;
        let mut seen = HashSet::new();
        for e in &self.entries {
            if e.id.is_empty() || e.timestamp.is_empty() || !seen.insert(e.id.as_str()) {
                return Err(Error::Protocol(
                    "invalid or duplicate session entry ID".into(),
                ));
            }
            if let Some(p) = &e.parent_id {
                if p == &e.id || !seen.contains(p.as_str()) {
                    return Err(Error::Protocol("session parent must precede child".into()));
                }
            }
            fields::entry(&e.data)?;
            // Validate even manually constructed Message fields before exporting.
            if let EntryData::Message { message } = &e.data {
                validate_message(message)?;
            } else {
                json::check_serialized_depth(e)?;
            }
        }
        let index = index::Index::new(&self.entries);
        for e in &self.entries {
            match &e.data {
                EntryData::Compaction {
                    first_kept_entry_id,
                    ..
                } => {
                    if first_kept_entry_id != &e.id
                        && index
                            .on_branch(first_kept_entry_id, e.parent_id.as_deref())
                            .is_none()
                    {
                        return Err(Error::Protocol(
                            "compaction firstKeptEntryId is not on branch".into(),
                        ));
                    }
                }
                EntryData::ContextEdit { target_id, .. } => {
                    let target = index
                        .on_branch(target_id, e.parent_id.as_deref())
                        .ok_or_else(|| {
                            Error::Protocol("context edit target is not on branch".into())
                        })?;
                    if !matches!(&target.data,EntryData::Message{message} if matches!(message.role.as_str(),"user"|"assistant"|"toolResult"|"custom"))
                        && !matches!(target.data, EntryData::CustomMessage { .. })
                    {
                        return Err(Error::Protocol(
                            "context edit target is not editable".into(),
                        ));
                    }
                }
                EntryData::Label { target_id, .. } if !seen.contains(target_id.as_str()) => {
                    return Err(Error::Protocol("label target missing".into()));
                }
                _ => {}
            }
        }
        Ok(())
    }
    /// Raw root-to-leaf entries, not model context. A None leaf gives no entries.
    pub fn branch(&self, leaf: Option<&str>) -> Result<Vec<&SessionEntry>> {
        let index: HashMap<&str, &SessionEntry> =
            self.entries.iter().map(|e| (e.id.as_str(), e)).collect();
        let mut path = Vec::new();
        let mut cursor = leaf;
        let mut visited = HashSet::new();
        while let Some(id) = cursor {
            if !visited.insert(id) {
                return Err(Error::Protocol("session cycle".into()));
            }
            let e = *index
                .get(id)
                .ok_or_else(|| Error::Protocol("session leaf or parent missing".into()))?;
            path.push(e);
            cursor = e.parent_id.as_deref();
        }
        path.reverse();
        Ok(path)
    }
}
fn validate_message(message: &Message) -> Result<()> {
    fields::reserved(&message.fields, &["role"])?;
    // Native entry + message are two enclosing objects before any field value.
    for value in message.fields.values() {
        json::check_value_depth_at(value, 2)?;
    }
    let value = json::to_value(message).map_err(|_| Error::Protocol("invalid message".into()))?;
    json::decode::<Message>(&value)
        .map(|_| ())
        .map_err(|_| Error::Protocol("invalid native message".into()))
}
/// Convenience constructor for a native user message, not history-as-prompt.
pub fn user_message(content: MessageContent, timestamp_ms: u64) -> Result<Message> {
    let content = crate::owned::Owned::new(content);
    fields::content(&content)?;
    let content = json::to_value(&*content)?;
    json::decode(&serde_json::json!({"role":"user","content":content,"timestamp":timestamp_ms}))
        .map_err(|_| Error::Protocol("invalid user message".into()))
}
