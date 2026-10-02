//! Local non-PTY subprocess lifecycle. No config files or credentials are read
//! by this crate. The executable inherits its environment and controls Pi setup.
use crate::{Config, Error, PI_VERSION, PiClient, PiConnection, PiEvents, Result, Transport};
use std::{
    collections::VecDeque,
    ffi::OsString,
    path::PathBuf,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::AsyncReadExt,
    process::{Child, Command},
    sync::{oneshot, watch},
    task::JoinHandle,
};

#[derive(Debug, Clone)]
pub struct ProcessOptions {
    pub executable: PathBuf,
    /// Extra CLI arguments. Do not put secrets here.
    pub args: Vec<OsString>,
    pub cwd: Option<PathBuf>,
    pub expected_version: String,
    pub startup_timeout: Duration,
    pub stderr_bytes: usize,
    pub connection: Config,
}
impl Default for ProcessOptions {
    fn default() -> Self {
        Self {
            executable: "pi".into(),
            args: Vec::new(),
            cwd: None,
            expected_version: PI_VERSION.into(),
            startup_timeout: Duration::from_secs(5),
            stderr_bytes: 64 * 1024,
            connection: Config::default(),
        }
    }
}
#[derive(Clone)]
pub struct Diagnostics {
    bytes: Arc<Mutex<VecDeque<u8>>>,
}
impl Diagnostics {
    /// Explicit opt-in diagnostic access. Stderr may contain sensitive data.
    /// The SDK never prints/logs this ring automatically.
    pub fn snapshot(&self) -> Vec<u8> {
        self.bytes.lock().unwrap().iter().copied().collect()
    }
}
/// Owns the local child and connection. Cloned PiClient handles never own/kill it.
/// Drop signals termination and cleanup. Explicit terminate also awaits reaping.
pub struct PiProcess {
    connection: PiConnection,
    diagnostics: Diagnostics,
    kill: Option<oneshot::Sender<()>>,
    supervisor: Option<JoinHandle<()>>,
    exit: watch::Receiver<Option<Error>>,
}
fn base_command(options: &ProcessOptions) -> Command {
    let mut c = Command::new(&options.executable);
    if let Some(cwd) = &options.cwd {
        c.current_dir(cwd);
    }
    c.kill_on_drop(true);
    c
}
async fn bounded_version_read<R: tokio::io::AsyncRead + Unpin>(reader: R) -> Result<Vec<u8>> {
    let mut bytes = Vec::new();
    reader
        .take(4097)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| Error::Transport(e.to_string()))?;
    if bytes.len() > 4096 {
        return Err(Error::Protocol(
            "version probe output limit exceeded".into(),
        ));
    }
    Ok(bytes)
}
async fn verify(options: &ProcessOptions) -> Result<()> {
    if options.expected_version != PI_VERSION {
        return Err(Error::Version);
    }
    let mut child = base_command(options)
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| Error::Transport(e.to_string()))?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let result = tokio::time::timeout(options.startup_timeout, async {
        let (out, err, status) = tokio::join!(
            bounded_version_read(stdout),
            bounded_version_read(stderr),
            child.wait()
        );
        let out = out?;
        err?;
        let status = status.map_err(|e| Error::Transport(e.to_string()))?;
        if !status.success() {
            return Err(Error::ProcessExit(status.code()));
        }
        if std::str::from_utf8(&out).ok().map(str::trim) != Some(options.expected_version.as_str())
        {
            return Err(Error::Version);
        }
        Ok(())
    })
    .await;
    match result {
        Ok(r) => {
            if r.is_err() {
                let _ = child.kill().await;
            }
            r
        }
        Err(_) => {
            let _ = child.kill().await;
            Err(Error::Transport("version probe deadline elapsed".into()))
        }
    }
}
impl PiProcess {
    /// Runs only `--version` to verify 0.99.2 before launching `--mode rpc`.
    pub async fn spawn(options: ProcessOptions) -> Result<(PiClient, PiEvents, Self)> {
        verify(&options).await?;
        let mut child = base_command(&options)
            .args(&options.args)
            .args(["--mode", "rpc"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .map_err(|e| Error::Transport(e.to_string()))?;
        let reader = child.stdout.take().unwrap();
        let writer = child.stdin.take().unwrap();
        let mut stderr = child.stderr.take().unwrap();
        let (exit_tx, exit) = watch::channel(None);
        let mut transport = Transport::new(reader, writer, &options.expected_version)?;
        transport.exit = Some(exit.clone());
        let (client, events, connection) = PiConnection::connect(transport, options.connection)?;
        let diagnostics = Diagnostics {
            bytes: Arc::new(Mutex::new(VecDeque::new())),
        };
        let ring = diagnostics.clone();
        let cap = options.stderr_bytes;
        let (kill, kill_rx) = oneshot::channel();
        let mut terminal = connection.terminal_receiver();
        let supervisor = tokio::spawn(async move {
            let drain = tokio::spawn(async move {
                let mut buf = [0u8; 4096];
                while let Ok(n) = stderr.read(&mut buf).await {
                    if n == 0 {
                        break;
                    }
                    let mut bytes = ring.bytes.lock().unwrap();
                    for b in &buf[..n] {
                        if cap > 0 {
                            if bytes.len() == cap {
                                bytes.pop_front();
                            }
                            bytes.push_back(*b);
                        }
                    }
                }
            });
            let reason = tokio::select! {
                status=child.wait()=>status.map(|s|Error::ProcessExit(s.code())).unwrap_or_else(|e|Error::Transport(e.to_string())),
                _=kill_rx=>kill_and_reap(&mut child).await,
                _=terminal.changed()=>kill_and_reap(&mut child).await,
            };
            exit_tx.send_replace(Some(reason));
            // Descendants may keep stderr open. Do not let them leak a drain task.
            let mut drain = drain;
            if tokio::time::timeout(Duration::from_millis(100), &mut drain)
                .await
                .is_err()
            {
                drain.abort();
                let _ = drain.await;
            }
        });
        Ok((
            client,
            events,
            Self {
                connection,
                diagnostics,
                kill: Some(kill),
                supervisor: Some(supervisor),
                exit,
            },
        ))
    }
    pub fn diagnostics(&self) -> Diagnostics {
        self.diagnostics.clone()
    }
    pub fn terminal_reason(&self) -> Option<Error> {
        self.connection.terminal_reason()
    }
    pub async fn wait_terminal(&mut self) -> Error {
        self.connection.wait_terminal().await
    }
    /// Orderly stdin EOF, then wait for child cleanup. Does not send abort.
    pub async fn close(&mut self) -> Error {
        let r = self.connection.close().await;
        if let Some(t) = self.supervisor.take() {
            let _ = t.await;
        }
        r
    }
    /// Explicit forced termination, distinct from abort and queue clearing.
    pub async fn terminate(&mut self) -> Error {
        self.connection.fail(Error::Terminated);
        if let Some(k) = self.kill.take() {
            let _ = k.send(());
        }
        if let Some(t) = self.supervisor.take() {
            let _ = t.await;
        }
        self.exit.borrow().clone().unwrap_or(Error::Terminated)
    }
    pub fn process_exit(&self) -> Option<Error> {
        self.exit.borrow().clone()
    }
}
async fn kill_and_reap(child: &mut Child) -> Error {
    let _ = child.start_kill();
    child
        .wait()
        .await
        .map(|s| Error::ProcessExit(s.code()))
        .unwrap_or_else(|e| Error::Transport(e.to_string()))
}
impl Drop for PiProcess {
    fn drop(&mut self) {
        if let Some(k) = self.kill.take() {
            let _ = k.send(());
        } /* supervisor remains alive to reap the child */
    }
}
