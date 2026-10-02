//! An in-memory transport stands in for an ordered remote Sandbox byte pipe.
use pi_rpc::{Config, PI_VERSION, PiConnection, Transport, codec::JsonlReader};
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let (local, remote) = tokio::io::duplex(4096);
    let (read, write) = tokio::io::split(local);
    let (client, mut events, mut connection) =
        PiConnection::connect(Transport::new(read, write, PI_VERSION)?, Config::default())?;
    let peer = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(remote);
        let mut reader = JsonlReader::new(read, 4096);
        let (command, _) = reader.next().await.unwrap().unwrap();
        let reply = serde_json::json!({"type":"response","id":command["id"],"command":"get_messages","success":true,"data":{"messages":[]}});
        write
            .write_all(&pi_rpc::codec::encode(&reply, 4096).unwrap())
            .await
            .unwrap();
        // Keep the pipe open until orderly stdin EOF.
        assert!(reader.next().await.unwrap().is_none());
    });
    assert!(client.get_messages().await?.messages.is_empty());
    let _ = connection.close().await;
    peer.await.unwrap();
    let _ = events.recv().await;
    Ok(())
}
