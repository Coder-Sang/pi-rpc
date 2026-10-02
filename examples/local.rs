use pi_rpc::{PiProcess, ProcessOptions};
#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let (client, mut events, mut process) = PiProcess::spawn(ProcessOptions {
        executable: std::env::args_os()
            .nth(1)
            .map(Into::into)
            .unwrap_or_else(|| "pi".into()),
        args: vec!["--no-session".into()],
        ..Default::default()
    })
    .await?;
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
