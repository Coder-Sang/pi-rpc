//! Real Pi extension UI acceptance, with no model inference for UI commands.
//! Run with PI_CODING_AGENT_DIR and PI_CODING_AGENT_SESSION_DIR inside isolated cwd.
use pi_rpc::{
    Config, Error, PiClient, PiEvents, PiProcess, ProcessOptions, UiPolicy,
    protocol::{
        CompactionReason, EntryData, Event, KnownEvent, PromptDisposition, UiAnswer, UiMethod,
    },
};
use std::{
    collections::BTreeSet,
    path::PathBuf,
    time::{Duration, Instant},
};

fn check(condition: bool, message: &str) -> pi_rpc::Result<()> {
    if condition {
        Ok(())
    } else {
        Err(Error::Protocol(message.into()))
    }
}

async fn next(events: &mut PiEvents) -> pi_rpc::Result<Event> {
    events.recv().await.ok_or(Error::Closed)?
}

async fn ui_events(
    client: &PiClient,
    events: &mut PiEvents,
    policy: UiPolicy,
) -> pi_rpc::Result<()> {
    let mut methods = BTreeSet::new();
    let mut answers = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mode = if policy == UiPolicy::Manual {
        "manual"
    } else {
        "cancel"
    };
    loop {
        let event = next(events).await?;
        let Event::Ui(request) = event else {
            if matches!(
                event,
                Event::Known(KnownEvent::AgentStart | KnownEvent::ExtensionError { .. })
            ) {
                return Err(Error::Protocol(
                    "UI command triggered agent or extension error".into(),
                ));
            }
            continue;
        };
        check(
            !request.id.is_empty() && ids.insert(request.id.clone()),
            "UI request ID missing or duplicated",
        )?;
        let (name, answer) = match &request.method {
            UiMethod::Select { title, options, .. } => {
                check(
                    title == "PI_RPC_UI_SELECT" && options == &["first", "second"],
                    "select payload",
                )?;
                ("select", Some(UiAnswer::Value("second".into())))
            }
            UiMethod::Confirm { title, message, .. } => {
                check(
                    title == "PI_RPC_UI_CONFIRM" && message == "Confirm fixture only",
                    "confirm payload",
                )?;
                ("confirm", Some(UiAnswer::Confirmed(true)))
            }
            UiMethod::Input {
                title, placeholder, ..
            } => {
                check(
                    title == "PI_RPC_UI_INPUT"
                        && placeholder.as_deref() == Some("fixture placeholder"),
                    "input payload",
                )?;
                ("input", Some(UiAnswer::Value("PI_RPC_INPUT_ANSWER".into())))
            }
            UiMethod::Editor { title, prefill } => {
                check(
                    title == "PI_RPC_UI_EDITOR"
                        && prefill.as_deref() == Some("fixture prefill\nsecond line"),
                    "editor payload",
                )?;
                (
                    "editor",
                    Some(UiAnswer::Value("PI_RPC_EDITOR_ANSWER\nsecond line".into())),
                )
            }
            UiMethod::Notify {
                message,
                notify_type,
            } => {
                check(notify_type.as_deref() == Some("info"), "notify type")?;
                if message.starts_with("PI_RPC_UI_ANSWER_") {
                    answers.insert(message.clone());
                }
                if message == &format!("PI_RPC_UI_DONE:{mode}") {
                    break;
                }
                ("notify", None)
            }
            UiMethod::SetStatus {
                status_key,
                status_text,
            } => {
                check(
                    status_key == "rpc-acceptance"
                        && status_text.as_deref() == Some("PI_RPC_UI_STATUS"),
                    "status payload",
                )?;
                ("setStatus", None)
            }
            UiMethod::SetWidget {
                widget_key,
                widget_lines,
                widget_placement,
            } => {
                check(
                    widget_key == "rpc-acceptance"
                        && widget_lines
                            .as_ref()
                            .is_some_and(|v| v == &["PI_RPC_UI_WIDGET", "second line"])
                        && widget_placement.as_deref() == Some("aboveEditor"),
                    "widget payload",
                )?;
                ("setWidget", None)
            }
            UiMethod::SetTitle { title } => {
                check(title == "PI_RPC_UI_TITLE", "title payload")?;
                ("setTitle", None)
            }
            UiMethod::SetEditorText { text } => {
                check(text == "PI_RPC_UI_EDITOR_TEXT", "editor text payload")?;
                ("set_editor_text", None)
            }
        };
        check(
            methods.insert(name) || name == "notify",
            "duplicate UI method",
        )?;
        if policy == UiPolicy::Manual {
            if let Some(answer) = answer {
                client.respond_ui(&request, answer).await?;
            }
        }
    }
    check(methods.len() == 9, "missing UI method")?;
    let expected = if policy == UiPolicy::Manual {
        [
            "PI_RPC_UI_ANSWER_SELECT:\"second\"",
            "PI_RPC_UI_ANSWER_CONFIRM:true",
            "PI_RPC_UI_ANSWER_INPUT:\"PI_RPC_INPUT_ANSWER\"",
            "PI_RPC_UI_ANSWER_EDITOR:\"PI_RPC_EDITOR_ANSWER\\nsecond line\"",
        ]
    } else {
        [
            "PI_RPC_UI_ANSWER_SELECT:null",
            "PI_RPC_UI_ANSWER_CONFIRM:false",
            "PI_RPC_UI_ANSWER_INPUT:null",
            "PI_RPC_UI_ANSWER_EDITOR:null",
        ]
    };
    check(
        answers == expected.into_iter().map(str::to_owned).collect(),
        "extension answer markers differ",
    )?;
    Ok(())
}

