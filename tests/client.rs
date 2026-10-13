use pi_rpc::{
    Config, Error, PI_VERSION, PiClient, PiConnection, PiEvents, Transport,
    codec::{self, JsonlReader},
    protocol::*,
};
use serde_json::{Value, json};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt, DuplexStream, ReadHalf, WriteHalf};

type PeerReader = JsonlReader<ReadHalf<DuplexStream>>;
struct EventThenFailure {
    buffered: bool,
}
impl tokio::io::AsyncRead for EventThenFailure {
    fn poll_read(
        mut self: std::pin::Pin<&mut Self>,
        _: &mut std::task::Context<'_>,
        buf: &mut tokio::io::ReadBuf<'_>,
    ) -> std::task::Poll<std::io::Result<()>> {
        if self.buffered {
            self.buffered = false;
            buf.put_slice(b"{\"type\":\"agent_start\"}\n");
            std::task::Poll::Ready(Ok(()))
        } else {
            std::task::Poll::Ready(Err(std::io::Error::other("event-first transport error")))
        }
    }
}
async fn bounded_event(events: &mut PiEvents) -> Option<pi_rpc::Result<Event>> {
    tokio::time::timeout(Duration::from_secs(2), events.recv())
        .await
        .expect("terminal delivery must not hang")
}
fn setup(
    config: Config,
) -> (
    PiClient,
    PiEvents,
    PiConnection,
    PeerReader,
    WriteHalf<DuplexStream>,
) {
    let (local, remote) = tokio::io::duplex(65536);
    let (r, w) = tokio::io::split(local);
    let (c, e, o) =
        PiConnection::connect(Transport::new(r, w, PI_VERSION).unwrap(), config).unwrap();
    let (r, w) = tokio::io::split(remote);
    (c, e, o, JsonlReader::new(r, 65536), w)
}
async fn next(r: &mut PeerReader) -> Value {
    tokio::time::timeout(Duration::from_secs(2), r.next())
        .await
        .unwrap()
        .unwrap()
        .unwrap()
        .0
}
async fn send(w: &mut WriteHalf<DuplexStream>, v: Value) {
    w.write_all(&codec::encode(&v, 65536).unwrap())
        .await
        .unwrap();
}
async fn reply(w: &mut WriteHalf<DuplexStream>, command: &Value, data: Option<Value>) {
    let mut v =
        json!({"type":"response","id":command["id"],"command":command["type"],"success":true});
    if let Some(data) = data {
        v["data"] = data;
    }
    send(w, v).await;
}
async fn terminal(o: &mut PiConnection) -> Error {
    tokio::time::timeout(Duration::from_secs(2), o.wait_terminal())
        .await
        .unwrap()
}

#[path = "client/capacity.rs"]
mod capacity;
#[path = "client/events.rs"]
mod events;
#[path = "client/lifecycle.rs"]
mod lifecycle;
#[path = "client/requests.rs"]
mod requests;
#[path = "client/ui.rs"]
mod ui;
