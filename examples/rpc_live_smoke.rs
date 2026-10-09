//! Opt-in paid acceptance of live queue/tool/abort/compaction/session workflows.
use pi_rpc::{Config, Error, PiClient, PiProcess, ProcessOptions, protocol::*};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::sync::mpsc;

async fn wait_for(
    events: &mut mpsc::UnboundedReceiver<pi_rpc::Result<Event>>,
    predicate: impl Fn(&KnownEvent) -> bool,
) -> pi_rpc::Result<KnownEvent> {
    tokio::time::timeout(Duration::from_secs(180), async {
        while let Some(record) = events.recv().await {
            if let Event::Known(event) = record? {
                if predicate(&event) {
                    return Ok(event);
                }
                if matches!(event, KnownEvent::AgentSettled { .. }) {
                    return Err(Error::Protocol(
                        "agent settled before the expected event".into(),
                    ));
                }
            }
        }
        Err(Error::Eof)
    })
    .await
    .map_err(|_| Error::Timeout)?
}

fn has_user(messages: &Messages, marker: &str) -> bool {
    messages.messages.iter().any(|m| {
        m.role == "user"
            && m.fields
                .get("content")
                .is_some_and(|v| v.to_string().contains(marker))
    })
}

async fn exercise(
    client: &PiClient,
    events: &mut mpsc::UnboundedReceiver<pi_rpc::Result<Event>>,
    cwd: &Path,
) -> pi_rpc::Result<()> {
    client.set_model("anthropic", "claude-opus-4-6").await?;
    client.set_thinking_level(ThinkingLevel::Off).await?;
    client
        .command(Command::SetAutoRetry { enabled: false })
        .await?;
    client
        .command(Command::SetAutoCompaction { enabled: false })
        .await?;
    let commands = client.get_commands().await?;
    assert!(!commands.commands.iter().any(|c| c.name == "unexpected"));
    println!("Starting paid Opus queue/tool workflow");
    assert_eq!(client.prompt("Call rpc_acceptance_wait exactly once now, wait for its result, then reply PI_INITIAL_OK. Do not call any other tool.").await?, PromptDisposition::Started);
    wait_for(events, |e| matches!(e, KnownEvent::ToolExecutionUpdate { tool_name, .. } if tool_name == "rpc_acceptance_wait")).await?;
    assert!(client.get_state().await?.is_streaming);
    assert_eq!(
        client.steer("PI_DROP_STEER", None).await?,
        QueuedInputDisposition::Queued
    );
    assert_eq!(
        client.follow_up("PI_DROP_FOLLOWUP", None).await?,
        QueuedInputDisposition::Queued
    );
    assert_eq!(
        client
            .prompt_with("PI_DROP_PROMPT", None, Some(StreamingBehavior::FollowUp))
            .await?,
        PromptDisposition::Queued
    );
    let removed = client.clear_queue().await?;
    assert_eq!(removed.steering, ["PI_DROP_STEER"]);
    assert_eq!(removed.follow_up, ["PI_DROP_FOLLOWUP", "PI_DROP_PROMPT"]);
    assert_eq!(client.get_state().await?.pending_message_count, 0);
    assert_eq!(client.steer("PI_STEER_OK: after the tool completes, reply exactly PI_STEER_OK without calling any more tools.", None).await?, QueuedInputDisposition::Queued);
    assert_eq!(
        client
            .follow_up(
                "PI_FOLLOWUP_OK: reply exactly PI_FOLLOWUP_OK, without tools.",
                None
            )
            .await?,
        QueuedInputDisposition::Queued
    );
    assert_eq!(client.get_state().await?.pending_message_count, 2);
    std::fs::write(cwd.join("rpc-tool-release"), "release").expect("release local tool");
    let tool = wait_for(events, |e| matches!(e, KnownEvent::ToolExecutionEnd { tool_name, .. } if tool_name == "rpc_acceptance_wait")).await?;
    if let KnownEvent::ToolExecutionEnd {
        is_error,
        duration_ms,
        result,
        ..
    } = tool
    {
        assert!(!is_error);
        assert!(duration_ms.is_some_and(|ms| ms >= 0.0));
        assert!(result.to_string().contains("PI_TOOL_OK"));
        println!(
            "PASS real tool start/update/end, result and durationMs={}",
            duration_ms.unwrap()
        );
    }
    assert!(matches!(
        wait_for(events, |e| matches!(e, KnownEvent::AgentSettled { .. })).await?,
        KnownEvent::AgentSettled { aborted: false }
    ));
    let messages = client.get_messages().await?;
    assert!(has_user(&messages, "PI_STEER_OK"));
    assert!(has_user(&messages, "PI_FOLLOWUP_OK"));
    for marker in ["PI_DROP_STEER", "PI_DROP_FOLLOWUP", "PI_DROP_PROMPT"] {
        assert!(!has_user(&messages, marker));
    }
    assert_eq!(
        client
            .get_last_assistant_text()
            .await?
            .text
            .as_deref()
            .map(str::trim),
        Some("PI_FOLLOWUP_OK")
    );
    assert_eq!(client.get_state().await?.pending_message_count, 0);
    println!(
        "PASS prompt started/queued, steer and follow_up consumed, clear_queue removes exact inputs"
    );

    // Fresh local tool gate makes interruption observable rather than racing an idle agent.
    std::fs::remove_file(cwd.join("rpc-tool-release")).expect("reset local gate");
    assert_eq!(
        client
            .prompt("Call rpc_acceptance_wait exactly once now. Do not call any other tool.")
            .await?,
        PromptDisposition::Started
    );
    wait_for(events, |e| matches!(e, KnownEvent::ToolExecutionUpdate { tool_name, .. } if tool_name == "rpc_acceptance_wait")).await?;
    assert!(client.get_state().await?.is_streaming);
    client.abort().await?;
    assert!(matches!(
        wait_for(events, |e| matches!(e, KnownEvent::AgentSettled { .. })).await?,
        KnownEvent::AgentSettled { aborted: true }
    ));
    assert!(!client.get_state().await?.is_streaming);
    println!("PASS active abort during real tool execution, agent_settled.aborted=true");

    let stats = client.get_session_stats().await?;
    println!(
        "Live generation reported tokens={} estimated_cost={}",
        stats.tokens.total, stats.cost
    );
    let original = client.get_state().await?;
    let original_file = original.session_file.expect("persistent original session");
    let entries_before = client.get_entries(None).await?.entries.len();
    let compact: CompactionResult = client.typed(Command::Compact { custom_instructions: Some("Summarize very briefly, retain the RPC acceptance markers, no more than 150 words.".into()) }).await?;
    assert!(!compact.summary.trim().is_empty());
    assert!(compact.tokens_before > 0.0);
    let entries = client.get_entries(None).await?;
    assert!(entries.entries.len() > entries_before);
    let raw = serde_json::to_value(&entries.entries).unwrap();
    assert!(
        raw.as_array()
            .unwrap()
            .iter()
            .any(|e| e["type"] == "compaction" && e["summary"] == compact.summary)
    );
    assert!(!client.get_state().await?.is_compacting);
    let compact_event = wait_for(events, |e| matches!(e, KnownEvent::CompactionEnd { .. })).await?;
    assert!(matches!(
        compact_event,
        KnownEvent::CompactionEnd {
            result: Some(_),
            aborted: false,
            error_message: None,
            ..
        }
    ));
    println!(
        "PASS manual compaction with real Opus summary, persisted entry, tokensBefore={} summary_chars={} usage_cost={:?}",
        compact.tokens_before,
        compact.summary.len(),
        compact.usage.map(|u| u.cost.total)
    );

    let fork_messages = client.get_fork_messages().await?;
    let selected = fork_messages.messages.first().expect("real user entry");
    let fork = client.fork(&selected.entry_id).await?;
    assert!(!fork.cancelled);
    assert_eq!(fork.text, selected.text);
    let forked = client.get_state().await?;
    assert_ne!(forked.session_id, original.session_id);
    assert_eq!(forked.pending_message_count, 0);
    assert!(
        !client
            .get_messages()
            .await?
            .messages
            .iter()
            .any(|m| m.role == "user")
    );
    let switched: SessionChange = client
        .typed(Command::SwitchSession {
            session_path: original_file,
        })
        .await?;
    assert!(!switched.cancelled);
    assert_eq!(client.get_state().await?.session_id, original.session_id);
    assert!(has_user(&client.get_messages().await?, "PI_FOLLOWUP_OK"));
    println!(
        "PASS fork selected real user entry and text, branch excludes selected input, switch restores original session"
    );
    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let executable: PathBuf = args
        .next()
        .expect("usage: rpc_live_smoke /path/pi /fresh/root")
        .into();
    let root: PathBuf = args.next().expect("fresh root required").into();
    assert!(root.is_absolute(), "root must be absolute");
    assert_eq!(
        std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from),
        Some(root.join("agent")),
        "set PI_CODING_AGENT_DIR to ROOT/agent with private auth/models copies"
    );
    let cwd = root.join("work");
    assert!(!cwd.exists(), "work directory must be fresh");
    std::fs::create_dir_all(&cwd).unwrap();
    // This dedicated agent directory is private to this acceptance run.
    std::fs::write(
        root.join("agent/settings.json"),
        r#"{"compaction":{"enabled":false,"keepRecentTokens":0},"retry":{"enabled":false}}"#,
    )
    .unwrap();
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/rpc_live_tool.ts");
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        executable,
        cwd: Some(cwd.clone()),
        args: vec![
            "--session-dir".into(),
            root.join("sessions").into_os_string(),
            "--no-extensions".into(),
            "--extension".into(),
            fixture.into_os_string(),
            "--no-mcp".into(),
            "--no-skills".into(),
            "--no-prompt-templates".into(),
            "--no-themes".into(),
            "--no-builtin-tools".into(),
            "--provider".into(),
            "anthropic".into(),
            "--model".into(),
            "claude-opus-4-6".into(),
            "--thinking".into(),
            "off".into(),
        ],
        connection: Config {
            command_timeout: Duration::from_secs(180),
            ..Default::default()
        },
        ..Default::default()
    })
    .await?;
    let (tx, mut rx) = mpsc::unbounded_channel();
    let consumer = tokio::spawn(async move {
        let mut terminal = None;
        while let Some(record) = events.recv().await {
            if let Err(error) = &record {
                assert!(terminal.replace(error.clone()).is_none());
            }
            let _ = tx.send(record);
        }
        terminal
    });
    let mut exercise_task = tokio::spawn(async move { exercise(&client, &mut rx, &cwd).await });
    let outcome = tokio::time::timeout(Duration::from_secs(600), &mut exercise_task).await;
    if !matches!(&outcome, Ok(Ok(Ok(())))) {
        exercise_task.abort();
        process.terminate().await;
    }
    let reason = process.close().await;
    let terminal = consumer.await.unwrap();
    outcome
        .map_err(|_| Error::Timeout)?
        .map_err(|_| Error::Protocol("live acceptance task failed".into()))??;
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(terminal, Some(reason));
    println!("PASS clean exit and exactly one terminal event");
    Ok(())
}
