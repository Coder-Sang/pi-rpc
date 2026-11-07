//! Opt-in input tracking from Coder-Sang/Pi-Bolt, not official Pi/Bolt capability detection.
//!
//! The reviewed fork is bolt-v1.1.1 at SOURCE_COMMIT, based on Pi 1.5.0.
//! Select BoltMessageIdV1 explicitly for a trusted extension-capable executable
//! or Transport. Automatic dispatch continues to select official Pi profiles.
//! IDs and batches describe input handoff, not assistant completion or idempotency.
#![doc = include_str!("../../docs/bolt-message-id.md")]
use super::{Pi15, PiProtocol, Response, Sealed, UiRequest, v1_5};
use crate::{Error, Result, json};
use serde::{Deserialize, Deserializer, Serialize, Serializer, de};
use serde_json::Value;
use std::{fmt, sync::Arc};

/// Fixed source defining this extension, independently of the base Pi baseline.
pub const SOURCE_COMMIT: &str = "7193587994d2fee4b62a213c1f14d82acb832d5c";

/// A nonempty opaque input ID. Whitespace is preserved; clones share the string.
/// It is independent of the SDK-generated RPC command ID.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct MessageId(Arc<String>);
impl MessageId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        if value.is_empty() {
            return Err(Error::Protocol(
                "message_id must be a non-empty string".into(),
            ));
        }
        Ok(Self(Arc::new(value)))
    }
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
    pub(crate) fn shared(&self) -> Arc<String> {
        self.0.clone()
    }
}
impl AsRef<str> for MessageId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
impl fmt::Display for MessageId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.as_str().fmt(f)
    }
}
impl Serialize for MessageId {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}
impl<'de> Deserialize<'de> for MessageId {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        Self::new(String::deserialize(d)?).map_err(de::Error::custom)
    }
}

/// An identified input's acceptance. It does not mean consumption or completion.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InputAcceptance<T> {
    pub disposition: T,
    pub message_id: MessageId,
}

/// This type deliberately represents only tracking schema version 1.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrackingVersion {
    V1,
}
impl Serialize for TrackingVersion {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_u8(1)
    }
}
impl<'de> Deserialize<'de> for TrackingVersion {
    fn deserialize<D: Deserializer<'de>>(d: D) -> std::result::Result<Self, D::Error> {
        match u8::deserialize(d)? {
            1 => Ok(Self::V1),
            _ => Err(de::Error::custom("unsupported input tracking version")),
        }
    }
}
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum InputSource {
    Prompt,
    Steer,
    FollowUp,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ClearedInputs {
    pub steering: Vec<Option<MessageId>>,
    pub follow_up: Vec<Option<MessageId>>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "reason", rename_all = "snake_case")]
