//! Async, bounded Pi 1.1.0–1.2.0 RPC. See the README for lifecycle and gateway policy.
#![doc = include_str!("../README.md")]
#![forbid(unsafe_code)]
pub mod client;
pub mod codec;
mod json;
mod owned;
pub mod protocol;
#[cfg(feature = "session_file")]
pub mod session_file;
#[cfg(feature = "subprocess")]
pub mod subprocess;
pub mod transport;
mod version;

pub use client::{Config, PiClient, PiConnection, PiEvents, UiPolicy};
#[cfg(feature = "subprocess")]
pub use subprocess::{PiProcess, ProcessOptions};
pub use transport::Transport;

/// Latest reviewed Pi release; not the entire supported version range.
pub const PI_VERSION: &str = "1.2.0";
/// Inclusive supported range of canonical three-component Pi releases.
pub const PI_VERSION_REQUIREMENT: &str = ">=1.1.0, <=1.2.0";
/// Upstream commit of the latest reviewed release, [`PI_VERSION`].
pub const SOURCE_COMMIT: &str = "06a466935536f8c9bcacb2d5efc0fc021be1bdff";

/// Terminal errors are also delivered to every outstanding command and the owner.
/// Error messages never include inbound records or submitted prompts.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum Error {
    #[error("Pi rejected {command}: {message}")]
    Rejected { command: String, message: String },
    #[error("command response timed out after sending, outcome uncertain")]
    Timeout,
    #[error("protocol error: {0}")]
    Protocol(String),
    #[error("transport error: {0}")]
    Transport(String),
    #[error("process exited with code {0:?}")]
    ProcessExit(Option<i32>),
    #[error("bounded capacity exceeded: {0}")]
    Capacity(&'static str),
    #[error("event buffer overflow")]
    EventOverflow,
    #[error("peer reached EOF")]
    Eof,
    #[error("connection closed")]
    Closed,
    #[error("close deadline elapsed")]
    CloseTimeout,
    #[error("process terminated by owner")]
    Terminated,
    #[error("unsupported or mismatched Pi version (supported {PI_VERSION_REQUIREMENT})")]
    Version,
}
pub type Result<T> = std::result::Result<T, Error>;
