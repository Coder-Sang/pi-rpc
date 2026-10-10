//! Incremental admission for message-only bulk appends to an owned session.
use super::{SessionFile, validate_message};
use crate::{
    Error, Result,
    protocol::{EntryData, Message, SessionEntry},
};
use std::collections::HashSet;

/// Owns immutable history plus an ID index for linear-time bulk message construction.
/// Existing SessionFile fields remain public; only this builder controls mutation.
#[derive(Debug)]
pub struct SessionFileBuilder {
    file: SessionFile,
    ids: HashSet<String>,
}

impl SessionFileBuilder {
    /// Creates and validates a new native v3 header before accepting messages.
    pub fn new(
        id: impl Into<String>,
        timestamp: impl Into<String>,
        cwd: impl Into<String>,
    ) -> Result<Self> {
        Self::from_session_file(SessionFile::new(id, timestamp, cwd))
    }

    /// Validates existing history once and takes exclusive ownership of it.
    pub fn from_session_file(file: SessionFile) -> Result<Self> {
        file.validate()?;
        let ids = file.entries.iter().map(|entry| entry.id.clone()).collect();
        Ok(Self { file, ids })
    }

    /// Checks only the new message and its ID/parent; failures leave history unchanged.
    /// A parent may be any existing entry. None starts another native root.
    pub fn append_message(
        &mut self,
        id: impl Into<String>,
        parent_id: Option<String>,
        timestamp: impl Into<String>,
        message: Message,
    ) -> Result<()> {
        let id = id.into();
        let timestamp = timestamp.into();
        if id.is_empty() || timestamp.is_empty() || self.ids.contains(&id) {
            return Err(Error::Protocol(
                "invalid or duplicate session entry ID".into(),
            ));
        }
        if let Some(parent) = &parent_id {
            if !self.ids.contains(parent) {
                return Err(Error::Protocol("session parent must precede child".into()));
            }
        }
        validate_message(&message)?;
        self.ids.insert(id.clone());
        self.file.entries.push(SessionEntry {
            id,
            parent_id,
            timestamp,
            data: EntryData::Message { message },
        });
        Ok(())
    }

    /// Performs final full validation and transfers the native file without cloning.
    /// Bulk construction of message-only history is O(n) overall.
    pub fn finish(self) -> Result<SessionFile> {
        self.file.validate()?;
        Ok(self.file)
    }
}
