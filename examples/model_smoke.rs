//! Opt-in paid model acceptance. Pi owns existing provider configuration/auth.
use pi_rpc::{
    Config, Error, PiProcess, ProcessOptions,
    protocol::{AssistantEvent, Command, Event, KnownEvent, PromptDisposition, ThinkingLevel},
};
use serde_json::Value;
use std::{path::PathBuf, time::Duration};

const MODEL: &str = "claude-opus-4-6";
const EXPECTED: &str = "PI_RPC_OPUS_OK";

#[derive(Default)]
struct Observation {
    started: bool,
    ended: bool,
    settled: bool,
    updates: usize,
    text: String,
    assistant: Option<pi_rpc::protocol::Message>,
    terminal: Option<Error>,
}

async fn exercise(
    client: &pi_rpc::PiClient,
    ready: tokio::sync::oneshot::Receiver<bool>,
    provider: Option<String>,
) -> pi_rpc::Result<bool> {
    let models = client.get_available_models().await?.models;
    let available: Vec<_> = models
        .iter()
        .filter(|model| model["id"] == MODEL)
        .filter_map(|model| model["provider"].as_str())
        .collect();
    println!("Available {MODEL} providers: {}", available.join(", "));
    if available.is_empty() {
        for model in &models {
            if model["id"].as_str().is_some_and(|id| id.contains("opus")) {
                println!(
                    "Available Opus model: {}/{}",
                    model["provider"].as_str().unwrap_or("?"),
                    model["id"].as_str().unwrap_or("?")
                );
            }
        }
    }
    let Some(provider) = provider else {
        return Ok(false);
    };
    if !available.contains(&provider.as_str()) {
        return Err(Error::Protocol(
            "requested model/provider is not available".into(),
        ));
    }
    let model = client.set_model(&provider, MODEL).await?;
    assert_eq!(model["id"], MODEL);
    assert_eq!(model["provider"], provider);
    client.set_thinking_level(ThinkingLevel::Off).await?;
    client
        .command(Command::SetAutoRetry { enabled: false })
        .await?;
    client
        .command(Command::SetAutoCompaction { enabled: false })
        .await?;
    let disposition = client
        .prompt(format!("Reply with exactly {EXPECTED}. No other text."))
        .await?;
    assert_eq!(disposition, PromptDisposition::Started);
    println!("Prompt accepted: started");
    let aborted = ready
        .await
        .map_err(|_| Error::Protocol("run ended before agent_settled".into()))?;
    assert!(!aborted, "model run was aborted");
    let state = client.get_state().await?;
    assert!(!state.is_streaming);
    assert_eq!(state.pending_message_count, 0);
    let current = state.model.expect("selected model");
    assert_eq!(current["id"], MODEL);
    assert_eq!(current["provider"], provider);
    assert_eq!(
        client
            .get_last_assistant_text()
            .await?
            .text
            .as_deref()
            .map(str::trim),
        Some(EXPECTED)
    );
    let messages = client.get_messages().await?.messages;
    assert!(messages.iter().any(|message| message.role == "user"));
    assert!(messages.iter().any(|message| message.role == "assistant"));
    let stats = client.get_session_stats().await?;
    println!(
        "Session tokens={} reported_cost={}",
        stats.tokens.total, stats.cost
    );
    Ok(true)
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    run(std::env::args_os().skip(1)).await
}

