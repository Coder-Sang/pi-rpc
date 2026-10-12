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
struct PreparedRecord {
    slot: mpsc::OwnedPermit<WriteMessage>,
    bytes: Vec<u8>,
    budget: OwnedSemaphorePermit,
}
impl PreparedRecord {
    // Caller holds the admission state lock and has checked closing/terminal.
    fn enqueue(self) -> oneshot::Receiver<Result<()>> {
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
    fn check_open(&self) -> Result<()> {
        let state = self.shared.state.lock().unwrap();
        if let Some(error) = &state.terminal {
            return Err(error.clone());
        }
        if state.closing {
            return Err(Error::Closed);
        }
        Ok(())
    }
    fn prepare_record<T: serde::Serialize>(&self, value: &T) -> Result<PreparedRecord> {
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
                let sent = prepared.enqueue();
                s.pending.insert(
                    id.clone(),
                    Pending {
                        command: name,
                        response,
                        _admission,
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
        let sent = self.send_one_way(|| answer.wire_record(request))?;
        drop(answer); // The queued output owns the reply bytes through flush.
        tokio::time::timeout(self.shared.config.command_timeout, async {
            sent.await.map_err(|_| self.shared.reason())?
        })
        .await
        .map_err(|_| Error::Timeout)?
    }
    fn send_one_way<T: serde::Serialize>(
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
    fn cancel_ui_if_open(&self, request: &UiRequest) -> Result<()> {
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
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<PromptDisposition>>(Command::Prompt {
            message,
            images: images.take(),
            streaming_behavior,
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn steer(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::Steer {
            message,
            images: images.take(),
        });
        async move { Ok(response.await?.disposition) }
    }
    pub fn follow_up(
        &self,
        message: impl Into<String>,
        images: Option<Vec<ImageContent>>,
    ) -> impl std::future::Future<Output = Result<QueuedInputDisposition>> + Send {
        let images = crate::owned::Owned::new(images);
        let message = message.into();
        let response = self.typed::<Disposition<QueuedInputDisposition>>(Command::FollowUp {
            message,
            images: images.take(),
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
    #[ignore = "manual Release encoding measurement; no timing assertions"]
    async fn image_encoding_measurement() {
        for count in [1, 4] {
            let command = Command::Prompt {
                message: "benchmark".into(),
                streaming_behavior: None,
                images: Some(
                    (0..count)
                        .map(|_| ImageContent {
                            r#type: ImageType::Image,
                            data: "A".repeat(9 * 1024 * 1024 / 2 - 4096),
                            mime_type: "image/png".into(),
                            extra: Default::default(),
                        })
                        .collect(),
                ),
            };
            let (client, _events, owner) = budget_fixture(Config::default());
            let mut single = Vec::new();
            let mut reserved = Vec::new();
            let mut peak = 0;
            for _ in 0..7 {
                let start = std::time::Instant::now();
                drop(codec::encode(&command, client.shared.config.max_record_bytes).unwrap());
                single.push(start.elapsed().as_micros());
                let start = std::time::Instant::now();
                let prepared = client.prepare_record(&command).unwrap();
                peak = client.shared.config.max_writer_bytes - client.budget.available_permits();
                drop(prepared);
                reserved.push(start.elapsed().as_micros());
            }
            single.sort_unstable();
            reserved.sort_unstable();
            println!(
                "images={count} single_median_us={} reserved_median_us={} reserved_peak_bytes={peak}",
                single[3], reserved[3]
            );
            drop(owner);
        }
    }

    fn budget_fixture(config: Config) -> (PiClient, PiEvents, PiConnection) {
        PiConnection::connect(
            Transport::new(tokio::io::empty(), tokio::io::sink(), crate::PI_VERSION).unwrap(),
            config,
        )
        .unwrap()
    }

    struct Counted<'a> {
        calls: &'a std::sync::atomic::AtomicUsize,
    }
    impl serde::Serialize for Counted<'_> {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            self.calls.fetch_add(1, Ordering::SeqCst);
            serializer.serialize_bool(true)
        }
    }

    #[tokio::test]
    async fn dropped_unsubmitted_and_submitted_futures_release_pending_slots() {
        let (local, _peer) = tokio::io::duplex(1);
        let (r, w) = tokio::io::split(local);
        let (client, _events, owner) = PiConnection::connect(
            Transport::new(r, w, crate::PI_VERSION).unwrap(),
            Config {
                max_pending_commands: 1,
                ..Config::default()
            },
        )
        .unwrap();
        drop(client.command(Command::GetState));
        assert_eq!(client.admission.available_permits(), 1);
        let mut command = Box::pin(client.command(Command::GetState));
        assert!(
            std::future::poll_fn(|cx| std::task::Poll::Ready(command.as_mut().poll(cx)))
                .await
                .is_pending()
        );
        assert_eq!(client.admission.available_permits(), 0);
        assert_eq!(client.shared.state.lock().unwrap().pending.len(), 1);
        drop(command);
        assert_eq!(client.admission.available_permits(), 1);
        assert!(client.shared.state.lock().unwrap().pending.is_empty());
        // The admitted write still owns its budget after the waiter is dropped.
        assert!(client.budget.available_permits() < client.shared.config.max_writer_bytes);
        drop(owner);
    }

    #[tokio::test]
    async fn queue_and_byte_rejection_precede_output_encoding() {
        use std::sync::atomic::AtomicUsize;
        let (client, _events, owner) = budget_fixture(Config {
            max_writer_records: 1,
            max_writer_bytes: 5,
            ..Config::default()
        });
        let held = client.prepare_record(&true).unwrap();
        let calls = AtomicUsize::new(0);
        assert!(matches!(
            client.prepare_record(&Counted { calls: &calls }),
            Err(Error::Capacity("writer records"))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 0);
        drop(held);
        assert_eq!(client.budget.available_permits(), 5);
        drop(owner);
        let (client, _events, owner) = budget_fixture(Config {
            max_writer_records: 2,
            max_writer_bytes: 5,
            ..Config::default()
        });
        let held = client.prepare_record(&true).unwrap();
        assert!(matches!(
            client.prepare_record(&Counted { calls: &calls }),
            Err(Error::Capacity("writer bytes"))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        drop(held);
        assert_eq!(client.budget.available_permits(), 5);
        assert_eq!(client.writer.capacity(), 2);
        drop(owner);
    }

    #[tokio::test]
    async fn write_and_flush_errors_return_reserved_bytes() {
        struct FailingWriter {
            fail_flush: bool,
        }
        impl tokio::io::AsyncWrite for FailingWriter {
            fn poll_write(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
                bytes: &[u8],
            ) -> std::task::Poll<std::io::Result<usize>> {
                std::task::Poll::Ready(if self.fail_flush {
                    Ok(bytes.len())
                } else {
                    Err(std::io::Error::other("write failed"))
                })
            }
            fn poll_flush(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Err(std::io::Error::other("flush failed")))
            }
            fn poll_shutdown(
                self: std::pin::Pin<&mut Self>,
                _: &mut std::task::Context<'_>,
            ) -> std::task::Poll<std::io::Result<()>> {
                std::task::Poll::Ready(Ok(()))
            }
        }
        for fail_flush in [false, true] {
            let (input, _peer) = tokio::io::duplex(64);
            let (client, _events, mut owner) = PiConnection::connect(
                Transport::new(input, FailingWriter { fail_flush }, crate::PI_VERSION).unwrap(),
                Config {
                    max_writer_bytes: 5,
                    ..Config::default()
                },
            )
            .unwrap();
            let prepared = client.prepare_record(&true).unwrap();
            assert_eq!(client.budget.available_permits(), 0);
            let sent = {
                let _state = client.shared.state.lock().unwrap();
                prepared.enqueue()
            };
            let error = sent.await.unwrap().unwrap_err();
            assert!(matches!(&error, Error::Transport(_)));
            assert_eq!(owner.wait_terminal().await, error);
            assert_eq!(client.budget.available_permits(), 5);
        }
    }

    #[tokio::test]
    async fn framed_output_budget_includes_lf() {
        let (client, _events, owner) = budget_fixture(Config {
            max_record_bytes: 4,
            max_writer_bytes: 4,
            ..Config::default()
        });
        assert!(matches!(
            client.prepare_record(&true),
            Err(Error::Capacity("writer bytes"))
        ));
        assert_eq!(client.budget.available_permits(), 4);
        drop(owner);
        let (client, _events, owner) = budget_fixture(Config {
            max_record_bytes: 4,
            max_writer_bytes: 5,
            ..Config::default()
        });
        let prepared = client.prepare_record(&true).unwrap();
        assert_eq!(prepared.bytes, b"true\n");
        assert_eq!(client.budget.available_permits(), 0);
        drop(prepared);
        assert_eq!(client.budget.available_permits(), 5);
        drop(owner);
    }

    #[tokio::test]
    async fn concurrent_encoding_holds_budget_before_serializer_runs() {
        use std::sync::{Barrier, atomic::AtomicUsize};
        struct Paused {
            calls: AtomicUsize,
            entered: Arc<Barrier>,
            release: Arc<Barrier>,
        }
        impl serde::Serialize for Paused {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                    self.entered.wait();
                    self.release.wait();
                }
                serializer.serialize_bool(true)
            }
        }
        let (client, _events, owner) = budget_fixture(Config {
            max_writer_bytes: 5,
            max_writer_records: 2,
            ..Config::default()
        });
        let entered = Arc::new(Barrier::new(2));
        let release = Arc::new(Barrier::new(2));
        let worker = {
            let client = client.clone();
            let entered = entered.clone();
            let release = release.clone();
            std::thread::spawn(move || {
                client
                    .prepare_record(&Paused {
                        calls: AtomicUsize::new(0),
                        entered,
                        release,
                    })
                    .unwrap()
            })
        };
        entered.wait();
        assert_eq!(client.budget.available_permits(), 0);
        let calls = AtomicUsize::new(0);
        assert!(matches!(
            client.prepare_record(&Counted { calls: &calls }),
            Err(Error::Capacity("writer bytes"))
        ));
        assert_eq!(calls.load(Ordering::SeqCst), 1);
        release.wait();
        drop(worker.join().unwrap());
        assert_eq!(client.budget.available_permits(), 5);
        assert_eq!(client.writer.capacity(), 2);
        drop(owner);
    }

    #[tokio::test]
    async fn second_pass_failure_panic_size_change_and_close_refund_admission() {
        use std::sync::atomic::AtomicUsize;
        struct Fault<'a> {
            calls: AtomicUsize,
            mode: usize,
            shared: &'a Shared,
        }
        impl serde::Serialize for Fault<'_> {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                    match self.mode {
                        0 => return Err(serde::ser::Error::custom("injected encoding failure")),
                        1 => panic!("injected encoding panic"),
                        2 => return serializer.serialize_bool(false),
                        3 => return serializer.serialize_u8(0),
                        _ => self.shared.state.try_lock().unwrap().closing = true,
                    }
                }
                serializer.serialize_bool(true)
            }
        }
        for mode in 0..5 {
            let (client, _events, owner) = budget_fixture(Config::default());
            let before = client.budget.available_permits();
            let slots = client.writer.capacity();
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                client.send_one_way(|| {
                    Ok(Fault {
                        calls: AtomicUsize::new(0),
                        mode,
                        shared: &client.shared,
                    })
                })
            }));
            if mode == 1 {
                assert!(result.is_err());
            } else {
                assert!(result.unwrap().is_err());
            }
            assert_eq!(client.budget.available_permits(), before);
            assert_eq!(client.writer.capacity(), slots);
            assert!(client.shared.state.lock().unwrap().pending.is_empty());
            drop(owner);
        }
    }

    #[tokio::test]
    async fn one_way_admission_skips_construction_after_close_or_terminal() {
        for terminal in [None, Some(Error::Transport("failed writer".into()))] {
            let (local, _peer) = tokio::io::duplex(1024);
            let (r, w) = tokio::io::split(local);
            let (client, _events, owner) = PiConnection::connect(
                Transport::new(r, w, crate::PI_VERSION).unwrap(),
                Config::default(),
            )
            .unwrap();
            let expected = terminal.clone().unwrap_or(Error::Closed);
            if let Some(error) = terminal {
                client.shared.finish(error);
            } else {
                client.shared.state.lock().unwrap().closing = true;
            }
            assert_eq!(
                client
                    .send_one_way(|| -> Result<Value> { panic!("record must not be constructed") })
                    .unwrap_err(),
                expected
            );
            drop(owner);
        }
    }

    #[tokio::test]
    async fn one_way_admission_rechecks_close_and_terminal_after_serialization() {
        struct Race<'a> {
            shared: &'a Shared,
            terminal: Option<Error>,
        }
        impl serde::Serialize for Race<'_> {
            fn serialize<S: serde::Serializer>(
                &self,
                serializer: S,
            ) -> std::result::Result<S::Ok, S::Error> {
                // This callback deterministically puts the state change between
                // preflight and enqueue, and asserts encoding releases the lock.
                let mut state = self
                    .shared
                    .state
                    .try_lock()
                    .expect("no lock during encoding");
                if let Some(error) = &self.terminal {
                    drop(state);
                    self.shared.finish(error.clone());
                } else {
                    state.closing = true;
                }
                serializer.serialize_unit()
            }
        }
        for terminal in [None, Some(Error::Transport("racing writer failure".into()))] {
            let (local, _peer) = tokio::io::duplex(1024);
            let (r, w) = tokio::io::split(local);
            let (client, _events, owner) = PiConnection::connect(
                Transport::new(r, w, crate::PI_VERSION).unwrap(),
                Config::default(),
            )
            .unwrap();
            let records = client.writer.capacity();
            let bytes = client.budget.available_permits();
            let expected = terminal.clone().unwrap_or(Error::Closed);
            assert_eq!(
                client
                    .send_one_way(|| Ok(Race {
                        shared: &client.shared,
                        terminal
                    }))
                    .unwrap_err(),
                expected
            );
            assert_eq!(client.writer.capacity(), records);
            assert_eq!(client.budget.available_permits(), bytes);
            assert!(client.shared.state.lock().unwrap().pending.is_empty());
            drop(owner);
        }
    }

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
