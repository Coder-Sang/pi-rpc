//! Read-only RPC acceptance against a caller-supplied, isolated Pi executable.
use pi_rpc::{Error, PI_VERSION, PiProcess, ProcessOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let executable = std::env::args_os()
        .nth(1)
        .expect("usage: release_smoke /absolute/path/to/pi");
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        expected_version: pi_rpc::PI_VERSION.into(),
        executable: executable.into(),
        args: [
            "--no-session",
            "--no-extensions",
            "--no-mcp",
            "--no-skills",
            "--no-prompt-templates",
            "--no-themes",
            "--no-tools",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        ..Default::default()
    })
    .await?;
    let consumer = tokio::spawn(async move {
        let mut terminal = None;
        while let Some(record) = events.recv().await {
            if let Err(error) = record {
                assert!(terminal.replace(error).is_none());
            }
        }
        terminal
    });
    let state = client.get_state().await?;
    assert!(!state.is_streaming);
    assert_eq!(state.pending_message_count, 0);
    assert!(client.get_messages().await?.messages.is_empty());
    client.get_available_models().await?;
    client.get_available_thinking_levels().await?;
    client.get_session_stats().await?;
    client.get_commands().await?;
    client.get_entries(None).await?;
    client.get_tree().await?;
    assert!(client.get_last_assistant_text().await?.text.is_none());
    client.get_fork_messages().await?;
    let reason = process.close().await;
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(consumer.await.unwrap(), Some(reason));
    println!(
        "Pi {PI_VERSION}: 10 read-only RPCs passed; clean exit and terminal delivery verified"
    );
    Ok(())
}
