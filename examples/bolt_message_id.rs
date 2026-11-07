//! In-memory input correlation. Never starts Pi/Bolt or calls a model.
use pi_rpc::protocol::bolt_message_id_v1::{Event, MessageId, QueueChange};
use pi_rpc::{BoltMessageIdV1, Config, PiConnection, Transport, codec};
use serde_json::json;
use tokio::io::AsyncWriteExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> pi_rpc::Result<()> {
    let (local, remote) = tokio::io::duplex(4096);
    let (read, write) = tokio::io::split(local);
    let (client, mut events, mut connection) = PiConnection::<BoltMessageIdV1>::connect(
        Transport::new(read, write, "1.5.0")?,
        Config::default(),
    )?;
    let peer = tokio::spawn(async move {
        let (read, mut write) = tokio::io::split(remote);
        let mut reader = codec::JsonlReader::new(read, 4096);
        let (command, _) = reader.next().await.unwrap().unwrap();
        assert_eq!(command["type"], "prompt");
        assert_eq!(command["message_id"], "input-A");
        for record in [
            json!({"type":"queue_update","steering":[],"followUp":[],"tracking":{
                "version":1,"instanceId":"live-instance","sequence":1,"batchId":"handoff-1",
                "change":{"reason":"prompt_start","source":"prompt","messageIds":["input-A"]},
                "steeringMessageIds":[],"followUpMessageIds":[]}}),
            json!({"type":"response","id":command["id"],"command":"prompt","success":true,
                "data":{"disposition":"started","message_id":"input-A"}}),
        ] {
            write
                .write_all(&codec::encode(&record, 4096).unwrap())
                .await
                .unwrap();
        }
        assert!(reader.next().await.unwrap().is_none());
    });
    // Establish the mapping and start consumption before submitting. Handoff
    // events may arrive before the acceptance response. Neither means completion.
    let message_id = MessageId::new("input-A")?;
    let expected = message_id.clone();
    let consumer = tokio::spawn(async move {
        let mut handoffs = 0;
        while let Some(record) = events.recv().await {
            match record {
                Ok(Event::QueueUpdate(update)) => {
                    if let Some(tracking) = update.tracking
                        && let QueueChange::PromptStart { message_ids, .. } = tracking.change
                    {
                        assert_eq!(message_ids, vec![Some(expected.clone())]);
                        handoffs += 1;
                    }
                }
                Ok(Event::Pi(_)) => {}
                Err(pi_rpc::Error::Closed) => break,
                Err(error) => panic!("unexpected terminal error: {error}"),
            }
        }
        assert_eq!(handoffs, 1);
    });
    let accepted = client
        .prompt_with_message_id("Example input", None, None, message_id)
        .await?;
    assert_eq!(accepted.message_id.as_str(), "input-A");
    connection.close().await;
    peer.await.unwrap();
    consumer.await.unwrap();
    Ok(())
}
