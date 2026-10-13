//! Shared admission, pending requests and authoritative terminal state.
use super::{Config, outbound::PreparedRecord};
use crate::{Error, Result, protocol::Response};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::{OwnedSemaphorePermit, oneshot, watch};

struct Pending {
    command: &'static str,
    response: oneshot::Sender<Result<Response>>,
    _admission: OwnedSemaphorePermit,
}
struct State {
    pending: HashMap<String, Pending>,
    terminal: Option<Error>,
    closing: bool,
}
impl State {
    fn check_terminal(&self) -> Result<()> {
        match &self.terminal {
            Some(error) => Err(error.clone()),
            None => Ok(()),
        }
    }
    fn check_open(&self) -> Result<()> {
        self.check_terminal()?;
        if self.closing {
            return Err(Error::Closed);
        }
        Ok(())
    }
    fn check_command_capacity(&self, max: usize) -> Result<()> {
        self.check_open()?;
        if self.pending.len() >= max {
            return Err(Error::Capacity("pending commands"));
        }
        Ok(())
    }
}

pub(super) struct Shared {
    state: Mutex<State>,
    terminal: watch::Sender<Option<Error>>,
    pub(super) config: Config,
}
impl Shared {
    pub(super) fn new(config: Config, terminal: watch::Sender<Option<Error>>) -> Self {
        Self {
            state: Mutex::new(State {
                pending: HashMap::new(),
                terminal: None,
                closing: false,
            }),
            terminal,
            config,
        }
    }
    pub(super) fn check_open(&self) -> Result<()> {
        self.state.lock().unwrap().check_open()
    }
    pub(super) fn check_command_capacity(&self) -> Result<()> {
        self.state
            .lock()
            .unwrap()
            .check_command_capacity(self.config.max_pending_commands)
    }
    pub(super) fn begin_close(&self) {
        self.state.lock().unwrap().closing = true;
    }
    pub(super) fn is_closing(&self) -> bool {
        self.state.lock().unwrap().closing
    }
    /// Admission and pending registration share close's lock. The writer may
    /// start immediately, but the reader cannot remove the ID before registration.
    pub(super) fn enqueue_command(
        &self,
        id: &str,
        command: &'static str,
        response: oneshot::Sender<Result<Response>>,
        admission: OwnedSemaphorePermit,
        prepared: PreparedRecord,
    ) -> Result<oneshot::Receiver<Result<()>>> {
        let mut state = self.state.lock().unwrap();
        state.check_command_capacity(self.config.max_pending_commands)?;
        let sent = prepared.enqueue();
        state.pending.insert(
            id.to_owned(),
            Pending {
                command,
                response,
                _admission: admission,
            },
        );
        Ok(sent)
    }
    pub(super) fn enqueue_reply(
        &self,
        prepared: PreparedRecord,
    ) -> Result<oneshot::Receiver<Result<()>>> {
        let state = self.state.lock().unwrap();
        state.check_open()?;
        Ok(prepared.enqueue())
    }
    pub(super) fn should_cancel_ui(&self) -> Result<bool> {
        let state = self.state.lock().unwrap();
        state.check_terminal()?;
        Ok(!state.closing)
    }
    /// Closing wins over preparation failure for automatic replies; terminal
    /// errors remain authoritative. Late UI requests still become ordered events.
    pub(super) fn enqueue_auto_reply(&self, prepared: Result<PreparedRecord>) -> Result<()> {
        let state = self.state.lock().unwrap();
        state.check_terminal()?;
        if state.closing {
            return Ok(());
        }
        prepared?.enqueue();
        Ok(())
    }
    /// Remove under the state lock, then deliver outside it. A missing ID stays
    /// observable; timed-out/cancelled or unsolicited IDs are ignored.
    pub(super) fn dispatch_response(&self, response: Response) -> Result<Option<Response>> {
        let Some(id) = &response.id else {
            return Ok(Some(response));
        };
        let pending = self.state.lock().unwrap().pending.remove(id);
        if let Some(pending) = pending {
            if pending.command != response.command {
                let error = Error::Protocol("response command does not match ID".into());
                let _ = pending.response.send(Err(error.clone()));
                return Err(error);
            }
            let result = if response.success {
                Ok(response)
            } else {
                Err(Error::Rejected {
                    command: response.command,
                    message: response.error.unwrap_or_default(),
                })
            };
            let _ = pending.response.send(result);
        }
        Ok(None)
    }
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
    #[cfg(test)]
    pub(super) fn pending_count(&self) -> usize {
        self.state.lock().unwrap().pending.len()
    }
    #[cfg(test)]
    pub(super) fn assert_unlocked(&self) {
        drop(
            self.state
                .try_lock()
                .expect("no state lock during encoding"),
        );
    }
}

pub(super) struct PendingGuard {
    shared: Arc<Shared>,
    id: String,
}
impl PendingGuard {
    pub(super) fn new(shared: Arc<Shared>, id: String) -> Self {
        Self { shared, id }
    }
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().pending.remove(&self.id);
    }
}

#[cfg(test)]
mod tests;
