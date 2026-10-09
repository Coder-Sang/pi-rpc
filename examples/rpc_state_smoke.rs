//! No-prompt Pi 1.1.0 acceptance using existing Pi authentication.
//! Usage: PI_CODING_AGENT_DIR=/scratch/root/agent cargo run --example
//! rpc_state_smoke -- /path/pi /scratch/root
//! Prepare opaque private auth/models copies in root/agent if needed. Bash-only
//! sessions cannot persist in Pi 1.1.0, so export/clone/switch are native boundaries.
//! Pi itself owns configuration/authentication. This example never reads either.
use pi_rpc::{
    Config, Error, PiClient, PiProcess, ProcessOptions,
    protocol::{Event, KnownEvent, QueueMode, SessionTreeNode, ThinkingLevel},
};
use std::{path::Path, time::Duration};
use tokio::sync::mpsc;

const MODEL: &str = "claude-opus-4-6";
const MARKER: &str = "PI_RPC_STATE_OK";
type SmokeResult<T> = Result<T, Box<dyn std::error::Error>>;

fn pass(command: &str) {
    println!("PASS {command}");
}

fn tree_ids(nodes: &[SessionTreeNode], ids: &mut Vec<String>) {
    for node in nodes {
        ids.push(node.entry.id.clone());
        tree_ids(&node.children, ids);
    }
}

async fn update_containing(
    rx: &mut mpsc::UnboundedReceiver<String>,
    marker: &str,
) -> SmokeResult<()> {
    tokio::time::timeout(Duration::from_secs(10), async {
        let mut output = String::new();
        while let Some(delta) = rx.recv().await {
            output.push_str(&delta);
            if output.contains(marker) {
                return Ok(());
            }
        }
        Err("event consumer ended before bash marker".into())
    })
    .await?
}

