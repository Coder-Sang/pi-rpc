//! Writer reservations, bounded encoding and one-way UI submissions.
use super::PiClient;
use crate::{
    Error, Result, codec,
    protocol::{UiAnswer, UiRequest},
};
use tokio::sync::{OwnedSemaphorePermit, mpsc, oneshot};

pub(super) struct Outbound {
    pub(super) bytes: Vec<u8>,
    pub(super) _budget: OwnedSemaphorePermit,
    pub(super) sent: oneshot::Sender<Result<()>>,
}
pub(super) enum WriteMessage {
    Record(Outbound),
    Close,
}
pub(super) struct PreparedRecord {
    slot: mpsc::OwnedPermit<WriteMessage>,
    pub(super) bytes: Vec<u8>,
    budget: OwnedSemaphorePermit,
}
impl PreparedRecord {
    // Caller holds the admission state lock and has checked closing/terminal.
    pub(super) fn enqueue(self) -> oneshot::Receiver<Result<()>> {
        let (sent, rx) = oneshot::channel();
        self.slot.send(WriteMessage::Record(Outbound {
            bytes: self.bytes,
            _budget: self.budget,
            sent,
        }));
        rx
    }
}
impl PiClient {
    pub(super) fn check_open(&self) -> Result<()> {
        let state = self.shared.state.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closing {
            return Err(Error::Closed);
        }
        Ok(())
    }
    pub(super) fn prepare_record<T: serde::Serialize>(&self, value: &T) -> Result<PreparedRecord> {
        let slot = self
            .writer
            .clone()
            .try_reserve_owned()
            .map_err(|error| match error {
                mpsc::error::TrySendError::Full(_) => Error::Capacity("writer records"),
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        let size = codec::measure(value, self.shared.config.max_record_bytes)?;
        self.check_open()?;
        let framed = size.checked_add(1).ok_or(Error::Capacity("writer bytes"))?;
        let budget = self
            .budget
            .clone()
            .try_acquire_many_owned(
                framed
                    .try_into()
                    .map_err(|_| Error::Capacity("writer bytes"))?,
            )
            .map_err(|_| Error::Capacity("writer bytes"))?;
        // Allocate only after the entire framed output has been reserved. The
        // second pass cannot grow beyond the measured size, even for test faults.
        let bytes = codec::encode_with_capacity(value, size, framed)?;
        if bytes.len() != framed {
            return Err(Error::Protocol(
                "record size changed during encoding".into(),
            ));
        }
        Ok(PreparedRecord {
            slot,
            bytes,
            budget,
        })
    }
    /// UI responses are write-only, never registered as pending commands.
    pub async fn respond_ui(&self, request: &UiRequest, answer: UiAnswer) -> Result<()> {
        let sent = self.send_one_way(|| answer.wire_record(request))?;
        drop(answer); // The queued output owns the reply bytes through flush.
        tokio::time::timeout(self.shared.config.command_timeout, async {
            sent.await.map_err(|_| self.shared.reason())?
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    pub(super) fn send_one_way<T: serde::Serialize>(
        &self,
        record: impl FnOnce() -> Result<T>,
    ) -> Result<oneshot::Receiver<Result<()>>> {
        // Check before validating/building the record, then serialize without
        // holding the state lock and check again atomically with queue admission.
        {
            let state = self.shared.state.lock().unwrap();
            if let Some(error) = &state.terminal {
                return Err(error.clone());
            }
            if state.closing {
                return Err(Error::Closed);
            }
        }
        let value = record()?;
        let prepared = self.prepare_record(&value)?;
        let s = self.shared.state.lock().unwrap();
        if let Some(e) = &s.terminal {
            return Err(e.clone());
        }
        if s.closing {
            return Err(Error::Closed);
        }
        Ok(prepared.enqueue())
    }
    /// Auto-replies share close's admission lock; late dialogs remain events.
    pub(super) fn cancel_ui_if_open(&self, request: &UiRequest) -> Result<()> {
        {
            let state = self.shared.state.lock().unwrap();
            if let Some(error) = &state.terminal {
                return Err(error.clone());
            }
            if state.closing {
                return Ok(());
            }
        }
        let record = UiAnswer::Cancel;
        let prepared = self.prepare_record(&record.wire_record(request)?);
        // The final decision shares close's state lock. Closing skips automatic
        // replies even if preparation raced with exhausted writer capacity.
        let state = self.shared.state.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closing {
            return Ok(());
        }
        prepared?.enqueue();
        Ok(())
    }
}
