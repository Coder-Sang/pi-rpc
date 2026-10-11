//! Iterative cleanup for caller-created JSON owned by library operations.
use crate::{
    json,
    protocol::{Command, Extra, Message},
};

pub(crate) trait Cleanup {
    fn cleanup(&mut self);
}

pub(crate) struct Owned<T: Cleanup>(Option<T>);
impl<T: Cleanup> Owned<T> {
    pub(crate) fn new(value: T) -> Self {
        Self(Some(value))
    }
    pub(crate) fn take(mut self) -> T {
        self.0.take().unwrap()
    }
}
impl<T: Cleanup> std::ops::Deref for Owned<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.0.as_ref().unwrap()
    }
}
impl<T: Cleanup> Drop for Owned<T> {
    fn drop(&mut self) {
        if let Some(value) = &mut self.0 {
            value.cleanup();
        }
    }
}
pub(crate) fn extra(extra: &mut Extra) {
    for value in std::mem::take(extra).into_values() {
        json::drop_value(value);
    }
}
impl Cleanup for Command {
    fn cleanup(&mut self) {
        match self {
            Command::Prompt { images, .. }
            | Command::Steer { images, .. }
            | Command::FollowUp { images, .. } => {
                for image in images.iter_mut().flatten() {
                    extra(&mut image.extra);
                }
            }
            _ => {}
        }
    }
}
impl Cleanup for Message {
    fn cleanup(&mut self) {
        extra(&mut self.fields);
    }
}
#[cfg(feature = "session_file")]
impl Cleanup for crate::protocol::MessageContent {
    fn cleanup(&mut self) {
        use crate::protocol::ContentBlock;
        if let Self::Blocks(blocks) = self {
            for block in blocks {
                match block {
                    ContentBlock::ToolCall {
                        arguments,
                        extra: fields,
                        ..
                    } => {
                        extra(arguments);
                        extra(fields);
                    }
                    ContentBlock::Text { extra: fields, .. }
                    | ContentBlock::Thinking { extra: fields, .. }
                    | ContentBlock::Image { extra: fields, .. } => extra(fields),
                }
            }
        }
    }
}
#[cfg(feature = "session_file")]
impl Cleanup for crate::protocol::SessionEntry {
    fn cleanup(&mut self) {
        use crate::protocol::EntryData;
        match &mut self.data {
            EntryData::Message { message } => message.cleanup(),
            EntryData::Usage {
                usage,
                extra: fields,
                ..
            } => {
                extra(&mut usage.extra);
                extra(&mut usage.cost.extra);
                extra(fields);
            }
            EntryData::Compaction { extra: fields, .. }
            | EntryData::BranchSummary { extra: fields, .. }
            | EntryData::Custom { extra: fields, .. } => extra(fields),
            EntryData::CustomMessage {
                content,
                extra: fields,
                ..
            } => {
                content.cleanup();
                extra(fields);
            }
            EntryData::ContextEdit {
                replacement: Some(replacement),
                ..
            } => replacement.content.cleanup(),
            _ => {}
        }
    }
}
#[cfg(feature = "session_file")]
impl Cleanup for crate::session_file::SessionFile {
    fn cleanup(&mut self) {
        extra(&mut self.header.extra);
        for entry in &mut self.entries {
            entry.cleanup();
        }
    }
}

impl Cleanup for Option<Vec<crate::protocol::ImageContent>> {
    fn cleanup(&mut self) {
        for image in self.iter_mut().flatten() {
            extra(&mut image.extra);
        }
    }
}