async fn exercise(
    client: &PiClient,
    root: &Path,
    rx: &mut mpsc::UnboundedReceiver<String>,
) -> SmokeResult<()> {
    client.get_state().await?;
    pass("get_state");
    let models = client.get_available_models().await?;
    assert!(
        models
            .models
            .iter()
            .any(|m| m["provider"] == "anthropic" && m["id"] == MODEL)
    );
    pass("get_available_models (only availability checked, metadata not printed)");
    client.get_available_thinking_levels().await?;
    pass("get_available_thinking_levels");
    client.get_messages().await?;
    pass("get_messages");
    client.get_commands().await?;
    pass("get_commands");
    client.get_session_stats().await?;
    pass("get_session_stats");
    assert!(client.get_fork_messages().await?.messages.is_empty());
    pass("get_fork_messages (empty, no user entries)");
    assert!(client.get_last_assistant_text().await?.text.is_none());
    pass("get_last_assistant_text (none)");
    client.get_entries(None).await?;
    pass("get_entries");
    client.get_tree().await?;
    pass("get_tree");

    let model = client.set_model("anthropic", MODEL).await?;
    assert_eq!(model["id"], MODEL);
    assert_eq!(model["provider"], "anthropic");
    pass("set_model");
    let cycled = client.cycle_model().await?;
    if let Some(cycled) = cycled {
        assert_eq!(client.get_state().await?.model, Some(cycled.model));
        pass("cycle_model + state");
    } else {
        pass("cycle_model (null, no cycle available)");
    }
    client.set_model("anthropic", MODEL).await?;
    assert_eq!(client.get_state().await?.model.unwrap()["id"], MODEL);
    pass("set_model restores Opus");
    client.set_thinking_level(ThinkingLevel::Low).await?;
    assert_eq!(client.get_state().await?.thinking_level, ThinkingLevel::Low);
    pass("set_thinking_level + state");
    let level = client
        .cycle_thinking_level()
        .await?
        .ok_or("no thinking cycle")?;
    assert_eq!(client.get_state().await?.thinking_level, level.level);
    pass("cycle_thinking_level + state");
    client.set_thinking_level(ThinkingLevel::Off).await?;
    assert_eq!(client.get_state().await?.thinking_level, ThinkingLevel::Off);
    pass("set_thinking_level restores off");
    for mode in [QueueMode::All, QueueMode::OneAtATime] {
        client.set_steering_mode(mode).await?;
        assert_eq!(client.get_state().await?.steering_mode, mode);
        pass("set_steering_mode + state");
        client.set_follow_up_mode(mode).await?;
        assert_eq!(client.get_state().await?.follow_up_mode, mode);
        pass("set_follow_up_mode + state");
    }
    for enabled in [false, true, false] {
        client.set_auto_compaction(enabled).await?;
        assert_eq!(client.get_state().await?.auto_compaction_enabled, enabled);
        pass("set_auto_compaction + state");
        client.set_auto_retry(enabled).await?;
        pass("set_auto_retry (ack only: Pi 1.1.0 get_state omits autoRetryEnabled)");
    }
    client.abort_retry().await?;
    assert!(!client.get_state().await?.is_streaming);
    pass("abort_retry (idle only, no retry run exercised)");
    let queue = client.clear_queue().await?;
    assert!(queue.steering.is_empty() && queue.follow_up.is_empty());
    assert_eq!(client.get_state().await?.pending_message_count, 0);
    pass("clear_queue (empty)");
    client.abort().await?;
    assert!(!client.get_state().await?.is_streaming);
    pass("abort (idle only)");

    let bash = client
        .bash("printf 'PI_RPC_STATE_OK\\n'", Some(false))
        .await?;
    assert_eq!(bash.output.trim(), MARKER);
    assert_eq!(bash.exit_code, Some(0));
    assert!(!bash.cancelled);
    update_containing(rx, MARKER).await?;
    pass("bash printf: output, exit 0, bash_execution_update");
    let long_client = client.clone();
    let long = tokio::spawn(async move {
        long_client
            .bash("printf 'START\\n'; sleep 30", Some(false))
            .await
    });
    update_containing(rx, "START").await?;
    client.abort_bash().await?;
    let cancelled = long.await??;
    assert!(cancelled.cancelled);
    assert!(cancelled.output.contains("START"));
    pass("abort_bash after observed START update: cancelled=true");

    let export = root.join("export.html");
    match client
        .export_html(Some(export.to_str().ok_or("non-UTF8 root")?.into()))
        .await
    {
        Ok(exported) => {
            assert_eq!(Path::new(&exported.path), export);
            assert!(!std::fs::read(&export)?.is_empty());
            pass("export_html: scratch/export.html nonempty");
        }
        Err(Error::Rejected { command, message })
            if command == "export_html"
                && message == "Nothing to export yet - start a conversation first" =>
        {
            println!(
                "BOUNDARY export_html: Pi rejects bash-only session before conversation persistence."
            );
        }
        Err(error) => return Err(error.into()),
    }
    client.set_session_name("rpc-state-smoke").await?;
    let old = client.get_state().await?;
    assert_eq!(old.session_name.as_deref(), Some("rpc-state-smoke"));
    let history = client.get_messages().await?.messages;
    assert!(history.iter().any(|m| m.role == "bashExecution"));
    pass("set_session_name + state/history");
    let entries = client.get_entries(None).await?;
    assert!(!entries.entries.is_empty());
    for (index, entry) in entries.entries.iter().enumerate() {
        let filtered = client.get_entries(Some(entry.id.clone())).await?;
        assert_eq!(filtered.entries, entries.entries[index + 1..]);
        assert_eq!(filtered.leaf_id, entries.leaf_id);
    }
    pass("get_entries native entries + since exclusive suffix (every entry including leaf)");
    let tree = client.get_tree().await?;
    let mut ids = Vec::new();
    tree_ids(&tree.tree, &mut ids);
    ids.sort();
    let mut expected: Vec<_> = entries.entries.iter().map(|e| e.id.clone()).collect();
    expected.sort();
    assert_eq!(ids, expected);
    assert_eq!(tree.leaf_id, entries.leaf_id);
    pass("get_tree native entries match get_entries");
    match client.clone_session().await {
        Ok(cloned) => {
            assert!(!cloned.cancelled);
            assert_ne!(client.get_state().await?.session_id, old.session_id);
            assert_eq!(client.get_messages().await?.messages, history);
            pass("clone: cancelled=false, new id, identical history");
        }
        Err(Error::Rejected { command, message })
            if command == "clone"
                && message
                    == "This session has not been saved yet. Send a message before cloning or forking it." =>
        {
            println!("BOUNDARY clone: Pi refuses unsaved bash-only session.");
            assert_eq!(client.get_state().await?.session_id, old.session_id);
            assert_eq!(client.get_messages().await?.messages, history);
        }
        Err(error) => return Err(error.into()),
    }
    assert!(!client.new_session(None).await?.cancelled);
    let new = client.get_state().await?;
    assert_ne!(new.session_id, old.session_id);
    assert!(client.get_messages().await?.messages.is_empty());
    pass("new_session: new id, empty history");
    let bash = client
        .bash("printf 'NEW_SESSION_MARKER\\n'", Some(false))
        .await?;
    assert_eq!(bash.exit_code, Some(0));
    assert!(bash.output.contains("NEW_SESSION_MARKER"));
    update_containing(rx, "NEW_SESSION_MARKER").await?;
    pass("new-session bash marker: output, exit 0, bash_execution_update");
    let new = client.get_state().await?;
    let old_file = old
        .session_file
        .ok_or("old session file missing from state")?;
    let new_file = new
        .session_file
        .ok_or("new session file missing from state")?;
    assert!(Path::new(&old_file).starts_with(root.join("sessions")));
    assert!(Path::new(&new_file).starts_with(root.join("sessions")));
    if Path::new(&old_file).is_file() && Path::new(&new_file).is_file() {
        pass("bash marker session persistence");
        assert!(!client.switch_session(old_file).await?.cancelled);
        let restored = client.get_state().await?;
        assert_eq!(restored.session_name, old.session_name);
        assert_eq!(client.get_messages().await?.messages, history);
        pass("switch_session restores name/history");
    } else {
        println!(
            "BOUNDARY persistence/switch_session: Pi 1.1.0 only flushes user/assistant conversations; bash-only history remains in memory. No prompt sent."
        );
    }
    println!("SKIP fork: no user entry can be created without prompt/model turn.");
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> SmokeResult<()> {
    let mut args = std::env::args_os().skip(1);
    let executable = args
        .next()
        .ok_or("usage: rpc_state_smoke /path/pi /scratch/root")?
        .into();
    let root = std::path::PathBuf::from(args.next().ok_or("scratch root required")?);
    assert!(args.next().is_none(), "unexpected arguments");
    // Toggle commands persist settings in Pi. Require caller-side isolation before spawn.
    // Example: PI_CODING_AGENT_DIR=/scratch/root/agent cargo run ... /scratch/root
    let isolated_agent = std::env::var_os("PI_CODING_AGENT_DIR")
        .ok_or("set PI_CODING_AGENT_DIR to /scratch/root/agent to protect user settings")?;
    assert!(root.is_absolute(), "scratch root must be absolute");
    assert_eq!(std::path::PathBuf::from(isolated_agent), root.join("agent"));
    // Caller may prepare agent/auth.json by opaque copy. Never inspect credentials.
    // Fresh work/sessions directories and absent export protect existing outputs.
    std::fs::create_dir_all(root.join("agent"))?;
    assert!(!root.join("export.html").exists(), "export already exists");
    let root = root.canonicalize()?;
    std::fs::create_dir(root.join("work"))?;
    std::fs::create_dir(root.join("sessions"))?;
    let mut cli: Vec<std::ffi::OsString> = [
        "--no-extensions",
        "--no-mcp",
        "--no-skills",
        "--no-prompt-templates",
        "--no-themes",
        "--no-tools",
        "--provider",
        "anthropic",
        "--model",
        MODEL,
        "--thinking",
        "off",
        "--session-dir",
    ]
    .into_iter()
    .map(Into::into)
    .collect();
    cli.push(root.join("sessions").into_os_string());
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        executable,
        args: cli,
        cwd: Some(root.join("work")),
        connection: Config {
            command_timeout: Duration::from_secs(45),
            ..Default::default()
        },
        ..Default::default()
    })
    .await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = tokio::spawn(async move {
        let mut terminal = None;
        let mut unexpected_run = false;
        while let Some(record) = events.recv().await {
            match record {
                Ok(Event::Known(KnownEvent::BashExecutionUpdate { delta, .. })) => {
                    let _ = tx.send(delta);
                }
                Ok(Event::Known(KnownEvent::AgentStart)) => unexpected_run = true,
                Err(error) => terminal = Some(error),
                _ => {}
            }
        }
        (terminal, unexpected_run)
    });
    let outcome =
        tokio::time::timeout(Duration::from_secs(120), exercise(&client, &root, &mut rx)).await;
    if !matches!(outcome, Ok(Ok(()))) {
        process.terminate().await;
    }
    let reason = process.close().await;
    let (terminal, unexpected_run) = consumer.await?;
    outcome??;
    assert!(!unexpected_run, "no model turn allowed");
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(terminal, Some(reason));
    pass("event consumer drained through clean exit; no agent/model run");
    Ok(())
}
