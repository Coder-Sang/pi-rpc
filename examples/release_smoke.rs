//! Read-only RPC acceptance against a caller-supplied, isolated Pi executable.
use pi_rpc::{Error, PI_VERSION, PiProcess, ProcessOptions};

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    run(std::env::args_os().skip(1)).await
}

async fn run(mut args: impl Iterator<Item = std::ffi::OsString>) -> pi_rpc::Result<()> {
    let executable = args
        .next()
        .expect("usage: release_smoke /absolute/path/to/pi [exact-pi-version]");
    let expected_version = args
        .next()
        .map(|value| value.into_string().expect("Pi version must be UTF-8"))
        .unwrap_or_else(|| PI_VERSION.into());
    assert!(args.next().is_none(), "unexpected arguments");
    // Acceptance must identify a concrete release, not merely admit a range.
    // Transport validates a concrete supported version without performing I/O.
    pi_rpc::Transport::new(tokio::io::empty(), tokio::io::sink(), &expected_version)?;
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        expected_version: expected_version.clone(),
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
        "Pi {expected_version}: 10 read-only RPCs passed; clean exit and terminal delivery verified"
    );
    Ok(())
}

#[cfg(test)]
#[path = "../tests/support/acceptance_version.rs"]
mod acceptance_version;

#[cfg(test)]
mod tests {
    use super::*;
    use acceptance_version::{Fixture, VERSIONS};

    #[tokio::test]
    async fn acceptance_requires_a_concrete_version_before_probing() {
        let root = tempfile::tempdir().unwrap();
        let executable = root.path().join("nonexistent-pi").into_os_string();
        for version in [
            pi_rpc::PI_VERSION_REQUIREMENT,
            ">=1.1.0, <=1.2.0",
            ">=1.1.0, <=1.3.0",
            "1.0.9",
            "1.4.1",
            "1.01.0",
            "1.3.0 ",
            "v1.3.0",
            "1.3.0-beta",
        ] {
            let result = run([executable.clone(), version.into()].into_iter()).await;
            assert_eq!(result, Err(Error::Version), "{version}");
        }
    }

    #[tokio::test]
    async fn default_version_is_the_latest_reviewed_exact_release() {
        let fixture = Fixture::new();
        for actual in VERSIONS {
            let probe = fixture.probe(actual);
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                run([probe.executable().as_os_str().to_owned()].into_iter()),
            )
            .await
            .expect("acceptance entrypoint stalled");
            probe.assert_outcome(result, PI_VERSION);
        }
    }

    #[tokio::test]
    async fn explicit_versions_are_exact_locks_before_rpc_startup() {
        let fixture = Fixture::new();
        for expected in VERSIONS {
            for actual in VERSIONS {
                let probe = fixture.probe(actual);
                let result = tokio::time::timeout(
                    std::time::Duration::from_secs(10),
                    run([probe.executable().as_os_str().to_owned(), expected.into()].into_iter()),
                )
                .await
                .expect("acceptance entrypoint stalled");
                probe.assert_outcome(result, expected);
            }
        }
    }
}
