//! One continuous reader, one serialized writer, ID-correlated command futures.
#![doc = include_str!("../docs/connections.md")]
use crate::{
    Error, Result,
    protocol::{Command, Event, Response},
    transport::Transport,
};
use serde::de::DeserializeOwned;
use std::{
    sync::{
        Arc,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    task::{JoinHandle, JoinSet},
};

mod commands;
mod outbound;
mod runtime;
mod state;
#[cfg(test)]
mod tests;

use outbound::WriteMessage;
use runtime::{read_loop, write_loop};
use state::{PendingGuard, Shared};

/// Dialog policy selected before connecting; one-way UI notifications are delivered in both modes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPolicy {
    /// Automatically cancel every dialog, including confirm, while still delivering its event.
    Cancel,
    /// Let the host answer dialogs with [`PiClient::respond_ui`].
    Manual,
}
/// Connection admission, wire budgets and deadlines.
///
/// Capacity limits must be positive and fit Tokio permit counts. They bound serialized
/// records and queues, not total process memory. See the
/// [connection guide](https://github.com/Coder-Sang/pi-rpc/blob/main/docs/connections.md#bounds-and-backpressure).
#[derive(Debug, Clone)]
pub struct Config {
    /// Maximum JSON bytes per inbound or outbound record, excluding LF/CR. Default: 32 MiB.
    pub max_record_bytes: usize,
    /// Maximum concurrently admitted commands, including preparation and response waits. Default: 128.
    pub max_pending_commands: usize,
    /// Maximum queued writer records; a slot is reserved before encoding. Default: 128.
    pub max_writer_records: usize,
    /// Shared byte budget for encoding, queued and in-flight output, including LF.
    /// Held through write/flush. Default: 64 MiB.
    pub max_writer_bytes: usize,
    /// Maximum queued events in the single-consumer stream. Default: 1,024.
    pub max_event_count: usize,
    /// Sum of queued events' original UTF-8 JSON record bytes, excluding LF/CR.
    /// Default: 128 MiB; decoded representations may occupy more memory.
    pub max_event_bytes: usize,
    /// Response deadline after local enqueue, including the write/flush wait. Default: 30 seconds.
    /// Timeout leaves the send/execution outcome uncertain and does not retry the command.
    pub command_timeout: Duration,
    /// Deadline for orderly connection close and write draining. Default: five seconds.
    pub close_timeout: Duration,
    /// Extension dialog policy. Default: [`UiPolicy::Cancel`].
    pub ui_policy: UiPolicy,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_record_bytes: 32 * 1024 * 1024,
            max_pending_commands: 128,
            max_writer_records: 128,
            max_writer_bytes: 64 * 1024 * 1024,
            max_event_count: 1024,
            max_event_bytes: 128 * 1024 * 1024,
            command_timeout: Duration::from_secs(30),
            close_timeout: Duration::from_secs(5),
            ui_policy: UiPolicy::Cancel,
        }
    }
}
impl Config {
    fn validate(&self) -> Result<()> {
        for n in [
            self.max_record_bytes,
            self.max_pending_commands,
            self.max_writer_records,
            self.max_writer_bytes,
            self.max_event_count,
            self.max_event_bytes,
        ] {
            if n == 0 || n > u32::MAX as usize || n > Semaphore::MAX_PERMITS {
                return Err(Error::Capacity("invalid configuration"));
            }
        }
        Ok(())
    }
}
struct BufferedEvent {
    event: Event,
    _budget: OwnedSemaphorePermit,
}
/// Clone drops are inert. Dropping a command future never withdraws a queued record.
#[derive(Clone)]
pub struct PiClient {
    shared: Arc<Shared>,
    writer: mpsc::Sender<WriteMessage>,
    budget: Arc<Semaphore>,
    ids: Arc<AtomicU64>,
    admission: Arc<Semaphore>,
}
/// Single consumer, ordered across all session events. Not a per-prompt stream.
pub struct PiEvents {
    rx: mpsc::Receiver<BufferedEvent>,
    terminal: watch::Receiver<Option<Error>>,
    error_delivered: bool,
}
impl PiEvents {
    /// Drains buffered events, then waits for the supervisor's authoritative
    /// terminal reason. Channel closure can precede terminal publication.
    /// Cancelling this await does not consume the terminal error.
    pub async fn recv(&mut self) -> Option<Result<Event>> {
        if let Some(e) = self.rx.recv().await {
            return Some(Ok(e.event));
        }
        if self.error_delivered {
            return None;
        }
        loop {
            // Clone before awaiting so a watch read guard never blocks finish.
            let reason = self.terminal.borrow().clone();
            if let Some(reason) = reason {
                self.error_delivered = true;
                return Some(Err(reason));
            }
            if self.terminal.changed().await.is_err() {
                // Defensive fallback if every authority disappears without a
                // reason. Owner drop normally publishes Closed before aborting.
                self.error_delivered = true;
                return Some(Err(Error::Closed));
            }
        }
    }
}
/// Sole background-task owner. Drop closes the connection and aborts all I/O tasks.
/// A remote transport is dropped, not remotely terminated.
pub struct PiConnection {
    client: PiClient,
    task: Option<JoinHandle<()>>,
    terminal: watch::Receiver<Option<Error>>,
}
impl PiConnection {
    /// Begins reading immediately, with the event buffer installed before return.
    pub fn connect(transport: Transport, config: Config) -> Result<(PiClient, PiEvents, Self)> {
        config.validate()?;
        let (terminal, terminal_rx) = watch::channel(None);
        let shared = Arc::new(Shared::new(config.clone(), terminal));
        let (writer, writer_rx) = mpsc::channel(config.max_writer_records);
        let client = PiClient {
            shared: shared.clone(),
            writer,
            budget: Arc::new(Semaphore::new(config.max_writer_bytes)),
            ids: Arc::new(AtomicU64::new(1)),
            admission: Arc::new(Semaphore::new(config.max_pending_commands)),
        };
        let (events, event_rx) = mpsc::channel(config.max_event_count);
        let event_budget = Arc::new(Semaphore::new(config.max_event_bytes));
        let read_client = client.clone();
        let mut stop = terminal_rx.clone();
        let task = tokio::spawn(async move {
            let mut tasks = JoinSet::new();
            tasks.spawn(read_loop(
                transport.reader,
                transport.exit,
                read_client,
                events,
                event_budget,
            ));
            tasks.spawn(write_loop(transport.writer, writer_rx));
            let result = tokio::select! {
                r=tasks.join_next()=>match r { Some(Ok(r))=>r, _=>Err(Error::Transport("I/O task failed".into())) },
                _=stop.changed()=>Ok(())
            };
            if let Err(e) = result {
                shared.finish(e);
            }
            tasks.abort_all();
            while tasks.join_next().await.is_some() {}
        });
        let events = PiEvents {
            rx: event_rx,
            terminal: terminal_rx.clone(),
            error_delivered: false,
        };
        let owner = Self {
            client: client.clone(),
            task: Some(task),
            terminal: terminal_rx,
        };
        Ok((client, events, owner))
    }
    pub fn terminal_reason(&self) -> Option<Error> {
        self.terminal.borrow().clone()
    }
    pub async fn wait_terminal(&mut self) -> Error {
        loop {
            if let Some(e) = self.terminal.borrow().clone() {
                return e;
            }
            if self.terminal.changed().await.is_err() {
                return Error::Closed;
            }
        }
    }
    /// Excludes all submissions, drains already queued writes, shuts down stdin,
    /// and keeps reading until EOF or deadline. Does not issue abort/clear_queue.
    pub async fn close(&mut self) -> Error {
        self.client.shared.begin_close();
        let deadline = self.client.shared.config.close_timeout;
        let writer = self.client.writer.clone();
        let result = tokio::time::timeout(deadline, async {
            let _ = writer.send(WriteMessage::Close).await;
            self.wait_terminal().await
        })
        .await;
        let reason = match result {
            Ok(e) => e,
            Err(_) => {
                self.client.shared.finish(Error::CloseTimeout);
                Error::CloseTimeout
            }
        };
        if let Some(t) = self.task.take() {
            let _ = t.await;
        }
        reason
    }
    #[cfg(feature = "subprocess")]
    pub(crate) fn fail(&self, reason: Error) {
        self.client.shared.finish(reason);
    }
    #[cfg(feature = "subprocess")]
    pub(crate) fn terminal_receiver(&self) -> watch::Receiver<Option<Error>> {
        self.terminal.clone()
    }
}
impl Drop for PiConnection {
    fn drop(&mut self) {
        self.client.shared.finish(Error::Closed);
        if let Some(t) = self.task.take() {
            t.abort();
        }
    }
}
impl PiClient {
    /// No automatic retries. Capacity failure means not submitted. A timeout
    /// after local enqueue has an uncertain send/execution outcome.
    pub fn command(
        &self,
        command: Command,
    ) -> impl std::future::Future<Output = Result<Response>> + Send {
        self.command_with_timeout(command, self.shared.config.command_timeout)
    }
    pub fn command_with_timeout(
        &self,
        command: Command,
        timeout: Duration,
    ) -> impl std::future::Future<Output = Result<Response>> + Send {
        let command = crate::owned::Owned::new(command);
        async move {
            let id = format!("pi-rpc-{}", self.ids.fetch_add(1, Ordering::Relaxed));
            let name = command.name();
            // Avoid copying rejected or oversized prompts into a temporary Value.
            self.shared.check_command_capacity()?;
            let _admission = self
                .admission
                .clone()
                .try_acquire_owned()
                .map_err(|_| Error::Capacity("pending commands"))?;
            #[derive(serde::Serialize)]
            struct CommandRecord<'a> {
                id: &'a str,
                #[serde(flatten)]
                command: &'a Command,
            }
            let prepared = self.prepare_record(&CommandRecord {
                id: &id,
                command: &command,
            })?;
            // The owned input (including Base64) is no longer needed once the
            // budgeted output exists; do not retain it while awaiting a reply.
            drop(command);
            let (response, rx) = oneshot::channel();
            let sent = self
                .shared
                .enqueue_command(&id, name, response, _admission, prepared)?;
            let _guard = PendingGuard::new(self.shared.clone(), id);
            tokio::time::timeout(timeout, async {
                sent.await.map_err(|_| self.shared.reason())??;
                rx.await.map_err(|_| self.shared.reason())?
            })
            .await
            .map_err(|_| Error::Timeout)?
        }
    }
    pub fn typed<T: DeserializeOwned>(
        &self,
        command: Command,
    ) -> impl std::future::Future<Output = Result<T>> + Send {
        let response = self.command(command);
        async move { response.await?.decode() }
    }
}
