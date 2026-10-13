use super::*;

async fn assert_bounded_writer_under_stalled_peer() {
    use std::{
        future::{Future, poll_fn},
        task::Poll,
    };
    for (bytes, count, expected) in [(60, 128, "writer bytes"), (65536, 1, "writer records")] {
        let (local, mut remote) = tokio::io::duplex(1);
        let (r, w) = tokio::io::split(local);
        let (c, _e, o) = PiConnection::connect(
            Transport::new(r, w, PI_VERSION).unwrap(),
            Config {
                max_writer_bytes: bytes,
                max_writer_records: count,
                ..Default::default()
            },
        )
        .unwrap();
        let a = c.clone();
        let first = tokio::spawn(async move { a.abort().await });
        // Reading one byte proves the serialized writer has begun and is stalled.
        let mut b = [0];
        remote.read_exact(&mut b).await.unwrap();
        let second = if count == 1 {
            let mut pending = Box::pin(c.abort());
            // Drive submission ourselves. Pending proves admission succeeded and
            // the command is awaiting the stalled writer, without scheduler guesses.
            poll_fn(|cx| {
                assert!(pending.as_mut().poll(cx).is_pending());
                Poll::Ready(())
            })
            .await;
            Some(pending)
        } else {
            None
        };
        assert_eq!(c.abort().await, Err(Error::Capacity(expected)));
        drop(o);
        assert_eq!(first.await.unwrap(), Err(Error::Closed));
        if let Some(t) = second {
            assert_eq!(t.await, Err(Error::Closed));
        }
    }
}
#[tokio::test]
async fn bounded_writer_bytes_and_count_under_stalled_peer() {
    assert_bounded_writer_under_stalled_peer().await;
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn bounded_writer_under_stalled_peer_on_multiple_threads() {
    assert_bounded_writer_under_stalled_peer().await;
}

#[tokio::test]
async fn oversized_record_and_unterminated_eof_are_protocol_errors() {
    for (bytes, max) in [
        (b"{\"type\":\"agent_start\"}".as_slice(), 100),
        (b"{\"type\":\"agent_start\"}\n".as_slice(), 5),
    ] {
        let (_c, _e, mut o, _r, mut w) = setup(Config {
            max_record_bytes: max,
            ..Default::default()
        });
        w.write_all(bytes).await.unwrap();
        w.shutdown().await.unwrap();
        assert!(matches!(terminal(&mut o).await, Error::Protocol(_)));
    }
    assert_eq!(
        codec::encode(&json!({"secret":"input"}), 1),
        Err(Error::Capacity("record bytes"))
    );
}
#[test]
fn invalid_config_and_declared_version_are_rejected() {
    assert!(matches!(
        Transport::new(tokio::io::empty(), tokio::io::sink(), "unknown"),
        Err(Error::Version)
    ));
    let t = Transport::new(tokio::io::empty(), tokio::io::sink(), PI_VERSION).unwrap();
    assert!(matches!(
        PiConnection::connect(
            t,
            Config {
                max_event_count: 0,
                ..Default::default()
            }
        ),
        Err(Error::Capacity(_))
    ));
}
