//! Runtime selection preserves a statically typed triple in every variant.
use super::Config;
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use super::{PiClient, PiConnection, PiEvents};
use crate::{ConnectionInfo, Error, Result, Transport, version};

/// An automatically selected connection. Match once to obtain typed handles.
#[non_exhaustive]
pub enum AutoConnection {
    #[cfg(feature = "pi_1_3")]
    V1_3 {
        client: PiClient<crate::Pi13>,
        events: PiEvents<crate::Pi13>,
        connection: PiConnection<crate::Pi13>,
    },
    #[cfg(feature = "pi_1_4")]
    V1_4 {
        client: PiClient<crate::Pi14>,
        events: PiEvents<crate::Pi14>,
        connection: PiConnection<crate::Pi14>,
    },
    #[cfg(feature = "pi_1_5")]
    V1_5 {
        client: PiClient<crate::Pi15>,
        events: PiEvents<crate::Pi15>,
        connection: PiConnection<crate::Pi15>,
    },
}

/// Selects an enabled protocol from Transport's concrete declared version.
/// No handshake or I/O tasks are started when admission fails.
pub fn connect_auto(transport: Transport, config: Config) -> Result<AutoConnection> {
    config.validate()?;
    version::enabled(transport.info.version())?;
    #[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
    macro_rules! connect {
        ($profile:ty, $variant:ident) => {{
            let (client, events, connection) =
                PiConnection::<$profile>::connect(transport, config)?;
            Ok(AutoConnection::$variant {
                client,
                events,
                connection,
            })
        }};
    }
    match version::minor(transport.info.version()) {
        #[cfg(feature = "pi_1_3")]
        Some(3) => connect!(crate::Pi13, V1_3),
        #[cfg(feature = "pi_1_4")]
        Some(4) => connect!(crate::Pi14, V1_4),
        #[cfg(feature = "pi_1_5")]
        Some(5) => connect!(crate::Pi15, V1_5),
        _ => unreachable!("enabled protocol checked before dispatch"),
    }
}

impl AutoConnection {
    pub fn info(&self) -> &ConnectionInfo {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref connection, .. } => connection.info(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref connection, .. } => connection.info(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref connection, .. } => connection.info(),
        }
    }
    pub fn terminal_reason(&self) -> Option<Error> {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref connection, .. } => connection.terminal_reason(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref connection, .. } => connection.terminal_reason(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref connection, .. } => connection.terminal_reason(),
        }
    }
    pub async fn wait_terminal(&mut self) -> Error {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 {
                ref mut connection, ..
            } => connection.wait_terminal().await,
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 {
                ref mut connection, ..
            } => connection.wait_terminal().await,
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 {
                ref mut connection, ..
            } => connection.wait_terminal().await,
        }
    }
    pub async fn close(&mut self) -> Error {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 {
                ref mut connection, ..
            } => connection.close().await,
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 {
                ref mut connection, ..
            } => connection.close().await,
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 {
                ref mut connection, ..
            } => connection.close().await,
        }
    }
}
