//! SDK-independent ordered binary transport. Never wrap a Pi process in a PTY.
use crate::{Error, Result, version};
use tokio::io::{AsyncRead, AsyncWrite};
pub type Reader = Box<dyn AsyncRead + Unpin + Send>;
pub type Writer = Box<dyn AsyncWrite + Unpin + Send>;
pub struct Transport {
    pub(crate) reader: Reader,
    pub(crate) writer: Writer,
    pub(crate) exit: Option<tokio::sync::watch::Receiver<Option<Error>>>,
}
impl Transport {
    /// The remote caller declares the installed version. There is no RPC
    /// version handshake and this constructor does not remotely verify it.
    /// Declare a concrete canonical release in [`crate::PI_VERSION_REQUIREMENT`].
    pub fn new<R, W>(reader: R, writer: W, declared_version: &str) -> Result<Self>
    where
        R: AsyncRead + Unpin + Send + 'static,
        W: AsyncWrite + Unpin + Send + 'static,
    {
        if !version::is_supported(declared_version) {
            return Err(Error::Version);
        }
        Ok(Self {
            reader: Box::new(reader),
            writer: Box::new(writer),
            exit: None,
        })
    }
}
