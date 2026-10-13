//! Shared admission, pending requests and authoritative terminal state.
use super::Config;
use crate::{Error, Result, protocol::Response};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, oneshot, watch};

pub(super) struct Pending {
    pub(super) command: &'static str,
    pub(super) response: oneshot::Sender<Result<Response>>,
    pub(super) _admission: OwnedSemaphorePermit,
}
pub(super) struct State {
    pub(super) pending: HashMap<String, Pending>,
    pub(super) terminal: Option<Error>,
    pub(super) closing: bool,
}
pub(super) struct Shared {
    pub(super) state: Mutex<State>,
    pub(super) terminal: watch::Sender<Option<Error>>,
    pub(super) config: Config,
}
impl Shared {
    pub(super) fn finish(&self, error: Error) {
        let mut s = self.state.lock().unwrap();
        if s.terminal.is_some() {
            return;
        }
        s.terminal = Some(error.clone());
        for (_, p) in s.pending.drain() {
            let _ = p.response.send(Err(error.clone()));
        }
        self.terminal.send_replace(Some(error));
    }
    pub(super) fn reason(&self) -> Error {
        self.state
            .lock()
            .unwrap()
            .terminal
            .clone()
            .unwrap_or(Error::Closed)
    }
}
pub(super) struct PendingGuard {
    pub(super) shared: Arc<Shared>,
    pub(super) id: String,
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().pending.remove(&self.id);
    }
}
