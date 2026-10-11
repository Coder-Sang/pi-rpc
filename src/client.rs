//! One continuous reader, one serialized writer, ID-correlated command futures.
use crate::{Error, Result, codec, protocol::*, transport::Transport};
use serde::de::DeserializeOwned;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::Duration,
};
use tokio::{
    io::AsyncWriteExt,
    sync::{OwnedSemaphorePermit, Semaphore, mpsc, oneshot, watch},
    task::{JoinHandle, JoinSet},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UiPolicy {
    Cancel,
    Manual,
}
#[derive(Debug, Clone)]
pub struct Config {
    pub max_record_bytes: usize,
    pub max_pending_commands: usize,
    pub max_writer_records: usize,
    pub max_writer_bytes: usize,
    pub max_event_count: usize,
    /// Sum of queued events' original UTF-8 JSON record bytes, excluding LF/CR.
    pub max_event_bytes: usize,
    pub command_timeout: Duration,
    pub close_timeout: Duration,
    pub ui_policy: UiPolicy,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            max_record_bytes: 4 * 1024 * 1024,
            max_pending_commands: 128,
            max_writer_records: 128,
            max_writer_bytes: 8 * 1024 * 1024,
            max_event_count: 1024,
            max_event_bytes: 16 * 1024 * 1024,
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
struct Pending {
    command: &'static str,
    response: oneshot::Sender<Result<Response>>,
}
struct State {
    pending: HashMap<String, Pending>,
    terminal: Option<Error>,
    closing: bool,
}
struct Shared {
    state: Mutex<State>,
    terminal: watch::Sender<Option<Error>>,
    config: Config,
}
impl Shared {
    fn finish(&self, error: Error) {
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
    fn reason(&self) -> Error {
        self.state
            .lock()
            .unwrap()
            .terminal
            .clone()
            .unwrap_or(Error::Closed)
    }
}
struct Outbound {
    bytes: Vec<u8>,
    _budget: OwnedSemaphorePermit,
    sent: oneshot::Sender<Result<()>>,
}
enum WriteMessage {
    Record(Outbound),
    Close,
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
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                pending: HashMap::new(),
                terminal: None,
                closing: false,
            }),
            terminal,
            config: config.clone(),
        });
        let (writer, writer_rx) = mpsc::channel(config.max_writer_records);
        let client = PiClient {
            shared: shared.clone(),
            writer,
            budget: Arc::new(Semaphore::new(config.max_writer_bytes)),
            ids: Arc::new(AtomicU64::new(1)),
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
        {
            let mut s = self.client.shared.state.lock().unwrap();
            s.closing = true;
        }
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
struct PendingGuard {
    shared: Arc<Shared>,
    id: String,
}
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().pending.remove(&self.id);
    }
}
impl PiClient {
    fn enqueue_locked(&self, bytes: Vec<u8>) -> Result<oneshot::Receiver<Result<()>>> {
        let permit = self
            .budget
            .clone()
            .try_acquire_many_owned(
                bytes
                    .len()
                    .try_into()
                    .map_err(|_| Error::Capacity("writer bytes"))?,
            )
            .map_err(|_| Error::Capacity("writer bytes"))?;
        let (sent, rx) = oneshot::channel();
        self.writer
            .try_send(WriteMessage::Record(Outbound {
                bytes,
                _budget: permit,
                sent,
            }))
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => Error::Capacity("writer records"),
                mpsc::error::TrySendError::Closed(_) => Error::Closed,
            })?;
        Ok(rx)
    }
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
            {
                let state = self.shared.state.lock().unwrap();
                if let Some(error) = &state.terminal {
                    return Err(error.clone());
                }
                if state.closing {
                    return Err(Error::Closed);
                }
                if state.pending.len() >= self.shared.config.max_pending_commands {
                    return Err(Error::Capacity("pending commands"));
                }
            }
            #[derive(serde::Serialize)]
            struct CommandRecord<'a> {
                id: &'a str,
                #[serde(flatten)]
                command: &'a Command,
            }
            let bytes = codec::encode(
                &CommandRecord {
                    id: &id,
                    command: &command,
                },
                self.shared.config.max_record_bytes,
            )?;
            let (response, rx) = oneshot::channel();
            let sent = {
                let mut s = self.shared.state.lock().unwrap();
                if let Some(e) = &s.terminal {
                    return Err(e.clone());
                }
                if s.closing {
                    return Err(Error::Closed);
                }
                if s.pending.len() >= self.shared.config.max_pending_commands {
                    return Err(Error::Capacity("pending commands"));
                }
                let sent = self.enqueue_locked(bytes)?;
                s.pending.insert(
                    id.clone(),
                    Pending {
                        command: name,
                        response,
                    },
                );
                sent
            };
            let _guard = PendingGuard {
                shared: self.shared.clone(),
                id,
            };
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
    /// UI responses are write-only, never registered as pending commands.
    pub async fn respond_ui(&self, request: &UiRequest, answer: UiAnswer) -> Result<()> {
        let record = answer.record(request)?;
        let sent = self.send_one_way(record)?;
        tokio::time::timeout(self.shared.config.command_timeout, async {
            sent.await.map_err(|_| self.shared.reason())?
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    fn send_one_way(&self, value: Value) -> Result<oneshot::Receiver<Result<()>>> {
        let bytes = codec::encode(&value, self.shared.config.max_record_bytes)?;
        let s = self.shared.state.lock().unwrap();
        if let Some(e) = &s.terminal {
            return Err(e.clone());
        }
        if s.closing {
            return Err(Error::Closed);
        }
        self.enqueue_locked(bytes)
    }
    /// Auto-replies share close's admission lock; late dialogs remain events.
    fn cancel_ui_if_open(&self, request: &UiRequest) -> Result<()> {
        let state = self.shared.state.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closing {
            return Ok(());
        }
        let bytes = codec::encode(
            &UiAnswer::Cancel.record(request)?,
            self.shared.config.max_record_bytes,
        )?;
        // Only local admission is performed here; the continuous reader never
        // waits for flush or registers a normal pending command for this reply.
        self.enqueue_locked(bytes)?;
        Ok(())
    }
    pub async fn prompt(&self, message: impl Into<String>) -> Result<PromptDisposition> {
        Ok(self
            .typed::<Disposition<PromptDisposition>>(Command::Prompt {
                message: message.into(),
                images: None,
                streaming_behavior: None,
            })
            .await?
            .disposition)
    }
    pub fn prompt_with(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
        streaming_behavior: Option<StreamingBehavior>,
    ) -> impl std::future::Future<Output = Result<PromptDisposition>> + Send {
        let response = self.typed::<Disposition<PromptDisposition>>(Command::Prompt {
            message: message.into(),
            images,
            streaming_behavior,
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn steer(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::Steer {
            message: message.into(),
            images,
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn follow_up(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::FollowUp {
            message: message.into(),
            images,
        });
        async move { Ok(response.await?.disposition) }
    }
    pub async fn abort(&self) -> Result<()> {
        self.command(Command::Abort).await.map(|_| ())
    }
    pub async fn clear_queue(&self) -> Result<Queue> {
        self.typed(Command::ClearQueue).await
    }
    pub async fn get_state(&self) -> Result<SessionState> {
        self.typed(Command::GetState).await
    }
    pub async fn new_session(&self, parent_session: Option<String>) -> Result<SessionChange> {
        self.typed(Command::NewSession { parent_session }).await
    }
    pub async fn switch_session(&self, session_path: impl Into<String>) -> Result<SessionChange> {
        self.typed(Command::SwitchSession {
            session_path: session_path.into(),
        })
        .await
    }
    pub async fn fork(&self, entry_id: impl Into<String>) -> Result<ForkResult> {
        self.typed(Command::Fork {
            entry_id: entry_id.into(),
        })
        .await
    }
    pub async fn clone_session(&self) -> Result<SessionChange> {
        self.typed(Command::Clone).await
    }
    pub async fn get_messages(&self) -> Result<Messages> {
        self.typed(Command::GetMessages).await
    }
    pub async fn get_entries(&self, since: Option<String>) -> Result<Entries> {
        self.typed(Command::GetEntries { since }).await
    }
    pub async fn get_tree(&self) -> Result<Tree> {
        self.typed(Command::GetTree).await
    }
    pub async fn get_commands(&self) -> Result<Commands> {
        self.typed(Command::GetCommands).await
    }
    pub async fn get_session_stats(&self) -> Result<SessionStats> {
        self.typed(Command::GetSessionStats).await
    }
    pub async fn bash(
        &self,
        command: impl Into<String>,
        exclude_from_context: Option<bool>,
    ) -> Result<BashResult> {
        self.typed(Command::Bash {
            command: command.into(),
            exclude_from_context,
        })
        .await
    }
    pub async fn compact(&self, custom_instructions: Option<String>) -> Result<CompactionResult> {
        self.typed(Command::Compact {
            custom_instructions,
        })
        .await
    }
    pub async fn set_model(
        &self,
        provider: impl Into<String>,
        model_id: impl Into<String>,
    ) -> Result<Value> {
        self.typed(Command::SetModel {
            provider: provider.into(),
            model_id: model_id.into(),
        })
        .await
    }
    pub async fn cycle_model(&self) -> Result<Option<ModelCycle>> {
        self.typed(Command::CycleModel).await
    }
    pub async fn get_available_models(&self) -> Result<Models> {
        self.typed(Command::GetAvailableModels).await
    }
    pub async fn set_thinking_level(&self, level: ThinkingLevel) -> Result<()> {
        self.command(Command::SetThinkingLevel { level })
            .await
            .map(|_| ())
    }
    pub async fn cycle_thinking_level(&self) -> Result<Option<Level>> {
        self.typed(Command::CycleThinkingLevel).await
    }
    pub async fn get_available_thinking_levels(&self) -> Result<Levels> {
        self.typed(Command::GetAvailableThinkingLevels).await
    }
    pub async fn set_steering_mode(&self, mode: QueueMode) -> Result<()> {
        self.command(Command::SetSteeringMode { mode })
            .await
            .map(|_| ())
    }
    pub async fn set_follow_up_mode(&self, mode: QueueMode) -> Result<()> {
        self.command(Command::SetFollowUpMode { mode })
            .await
            .map(|_| ())
    }
    pub async fn set_auto_compaction(&self, enabled: bool) -> Result<()> {
        self.command(Command::SetAutoCompaction { enabled })
            .await
            .map(|_| ())
    }
    pub async fn set_auto_retry(&self, enabled: bool) -> Result<()> {
        self.command(Command::SetAutoRetry { enabled })
            .await
            .map(|_| ())
    }
    pub async fn abort_retry(&self) -> Result<()> {
        self.command(Command::AbortRetry).await.map(|_| ())
    }
    pub async fn abort_bash(&self) -> Result<()> {
        self.command(Command::AbortBash).await.map(|_| ())
    }
    pub async fn export_html(&self, output_path: Option<String>) -> Result<ExportPath> {
        self.typed(Command::ExportHtml { output_path }).await
    }
    pub async fn get_fork_messages(&self) -> Result<ForkMessages> {
        self.typed(Command::GetForkMessages).await
    }
    pub async fn get_last_assistant_text(&self) -> Result<LastAssistantText> {
        self.typed(Command::GetLastAssistantText).await
    }
    pub async fn set_session_name(&self, name: impl Into<String>) -> Result<()> {
        self.command(Command::SetSessionName { name: name.into() })
            .await
            .map(|_| ())
    }
}
async fn write_loop(
    mut writer: crate::transport::Writer,
    mut rx: mpsc::Receiver<WriteMessage>,
) -> Result<()> {
    while let Some(msg) = rx.recv().await {
        match msg {
            WriteMessage::Record(out) => {
                let r = async {
                    writer.write_all(&out.bytes).await?;
                    writer.flush().await
                }
                .await
                .map_err(|e: std::io::Error| Error::Transport(e.to_string()));
                let _ = out.sent.send(r.clone());
                r?;
            }
            WriteMessage::Close => {
                writer
                    .shutdown()
                    .await
                    .map_err(|e| Error::Transport(e.to_string()))?;
                drop(writer);
                std::future::pending::<()>().await;
                return Ok(());
            }
        }
    }
    Ok(())
}
async fn read_loop(
    reader: crate::transport::Reader,
    mut exit: Option<watch::Receiver<Option<Error>>>,
    client: PiClient,
    events: mpsc::Sender<BufferedEvent>,
    budget: Arc<Semaphore>,
) -> Result<()> {
    let mut reader = codec::JsonlReader::new(reader, client.shared.config.max_record_bytes);
    // Child exit and pipe EOF are independent: descendants can retain stdout.
    // After exit, drain ready records for at most 50ms, then fail even if a
    // descendant keeps producing output. An EOF gets 100ms exit-race grace.
    let mut exited: Option<(Error, tokio::time::Instant)> = None;
    loop {
        let record = if let Some((reason, deadline)) = &exited {
            tokio::select! { biased;
                _=tokio::time::sleep_until(*deadline)=>return Err(reason.clone()),
                record=reader.next()=>record?,
            }
        } else {
            tokio::select! { biased;
                reason=wait_exit(&mut exit)=> {
                    exited=Some((reason,tokio::time::Instant::now()+Duration::from_millis(50)));
                    continue;
                }
                record=reader.next()=>record?,
            }
        };
        let Some((value, size)) = record else {
            if let Some((reason, _)) = exited {
                return Err(reason);
            }
            if let Some(rx) = exit.as_mut() {
                let reason = tokio::time::timeout(Duration::from_millis(100), async {
                    loop {
                        if let Some(e) = rx.borrow().clone() {
                            return Some(e);
                        }
                        if rx.changed().await.is_err() {
                            return None;
                        }
                    }
                })
                .await;
                if let Ok(Some(e)) = reason {
                    return Err(e);
                }
            }
            return Err(if client.shared.state.lock().unwrap().closing {
                Error::Closed
            } else {
                Error::Eof
            });
        };
        let event = match decode_record(value)? {
            Record::Response(r) => {
                if let Some(id) = &r.id {
                    let pending = client.shared.state.lock().unwrap().pending.remove(id);
                    if let Some(p) = pending {
                        if p.command != r.command {
                            let e = Error::Protocol("response command does not match ID".into());
                            let _ = p.response.send(Err(e.clone()));
                            return Err(e);
                        }
                        let result = if r.success {
                            Ok(r)
                        } else {
                            Err(Error::Rejected {
                                command: r.command,
                                message: r.error.unwrap_or_default(),
                            })
                        };
                        let _ = p.response.send(result);
                    }
                    // Timed-out/cancelled futures and unsolicited IDs are ignored.
                    continue;
                }
                Event::UncorrelatedResponse(r)
            }
            Record::Event(e) => e,
        };
        if let Event::Ui(u) = &event {
            if u.method.is_dialog() && client.shared.config.ui_policy == UiPolicy::Cancel {
                // Enqueue without awaiting the writer, so the reader remains continuous.
                client.cancel_ui_if_open(u)?;
            }
        }
        let permit = budget
            .clone()
            .try_acquire_many_owned(size as u32)
            .map_err(|_| Error::EventOverflow)?;
        events
            .try_send(BufferedEvent {
                event,
                _budget: permit,
            })
            .map_err(|_| Error::EventOverflow)?;
    }
}
async fn wait_exit(exit: &mut Option<watch::Receiver<Option<Error>>>) -> Error {
    if let Some(rx) = exit {
        loop {
            if let Some(e) = rx.borrow().clone() {
                return e;
            }
            if rx.changed().await.is_err() {
                break;
            }
        }
    }
    std::future::pending().await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn closed_event_channel_waits_for_publication_and_cancelled_recv_keeps_error() {
        let (tx, rx) = mpsc::channel(1);
        let (terminal, terminal_rx) = watch::channel(None);
        let mut events = PiEvents {
            rx,
            terminal: terminal_rx,
            error_delivered: false,
        };
        drop(tx); // Deterministically create the reader/supervisor publication gap.
        assert!(
            tokio::time::timeout(Duration::from_millis(5), events.recv())
                .await
                .is_err()
        );
        terminal.send_replace(Some(Error::Transport("delayed terminal".into())));
        assert_eq!(
            events.recv().await,
            Some(Err(Error::Transport("delayed terminal".into())))
        );
        assert_eq!(events.recv().await, None);
    }

    #[tokio::test]
    async fn terminal_authority_disappearing_after_event_close_returns_closed_once() {
        let (tx, rx) = mpsc::channel(1);
        let (terminal, terminal_rx) = watch::channel(None);
        let mut events = PiEvents {
            rx,
            terminal: terminal_rx,
            error_delivered: false,
        };
        drop(tx);
        drop(terminal);
        assert_eq!(events.recv().await, Some(Err(Error::Closed)));
        assert_eq!(events.recv().await, None);
    }
}
