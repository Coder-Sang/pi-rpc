//! Sealed, statically selected protocol implementations.
use super::{Response, UiRequest};
use crate::Result;
use serde_json::Value;

/// An audited Pi minor protocol, selected once when connecting.
///
/// Implementations are sealed. Cargo protocol features are additive; enabling
/// another version never changes the event type of an existing implementation.
// The private bound intentionally seals implementations AND keeps decoding hooks
// inaccessible through downstream generic bounds. A public hidden supertrait
// would still expose those hooks as P::known/P::unknown.
#[allow(private_bounds)]
pub trait PiProtocol:
    Sealed + std::fmt::Debug + Clone + Copy + PartialEq + Eq + Send + Sync + 'static
{
    /// The nominal event type belonging to this protocol.
    type Event: std::fmt::Debug + Clone + PartialEq + Send + Sync + 'static;
    /// Pi minor version represented by this implementation.
    const MINOR: u64;

    /// Whether a canonical, globally supported release uses this protocol.
    fn matches_version(version: &str) -> bool {
        crate::version::minor(version) == Some(Self::MINOR)
    }
}

pub(crate) trait Sealed {
    fn validate_response(_response: &Response) -> Result<()> {
        Ok(())
    }
    fn known(value: &Value) -> Result<<Self as PiProtocol>::Event>
    where
        Self: PiProtocol;
    fn ui(request: UiRequest) -> <Self as PiProtocol>::Event
    where
        Self: PiProtocol;
    fn unknown(value: Value) -> <Self as PiProtocol>::Event
    where
        Self: PiProtocol;
    fn response(response: Response) -> <Self as PiProtocol>::Event
    where
        Self: PiProtocol;
    fn ui_request(event: &<Self as PiProtocol>::Event) -> Option<&UiRequest>
    where
        Self: PiProtocol;
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
macro_rules! protocol_version {
    ($module:ident, $profile:ident, $minor:literal) => {
        /// Wire events for this audited Pi minor release family.
        pub mod $module {
            use super::{
                CompactionReason, CompactionResult, ContentBlock, Extra, Message, PiProtocol,
                Response, SessionEntry, SummarySource, ThinkingLevel, UiRequest, Usage,
                milliseconds::deserialize_milliseconds, optional_number,
            };
            use serde::{Deserialize, Serialize};
            use serde_json::Value;

            include!("protocol/event_types.rs");

            /// Statically selects this Pi minor protocol.
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $profile;

            impl PiProtocol for $profile {
                type Event = Event;
                const MINOR: u64 = $minor;
            }

            impl super::profile::Sealed for $profile {
                fn known(value: &Value) -> crate::Result<Event> {
                    let event: KnownEvent = crate::json::decode(value)?;
                    if let KnownEvent::MessageUpdate {
                        assistant_message_event,
                        ..
                    } = &event
                    {
                        let valid = match assistant_message_event {
                            AssistantEvent::ToolcallEnd { tool_call, .. } => {
                                matches!(tool_call, ContentBlock::ToolCall { .. })
                            }
                            AssistantEvent::Done { reason, message } => {
                                message.role == "assistant"
                                    && matches!(
                                        reason.as_str(),
                                        "stop" | "length" | "toolUse" | "deferred"
                                    )
                            }
                            AssistantEvent::Error { reason, error } => {
                                error.role == "assistant"
                                    && matches!(reason.as_str(), "aborted" | "error")
                            }
                            _ => true,
                        };
                        if !valid {
                            return Err(crate::Error::Protocol("invalid assistant event".into()));
                        }
                    }
                    Ok(Event::Known(event))
                }

                fn ui(request: UiRequest) -> Event {
                    Event::Ui(request)
                }
                fn unknown(value: Value) -> Event {
                    Event::Unknown(value)
                }
                fn response(response: Response) -> Event {
                    Event::UncorrelatedResponse(response)
                }
                fn ui_request(event: &Event) -> Option<&UiRequest> {
                    if let Event::Ui(request) = event {
                        Some(request)
                    } else {
                        None
                    }
                }
            }
        }
        pub use $module::$profile;
    };
}

#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
pub(super) use protocol_version;
