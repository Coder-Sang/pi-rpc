//! Wire types from the pinned Pi sources listed in docs/compatibility.md.
//! Tool, provider and extension payloads remain open JSON. Unknown events retain
//! their complete record. Known event decoding never falls back to Unknown.
use serde::Deserialize;
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

mod command;
mod decode;
mod event;
mod message;
mod milliseconds;
mod response;
mod session;
mod ui;

pub use command::{
    Command, PromptDisposition, QueueMode, QueuedInputDisposition, StreamingBehavior, ThinkingLevel,
};
pub use decode::{Event, Record, decode_record};
pub use event::{AssistantEvent, CompactionReason, KnownEvent, SummarySource};
pub use message::{ContentBlock, Cost, ImageContent, ImageType, Message, MessageContent, Usage};
pub use response::{
    BashResult, Commands, CompactionResult, Disposition, ExportPath, ForkMessage, ForkMessages,
    ForkResult, LastAssistantText, Level, Levels, Messages, ModelCycle, Models, Queue, Response,
    SessionChange, SessionState, SessionStats, SlashCommand, SourceInfo, TokenTotals,
};
pub use session::{ContextReplacement, Entries, EntryData, SessionEntry, SessionTreeNode, Tree};
pub use ui::{UiAnswer, UiMethod, UiRequest};