pub enum QueueChange {
    #[serde(rename_all = "camelCase")]
    Enqueue {
        source: InputSource,
        message_ids: Vec<Option<MessageId>>,
    },
    #[serde(rename_all = "camelCase")]
    Consume {
        source: InputSource,
        message_ids: Vec<Option<MessageId>>,
    },
    #[serde(rename_all = "camelCase")]
    PromptStart {
        source: InputSource,
        message_ids: Vec<Option<MessageId>>,
    },
    #[serde(rename_all = "camelCase")]
    Clear { message_ids: ClearedInputs },
}
fn nonempty<'de, D: Deserializer<'de>>(d: D) -> std::result::Result<String, D::Error> {
    let value = String::deserialize(d)?;
    if value.is_empty() {
        return Err(de::Error::custom("empty tracking identifier"));
    }
    Ok(value)
}
fn optional<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(d).map(Some)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "TrackingWire")]
pub struct QueueTrackingV1 {
    pub version: TrackingVersion,
    pub instance_id: String,
    pub sequence: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub batch_id: Option<String>,
    pub change: QueueChange,
    pub steering_message_ids: Vec<Option<MessageId>>,
    pub follow_up_message_ids: Vec<Option<MessageId>>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct TrackingWire {
    version: TrackingVersion,
    #[serde(deserialize_with = "nonempty")]
    instance_id: String,
    sequence: u64,
    #[serde(default, deserialize_with = "optional")]
    batch_id: Option<String>,
    change: QueueChange,
    steering_message_ids: Vec<Option<MessageId>>,
    follow_up_message_ids: Vec<Option<MessageId>>,
}
impl TryFrom<TrackingWire> for QueueTrackingV1 {
    type Error = &'static str;
    fn try_from(w: TrackingWire) -> std::result::Result<Self, Self::Error> {
        let handoff = matches!(
            w.change,
            QueueChange::Consume { .. } | QueueChange::PromptStart { .. }
        );
        if handoff != w.batch_id.is_some() || w.batch_id.as_ref().is_some_and(String::is_empty) {
            return Err("invalid input tracking batch");
        }
        match &w.change {
            QueueChange::PromptStart { source, .. } if *source != InputSource::Prompt => {
                return Err("invalid initial input tracking source");
            }
            QueueChange::Enqueue { source, .. } | QueueChange::Consume { source, .. }
                if *source == InputSource::Prompt =>
            {
                return Err("invalid queued input tracking source");
            }
            _ => {}
        }
        Ok(Self {
            version: w.version,
            instance_id: w.instance_id,
            sequence: w.sequence,
            batch_id: w.batch_id,
            change: w.change,
            steering_message_ids: w.steering_message_ids,
            follow_up_message_ids: w.follow_up_message_ids,
        })
    }
}

/// The fork's queue snapshot, including optional runtime-only tracking.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", try_from = "QueueWire")]
pub struct QueueUpdate {
    pub steering: Vec<String>,
    pub follow_up: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tracking: Option<QueueTrackingV1>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct QueueWire {
    steering: Vec<String>,
    follow_up: Vec<String>,
    #[serde(default, deserialize_with = "optional")]
    tracking: Option<QueueTrackingV1>,
}
impl TryFrom<QueueWire> for QueueUpdate {
    type Error = &'static str;
    fn try_from(w: QueueWire) -> std::result::Result<Self, Self::Error> {
        if let Some(tracking) = &w.tracking
            && (tracking.steering_message_ids.len() != w.steering.len()
                || tracking.follow_up_message_ids.len() != w.follow_up.len())
        {
            return Err("input tracking IDs do not align with queue text");
        }
        Ok(Self {
            steering: w.steering,
            follow_up: w.follow_up,
            tracking: w.tracking,
        })
    }
}

/// All queue updates use QueueUpdate, even before tracking is enabled.
/// Other records keep the unmodified Pi15 event representation.
#[derive(Debug, Clone, PartialEq)]
pub enum Event {
    QueueUpdate(QueueUpdate),
    Pi(v1_5::Event),
}

/// Explicitly selects the audited fork extension on the Pi 1.5.0 protocol.
/// Version probing checks the base Pi release, not this capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BoltMessageIdV1;
impl PiProtocol for BoltMessageIdV1 {
    type Event = Event;
    const MINOR: u64 = 5;
}
impl Sealed for BoltMessageIdV1 {
    fn validate_response(response: &Response) -> Result<()> {
        if response.success
            && matches!(response.command.as_str(), "prompt" | "steer" | "follow_up")
            && let Some(id) = response
                .data
                .as_ref()
                .and_then(|data| data.get("message_id"))
            && !id.as_str().is_some_and(|value| !value.is_empty())
        {
            return Err(Error::Protocol("invalid message_id response".into()));
        }
        Ok(())
    }
    fn known(value: &Value) -> Result<Event> {
        if value.get("type").and_then(Value::as_str) == Some("queue_update") {
            json::decode(value).map(Event::QueueUpdate)
        } else {
            <Pi15 as Sealed>::known(value).map(Event::Pi)
        }
    }
    fn ui(request: UiRequest) -> Event {
        Event::Pi(<Pi15 as Sealed>::ui(request))
    }
    fn unknown(value: Value) -> Event {
        Event::Pi(<Pi15 as Sealed>::unknown(value))
    }
    fn response(response: Response) -> Event {
        Event::Pi(<Pi15 as Sealed>::response(response))
    }
    fn ui_request(event: &Event) -> Option<&UiRequest> {
        match event {
            Event::Pi(event) => <Pi15 as Sealed>::ui_request(event),
            Event::QueueUpdate(_) => None,
        }
    }
}
