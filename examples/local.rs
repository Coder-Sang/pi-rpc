use pi_rpc::ProcessOptions;
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
use pi_rpc::{AutoProcess, PiClient, PiEvents, PiProcess, PiProtocol};
#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let process = pi_rpc::subprocess::spawn_auto(ProcessOptions {
        executable: std::env::args_os()
            .nth(1)
            .map(Into::into)
            .unwrap_or_else(|| "pi".into()),
        args: vec!["--no-session".into()],
        ..Default::default()
    })
    .await?;
    match process {
        #[cfg(feature = "pi_1_3")]
        AutoProcess::V1_3 {
            client,
            events,
            process,
        } => run(client, events, process).await,
        #[cfg(feature = "pi_1_4")]
        AutoProcess::V1_4 {
            client,
            events,
            process,
        } => run(client, events, process).await,
        #[cfg(feature = "pi_1_5")]
        AutoProcess::V1_5 {
            client,
            events,
            process,
        } => run(client, events, process).await,
        _ => Err(pi_rpc::Error::Version),
    }
}
#[cfg(any(feature = "pi_1_3", feature = "pi_1_4", feature = "pi_1_5"))]
async fn run<P: PiProtocol>(
    client: PiClient<P>,
    mut events: PiEvents<P>,
    mut process: PiProcess<P>,
) -> pi_rpc::Result<()> {
    // Consume events before submission. No prompt or paid model request here.
    let consumer = tokio::spawn(async move { while events.recv().await.is_some() {} });
    let state = client.get_state().await?;
    println!(
        "streaming={} pending={}",
        state.is_streaming, state.pending_message_count
    );
    let reason = process.close().await;
    println!("terminal: {reason}");
    consumer.await.unwrap();
    Ok(())
}