async fn run(mut args: impl Iterator<Item = std::ffi::OsString>) -> pi_rpc::Result<()> {
    let executable: PathBuf = args
        .next()
        .expect("usage: model_smoke /path/to/pi /isolated/work [provider] [exact-pi-version]")
        .into();
    let cwd: PathBuf = args
        .next()
        .expect("isolated work directory required")
        .into();
    let provider = args
        .next()
        .map(|value| value.into_string().expect("provider must be UTF-8"));
    let expected_version = args
        .next()
        .map(|value| value.into_string().expect("Pi version must be UTF-8"))
        .unwrap_or_else(|| pi_rpc::PI_VERSION.into());
    assert!(args.next().is_none(), "unexpected arguments");
    // Acceptance identifies one release, including when the SDK admits a range.
    pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), &expected_version)?;
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        expected_version: expected_version.clone(),
        executable,
        cwd: Some(cwd),
        args: [
            "--no-session",
            "--no-extensions",
            "--no-mcp",
            "--no-skills",
            "--no-prompt-templates",
            "--no-themes",
            "--no-tools",
            "--thinking",
            "off",
        ]
        .into_iter()
        .map(Into::into)
        .collect(),
        connection: Config {
            command_timeout: Duration::from_secs(30),
            ..Default::default()
        },
        ..Default::default()
    })
    .await?;
    let (ready, settled) = tokio::sync::oneshot::channel();
    let consumer = tokio::spawn(async move {
        let mut result = Observation::default();
        let mut ready = Some(ready);
        while let Some(record) = events.recv().await {
            match record {
                Ok(Event::Known(KnownEvent::AgentStart)) => result.started = true,
                Ok(Event::Known(KnownEvent::AgentEnd { will_retry, .. })) => {
                    assert!(!will_retry);
                    result.ended = true;
                }
                Ok(Event::Known(KnownEvent::AgentSettled { aborted })) => {
                    result.settled = true;
                    if let Some(ready) = ready.take() {
                        let _ = ready.send(aborted);
                    }
                }
                Ok(Event::Known(KnownEvent::MessageUpdate {
                    assistant_message_event,
                    ..
                })) => {
                    result.updates += 1;
                    if let AssistantEvent::TextDelta { delta, .. } = assistant_message_event {
                        result.text.push_str(&delta);
                    }
                }
                Ok(Event::Known(KnownEvent::MessageEnd { message }))
                    if message.role == "assistant" =>
                {
                    result.assistant = Some(message)
                }
                Ok(Event::Known(KnownEvent::ToolExecutionStart { .. })) => {
                    panic!("tools should be disabled")
                }
                Err(error) => {
                    assert!(result.terminal.replace(error).is_none());
                    // Failed connections must wake exercise, not wait for its deadline.
                    ready.take();
                }
                _ => {}
            }
        }
        result
    });
    let outcome = tokio::time::timeout(
        Duration::from_secs(180),
        exercise(&client, settled, provider),
    )
    .await;
    if !matches!(&outcome, Ok(Ok(_))) {
        process.terminate().await;
    }
    let reason = process.close().await;
    let observation = consumer.await.expect("event consumer");
    let paid = outcome.map_err(|_| Error::Timeout)??;
    assert_eq!(reason, Error::ProcessExit(Some(0)));
    assert_eq!(observation.terminal, Some(reason));
    if paid {
        assert!(observation.started && observation.ended && observation.settled);
        assert!(observation.updates > 0);
        assert_eq!(observation.text.trim(), EXPECTED);
        let message = observation.assistant.expect("assistant message_end");
        assert_eq!(message.fields["model"], MODEL);
        assert_eq!(message.fields["stopReason"], "stop");
        let duration = message.fields.get("durationMs").and_then(Value::as_f64);
        assert!(duration.is_some_and(|value| value >= 0.0));
        println!(
            "Pi {expected_version}: Reply={EXPECTED}; updates={}; durationMs={}; agent_settled.aborted=false; clean exit and terminal delivery PASS",
            observation.updates,
            duration.unwrap()
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn acceptance_requires_a_concrete_version_before_probing() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("nonexistent-pi").into_os_string();
        for version in [
            pi_rpc::PI_VERSION_REQUIREMENT,
            ">=1.1.0, <=1.2.0",
            "1.0.9",
            "1.3.1",
            "1.01.0",
            "1.3.0 ",
            "v1.3.0",
            "1.3.0-beta",
        ] {
            let result = run([
                executable.clone(),
                root.path().as_os_str().to_owned(),
                "anthropic".into(),
                version.into(),
            ]
            .into_iter())
            .await;
            assert_eq!(result, Err(Error::Version), "{version}");
        }
    }

    #[tokio::test]
    async fn concrete_and_default_versions_reach_the_probe() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("nonexistent-pi").into_os_string();
        for version in [
            None,
            Some("1.1.0"),
            Some("1.1.10"),
            Some("1.2.0"),
            Some("1.3.0"),
        ] {
            let mut args = vec![
                executable.clone(),
                root.path().as_os_str().to_owned(),
                "anthropic".into(),
            ];
            args.extend(version.map(std::ffi::OsString::from));
            let result = run(args.into_iter()).await;
            assert!(
                matches!(result, Err(Error::Transport(_))),
                "{version:?}: {result:?}"
            );
        }
    }
}