async fn ui(client: &PiClient, events: &mut PiEvents, policy: UiPolicy) -> pi_rpc::Result<()> {
    check(
        client
            .get_commands()
            .await?
            .commands
            .iter()
            .any(|c| c.name == "rpc-acceptance-ui"),
        "explicit extension command missing",
    )?;
    let mode = if policy == UiPolicy::Manual {
        "manual"
    } else {
        "cancel"
    };
    let (prompt, observed) = tokio::join!(
        client.prompt(format!("/rpc-acceptance-ui {mode}")),
        ui_events(client, events, policy)
    );
    check(
        prompt? == PromptDisposition::Handled,
        "slash command not handled",
    )?;
    observed?;
    check(
        client.get_messages().await?.messages.is_empty(),
        "UI-only command made model messages",
    )?;
    check(
        !client.get_state().await?.is_streaming,
        "UI run still streaming",
    )?;
    println!("UI {mode}: all 9 method payloads and extension-received answer markers PASS");
    Ok(())
}

async fn retry_case(
    client: &PiClient,
    events: &mut PiEvents,
    scenario: &str,
) -> pi_rpc::Result<()> {
    check(
        client
            .prompt(format!("/rpc-acceptance-retry {scenario}"))
            .await?
            == PromptDisposition::Handled,
        "retry fixture not armed",
    )?;
    let enabled = scenario != "disabled";
    // Pi owns persistence. The mandatory isolated config directory protects user settings.
    client.set_auto_retry(enabled).await?;
    check(
        client
            .prompt("Synthetic upstream fault acceptance only")
            .await?
            == PromptDisposition::Started,
        "synthetic prompt not started",
    )?;
    let mut agent_starts = 0;
    let mut agent_ends = 0;
    let mut errors = Vec::new();
    let mut retries = 0;
    let mut retry_end = None;
    let mut abort_at = None;
    let mut delay = Duration::ZERO;
    let mut successful = false;
    loop {
        match next(events).await? {
            Event::Known(KnownEvent::AgentStart) => agent_starts += 1,
            Event::Known(KnownEvent::AgentEnd { .. }) => agent_ends += 1,
            Event::Known(KnownEvent::MessageEnd { message }) if message.role == "assistant" => {
                check(
                    message.fields["provider"] == "rpc-acceptance-synthetic",
                    "unexpected upstream provider",
                )?;
                if let Some(error) = message
                    .fields
                    .get("errorMessage")
                    .and_then(serde_json::Value::as_str)
                {
                    errors.push(error.to_owned());
                } else {
                    successful =
                        message.fields["content"][0]["text"] == "PI_RPC_SYNTHETIC_RECOVERED:3";
                }
            }
            Event::Known(KnownEvent::AutoRetryStart {
                attempt,
                max_attempts,
                delay_ms,
                error_message,
            }) => {
                retries += 1;
                check(
                    enabled && attempt == retries && max_attempts >= 2 && delay_ms > 0,
                    "retry start metadata",
                )?;
                check(
                    error_message.contains(&format!("PI_RPC_SYNTHETIC_FAULT:{scenario}:{attempt}")),
                    "retry not caused by fixture fault",
                )?;
                if scenario == "abort" {
                    check(
                        retries == 1 && errors.len() == 1 && agent_starts == 1,
                        "abort must target first active retry",
                    )?;
                    check(
                        client.get_state().await?.is_streaming,
                        "abort_retry was idle, not active",
                    )?;
                    delay = Duration::from_millis(delay_ms);
                    abort_at = Some(Instant::now());
                    client.abort_retry().await?;
                }
            }
            Event::Known(KnownEvent::AutoRetryEnd {
                success,
                attempt,
                final_error,
            }) => {
                check(
                    retry_end.replace((success, attempt, final_error)).is_none(),
                    "duplicate retry end",
                )?;
            }
            Event::Known(KnownEvent::AgentSettled { aborted }) => {
                // abort_retry cancels the backoff, not the whole agent run.
                check(!aborted, "abort_retry unexpectedly marked agent aborted")?;
                break;
            }
            Event::Known(KnownEvent::ExtensionError { .. }) => {
                return Err(Error::Protocol("retry fixture error".into()));
            }
            _ => {}
        }
    }
    check(
        agent_starts > 0 && agent_starts == agent_ends,
        "agent start/end lifecycle",
    )?;
    check(
        errors.first().is_some_and(|e| e.starts_with("429 ")),
        "synthetic 429 missing",
    )?;
    match scenario {
        "recover" => {
            check(
                retries == 2 && errors.len() == 2 && errors[1].starts_with("503 ") && successful,
                "429/503 recovery observations",
            )?;
            check(
                retry_end == Some((true, 2, None)),
                "successful retry end metadata",
            )?;
        }
        "abort" => {
            check(
                retries == 1 && errors.len() == 1 && !successful,
                "abort allowed extra provider attempt",
            )?;
            check(
                retry_end == Some((false, 1, Some("Retry cancelled".into()))),
                "cancelled retry end metadata",
            )?;
            check(
                abort_at.is_some_and(|t| t.elapsed() < delay),
                "abort did not settle before original retry delay",
            )?;
        }
        "disabled" => check(
            retries == 0 && retry_end.is_none() && errors.len() == 1 && !successful,
            "disabled auto retry still retried",
        )?,
        _ => return Err(Error::Protocol("unknown retry scenario".into())),
    }
    let state = client.get_state().await?;
    check(
        !state.is_streaming && state.pending_message_count == 0,
        "retry agent not settled",
    )?;
    println!("Retry {scenario}: real Pi + synthetic upstream fault injection PASS");
    Ok(())
}

