//! Session operations on a scratch-only Pi session, optionally paid --compact.
use pi_rpc::{Error, PiProcess, ProcessOptions, protocol::*};
use std::path::PathBuf;

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let executable: PathBuf = args
        .next()
        .expect("usage: rpc_session_smoke /path/pi /root /scratch/session.jsonl")
        .into();
    let root: PathBuf = args.next().expect("root required").into();
    let session: PathBuf = args.next().expect("session required").into();
    assert!(root.is_absolute() && session.is_absolute());
    assert!(
        session.starts_with(&root),
        "only accept scratch sessions within root"
    );
    assert_eq!(
        std::env::var_os("PI_CODING_AGENT_DIR").map(PathBuf::from),
        Some(root.join("agent")),
        "isolated agent required"
    );
    let cwd = root.join("session-work");
    std::fs::create_dir_all(&cwd).unwrap();
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        executable,
        cwd: Some(cwd),
        args: vec![
            "--session".into(),
            session.into_os_string(),
            "--session-dir".into(),
            root.join("session-branches").into_os_string(),
            "--no-extensions".into(),
            "--no-mcp".into(),
            "--no-skills".into(),
            "--no-prompt-templates".into(),
            "--no-themes".into(),
            "--no-tools".into(),
        ],
        ..Default::default()
    })
    .await?;
    let consumer = tokio::spawn(async move {
        let mut terminal = None;
        let mut compaction_completed = false;
        while let Some(record) = events.recv().await {
            match record {
                Ok(Event::Known(KnownEvent::CompactionEnd {
                    result: Some(_),
                    aborted: false,
                    error_message: None,
                    ..
                })) => compaction_completed = true,
                Err(e) => assert!(terminal.replace(e).is_none()),
                _ => {}
            }
        }
        (terminal, compaction_completed)
    });
    let compact_requested = args.next().as_deref() == Some(std::ffi::OsStr::new("--compact"));
    let mut exercise_task = tokio::spawn(async move {
        if compact_requested {
            client.set_model("anthropic", "claude-opus-4-6").await?;
            let result: CompactionResult = client
                .command_with_timeout(
                    Command::Compact {
                        custom_instructions: Some(
                            "Keep the RPC acceptance markers. Summarize in at most 100 words."
                                .into(),
                        ),
                    },
                    std::time::Duration::from_secs(180),
                )
                .await?
                .decode()?;
            assert!(!result.summary.trim().is_empty());
            assert!(result.tokens_before > 0.0);
            let entries = serde_json::to_value(client.get_entries(None).await?).unwrap();
            assert!(
                entries["entries"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|e| e["type"] == "compaction" && e["summary"] == result.summary)
            );
            assert!(!client.get_state().await?.is_compacting);
            println!(
                "PASS default real Opus compaction summary, persisted entry, tokensBefore={} summary_chars={} usage_cost={:?}",
                result.tokens_before,
                result.summary.len(),
                result.usage.map(|u| u.cost.total)
            );
        }
        let before = client.get_state().await?;
        let original_file = before.session_file.expect("saved session");
        let messages = client.get_messages().await?;
        assert!(
            !messages.messages.is_empty(),
            "existing conversation or compaction projection required"
        );
        client
            .command(Command::SetSessionName {
                name: "PI_SESSION_RPC_OK".into(),
            })
            .await?;
        assert_eq!(
            client.get_state().await?.session_name.as_deref(),
            Some("PI_SESSION_RPC_OK")
        );
        let output = root.join("acceptance-export.html");
        let exported: ExportPath = client
            .typed(Command::ExportHtml {
                output_path: Some(output.to_string_lossy().into()),
            })
            .await?;
        assert_eq!(PathBuf::from(exported.path), output);
        let html = std::fs::read_to_string(output).unwrap();
        assert!(html.len() > 1000 && html.to_lowercase().contains("<html"));
        println!(
            "PASS set_session_name and actual HTML export, bytes={}",
            html.len()
        );
        let cloned: SessionChange = client.typed(Command::Clone).await?;
        assert!(!cloned.cancelled);
        assert_ne!(client.get_state().await?.session_id, before.session_id);
        assert_eq!(client.get_messages().await?, messages);
        println!("PASS clone creates new session and preserves complete history");
        let switch: SessionChange = client
            .typed(Command::SwitchSession {
                session_path: original_file.clone(),
            })
            .await?;
        assert!(!switch.cancelled);
        assert_eq!(client.get_state().await?.session_id, before.session_id);
        assert_eq!(
            client.get_state().await?.session_name.as_deref(),
            Some("PI_SESSION_RPC_OK")
        );
        let forks = client.get_fork_messages().await?;
        let selected = forks.messages.first().expect("user entry");
        let fork = client.fork(&selected.entry_id).await?;
        assert!(!fork.cancelled);
        assert_eq!(fork.text, selected.text);
        assert_ne!(client.get_state().await?.session_id, before.session_id);
        assert!(
            !client
                .get_messages()
                .await?
                .messages
                .iter()
                .any(|m| m.role == "user")
        );
        println!(
            "PASS fork selected native user entry and returned text, branch excludes selected input"
        );
        let new: SessionChange = client
            .typed(Command::NewSession {
                parent_session: Some(original_file.clone()),
            })
            .await?;
        assert!(!new.cancelled);
        assert_ne!(client.get_state().await?.session_id, before.session_id);
        assert!(client.get_messages().await?.messages.is_empty());
        let switch: SessionChange = client
            .typed(Command::SwitchSession {
                session_path: original_file,
            })
            .await?;
        assert!(!switch.cancelled);
        assert_eq!(client.get_messages().await?, messages);
        let entries = client.get_entries(None).await?;
        assert!(!entries.entries.is_empty());
        let tree = client.get_tree().await?;
        assert!(!tree.tree.is_empty());
        println!(
            "PASS new_session empty/new identity and switch_session restores original identity/history/name, nonempty entries/tree"
        );
        Ok::<bool, Error>(compact_requested)
    });
    let outcome =
        tokio::time::timeout(std::time::Duration::from_secs(240), &mut exercise_task).await;
    if !matches!(&outcome, Ok(Ok(Ok(_)))) {
        exercise_task.abort();
        process.terminate().await;
    }
    let reason = process.close().await;
    let (terminal, compaction_completed) = consumer.await.unwrap();
    let compact_requested = outcome
        .map_err(|_| Error::Timeout)?
        .map_err(|_| Error::Protocol("session acceptance task failed".into()))??;
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(terminal, Some(reason));
    assert!(
        !compact_requested || compaction_completed,
        "successful real compaction_end required"
    );
    println!("PASS clean exit and unique terminal delivery; paid_compaction={compact_requested}");
    Ok(())
}
