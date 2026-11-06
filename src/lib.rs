//! Async, bounded Pi 1.3.0–1.5.0 RPC with statically versioned events.
//!
//! See the [README](https://github.com/Coder-Sang/pi-rpc/blob/v0.3.1/README.md)
//! for the default-feature local-process example. It is included below when
//! `subprocess` and `pi_1_5` are enabled; other builds document their enabled APIs.
#![cfg_attr(
    all(feature = "subprocess", feature = "pi_1_5"),
    doc = include_str!("../README.md")
)]
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

pub use client::{AutoConnection, Config, PiClient, PiConnection, PiEvents, UiPolicy};
#[cfg(feature = "pi_1_3")]
pub use protocol::Pi13;
#[cfg(feature = "pi_1_4")]
pub use protocol::Pi14;
#[cfg(feature = "pi_1_5")]
pub use protocol::Pi15;
pub use protocol::PiProtocol;
#[cfg(feature = "subprocess")]
pub use subprocess::{AutoProcess, PiProcess, ProcessOptions};
pub use transport::{ConnectionInfo, Transport, VersionSource};

/// Latest reviewed Pi release; not the entire supported version range.
pub const PI_VERSION: &str = "1.5.0";
/// Inclusive supported range of canonical three-component Pi releases.
pub const PI_VERSION_REQUIREMENT: &str = ">=1.3.0, <=1.5.0";
/// Upstream commit of the latest reviewed release, [`PI_VERSION`].
pub const SOURCE_COMMIT: &str = "f9a26365568127461c47a07a37dcb42c140a3ac5";

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
    #[error("Pi {version} requires the {feature} feature")]
    VersionNotEnabled {
        version: String,
        feature: &'static str,
    },
    #[error("no Pi protocol feature is enabled")]
    NoProtocolEnabled,
}
pub type Result<T> = std::result::Result<T, Error>;