async fn retry(client: &PiClient, events: &mut PiEvents) -> pi_rpc::Result<()> {
    let model = client
        .set_model("rpc-acceptance-synthetic", "retry-fixture")
        .await?;
    check(
        model["provider"] == "rpc-acceptance-synthetic",
        "synthetic model selection failed",
    )?;
    for scenario in ["recover", "abort", "disabled"] {
        retry_case(client, events, scenario).await?;
    }
    auto_compaction(client, events).await?;
    Ok(())
}

async fn auto_compaction(client: &PiClient, events: &mut PiEvents) -> pi_rpc::Result<()> {
    client
        .set_model("rpc-acceptance-synthetic", "compact-fixture")
        .await?;
    client.set_auto_compaction(true).await?;
    check(
        client.prompt("/rpc-acceptance-retry compact").await? == PromptDisposition::Handled,
        "compaction fixture not armed",
    )?;
    check(
        client
            .prompt("Synthetic usage threshold compaction only")
            .await?
            == PromptDisposition::Started,
        "compaction prompt not started",
    )?;
    let mut started = false;
    let mut ended = false;
    let mut hook = false;
    loop {
        match next(events).await? {
            Event::Known(KnownEvent::CompactionStart { reason }) => {
                check(
                    !started && reason == CompactionReason::Threshold,
                    "auto compaction start reason",
                )?;
                started = true;
            }
            Event::Ui(request) => {
                if let UiMethod::Notify { message, .. } = request.method {
                    hook |= message == "PI_RPC_SYNTHETIC_COMPACTION_HOOK";
                }
            }
            Event::Known(KnownEvent::CompactionEnd {
                reason,
                result,
                aborted,
                will_retry,
                error_message,
            }) => {
                check(
                    started
                        && !ended
                        && reason == CompactionReason::Threshold
                        && !aborted
                        && !will_retry
                        && error_message.is_none(),
                    "auto compaction failed",
                )?;
                let result = result
                    .ok_or_else(|| Error::Protocol("auto compaction result missing".into()))?;
                check(
                    result.summary == "PI_RPC_EXTENSION_PROVIDED_SUMMARY"
                        && result
                            .details
                            .as_ref()
                            .is_some_and(|v| v["summarySource"] == "extension-fixture")
                        && result.tokens_before >= 35000.0
                        && !result.first_kept_entry_id.is_empty(),
                    "extension provided compaction result",
                )?;
                ended = true;
            }
            Event::Known(KnownEvent::AgentSettled { aborted }) => {
                check(
                    !aborted && started && ended && hook,
                    "auto compaction did not settle successfully",
                )?;
                break;
            }
            Event::Known(KnownEvent::ExtensionError { .. }) => {
                return Err(Error::Protocol("compaction fixture error".into()));
            }
            _ => {}
        }
    }
    check(
        !client.get_state().await?.is_streaming,
        "auto compaction still streaming",
    )?;
    let entries = client.get_entries(None).await?;
    check(entries.entries.iter().any(|entry| matches!(&entry.data,
        EntryData::Compaction { summary, extra, .. }
        if summary == "PI_RPC_EXTENSION_PROVIDED_SUMMARY" && extra.get("fromHook") == Some(&serde_json::Value::Bool(true))
    )), "native extension compaction entry missing")?;
    println!(
        "Auto compaction threshold: real Pi + synthetic usage, extension-provided summary (not Opus summarizer) PASS"
    );
    Ok(())
}

