//! Runtime process selection with one statically typed triple per version.
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use super::PiProcess;
use super::{Diagnostics, ProcessOptions, verify};
use crate::VersionSource;
use crate::{ConnectionInfo, Error, Result, version};
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use crate::{PiClient, PiEvents};

/// An automatically selected connection. Match once to obtain typed handles.
#[non_exhaustive]
pub enum AutoProcess {
    #[cfg(feature = "pi_1_3")]
    V1_3 {
        client: PiClient<crate::Pi13>,
        events: PiEvents<crate::Pi13>,
        process: PiProcess<crate::Pi13>,
    },
    #[cfg(feature = "pi_1_4")]
    V1_4 {
        client: PiClient<crate::Pi14>,
        events: PiEvents<crate::Pi14>,
        process: PiProcess<crate::Pi14>,
    },
    #[cfg(feature = "pi_1_5")]
    V1_5 {
        client: PiClient<crate::Pi15>,
        events: PiEvents<crate::Pi15>,
        process: PiProcess<crate::Pi15>,
    },
}

/// Probes the executable, then selects the corresponding enabled protocol.
/// Disabled protocols are rejected before the RPC child is created.
pub async fn spawn_auto(options: ProcessOptions) -> Result<AutoProcess> {
    crate::version::Expectation::new(&options.expected_version)?;
    if !cfg!(any(
        feature = "pi_1_3",
        feature = "pi_1_4",
        feature = "pi_1_5"
    )) {
        return Err(Error::NoProtocolEnabled);
    }
    let actual = verify(&options).await?;
    spawn_selected(options, &actual, VersionSource::Probed)
}

/// Selects an enabled protocol using a trusted declaration, skipping `--version`.
/// The caller guarantees that the executable really runs the declared release.
/// All version expectations still apply; startup_timeout is unused.
pub async fn spawn_auto_with_version(
    options: ProcessOptions,
    version: &str,
) -> Result<AutoProcess> {
    let expectation = crate::version::Expectation::new(&options.expected_version)?;
    if !expectation.matches(version) {
        return Err(Error::Version);
    }
    spawn_selected(options, version, VersionSource::Declared)
}

fn spawn_selected(
    _options: ProcessOptions,
    version: &str,
    _source: VersionSource,
) -> Result<AutoProcess> {
    version::enabled(version)?;
    #[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
    macro_rules! spawn {
        ($profile:ty, $variant:ident) => {{
            let (client, events, process) =
                PiProcess::<$profile>::spawn_rpc(_options, version, _source)?;
            Ok(AutoProcess::$variant {
                client,
                events,
                process,
            })
        }};
    }
    match version::minor(version) {
        #[cfg(feature = "pi_1_3")]
        Some(3) => spawn!(crate::Pi13, V1_3),
        #[cfg(feature = "pi_1_4")]
        Some(4) => spawn!(crate::Pi14, V1_4),
        #[cfg(feature = "pi_1_5")]
        Some(5) => spawn!(crate::Pi15, V1_5),
        _ => unreachable!("enabled protocol checked before dispatch"),
    }
}

impl AutoProcess {
    pub fn diagnostics(&self) -> Diagnostics {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref process, .. } => process.diagnostics(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref process, .. } => process.diagnostics(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref process, .. } => process.diagnostics(),
        }
    }
    pub fn process_exit(&self) -> Option<Error> {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref process, .. } => process.process_exit(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref process, .. } => process.process_exit(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref process, .. } => process.process_exit(),
        }
    }
    pub async fn terminate(&mut self) -> Error {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 {
                ref mut process, ..
            } => process.terminate().await,
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 {
                ref mut process, ..
            } => process.terminate().await,
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 {
                ref mut process, ..
            } => process.terminate().await,
        }
    }
    pub fn info(&self) -> &ConnectionInfo {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref process, .. } => process.info(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref process, .. } => process.info(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref process, .. } => process.info(),
        }
    }
    pub fn terminal_reason(&self) -> Option<Error> {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 { ref process, .. } => process.terminal_reason(),
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 { ref process, .. } => process.terminal_reason(),
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 { ref process, .. } => process.terminal_reason(),
        }
    }
    pub async fn wait_terminal(&mut self) -> Error {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 {
                ref mut process, ..
            } => process.wait_terminal().await,
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 {
                ref mut process, ..
            } => process.wait_terminal().await,
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 {
                ref mut process, ..
            } => process.wait_terminal().await,
        }
    }
    pub async fn close(&mut self) -> Error {
        match *self {
            #[cfg(feature = "pi_1_3")]
            Self::V1_3 {
                ref mut process, ..
            } => process.close().await,
            #[cfg(feature = "pi_1_4")]
            Self::V1_4 {
                ref mut process, ..
            } => process.close().await,
            #[cfg(feature = "pi_1_5")]
            Self::V1_5 {
                ref mut process, ..
            } => process.close().await,
        }
    }
}