async fn run(options: ProcessOptions, policy: Option<UiPolicy>) -> pi_rpc::Result<()> {
    let (client, mut events, mut process) = PiProcess::spawn(options).await?;
    let outcome = tokio::time::timeout(Duration::from_secs(25), async {
        match policy {
            Some(policy) => ui(&client, &mut events, policy).await,
            None => retry(&client, &mut events).await,
        }
    })
    .await;
    if !matches!(outcome, Ok(Ok(()))) {
        process.terminate().await;
    }
    let reason = process.close().await;
    let mut terminal = None;
    while let Some(record) = events.recv().await {
        if let Err(error) = record {
            check(
                terminal.replace(error).is_none(),
                "duplicate terminal event",
            )?;
        }
    }
    outcome.map_err(|_| Error::Timeout)??;
    check(
        reason == Error::ProcessExit(Some(0)) && terminal == Some(reason),
        "unclean Pi exit or terminal delivery",
    )
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let executable: PathBuf = args
        .next()
        .expect("usage: rpc_extension_smoke /path/to/pi /isolated/work")
        .into();
    let cwd: PathBuf = args.next().expect("isolated cwd required").into();
    for key in ["PI_CODING_AGENT_DIR", "PI_CODING_AGENT_SESSION_DIR"] {
        let path = PathBuf::from(std::env::var_os(key).ok_or_else(|| {
            Error::Protocol(format!("set {key} to an isolated directory beneath cwd"))
        })?);
        check(
            path.is_absolute() && path.starts_with(&cwd) && path != cwd,
            "Pi config/session isolation required",
        )?;
    }
    for policy in [Some(UiPolicy::Manual), Some(UiPolicy::Cancel), None] {
        let mut options = ProcessOptions {
            executable: executable.clone(),
            cwd: Some(cwd.clone()),
            args: [
                "--offline",
                "--no-session",
                "--no-extensions",
                "--no-mcp",
                "--no-skills",
                "--no-prompt-templates",
                "--no-themes",
                "--no-tools",
                "--no-context-files",
            ]
            .into_iter()
            .map(Into::into)
            .collect(),
            connection: Config {
                command_timeout: Duration::from_secs(10),
                ui_policy: policy.unwrap_or(UiPolicy::Cancel),
                ..Default::default()
            },
            ..Default::default()
        };
        options.args.extend([
            "--extension".into(),
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join(if policy.is_some() {
                    "tests/fixtures/rpc_acceptance_extension.ts"
                } else {
                    "tests/fixtures/rpc_retry_extension.ts"
                })
                .into_os_string(),
        ]);
        run(options, policy).await?;
    }
    println!(
        "Pi 1.1.0 real extension UI and synthetic retry acceptance: no paid request; clean exits PASS"
    );
    Ok(())
}
