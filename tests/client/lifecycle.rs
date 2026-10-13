use super::*;

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn event_first_eof_and_panicking_reader_do_not_hang_with_live_owner() {
    struct Panics;
    impl tokio::io::AsyncRead for Panics {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            _: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            panic!("intentional reader task panic");
        }
    }
    for (transport, expected) in [
        (
            Transport::new(tokio::io::empty(), tokio::io::sink(), PI_VERSION).unwrap(),
            Error::Eof,
        ),
        (
            Transport::new(Panics, tokio::io::sink(), PI_VERSION).unwrap(),
            Error::Transport("I/O task failed".into()),
        ),
    ] {
        let (_, mut events, mut owner) =
            PiConnection::connect(transport, Config::default()).unwrap();
        assert_eq!(
            bounded_event(&mut events).await,
            Some(Err(expected.clone()))
        );
        assert_eq!(terminal(&mut owner).await, expected);
        assert_eq!(bounded_event(&mut events).await, None);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn owner_drop_wakes_waiting_events_and_preserves_buffered_event_order() {
    let (client, mut events, owner, _reader, mut writer) = setup(Config::default());
    send(&mut writer, json!({"type":"agent_start"})).await;
    // Observe installation of the first event, then wait for another while owner lives.
    assert_eq!(
        bounded_event(&mut events).await,
        Some(Ok(Event::Known(KnownEvent::AgentStart)))
    );
    assert!(
        tokio::time::timeout(Duration::from_millis(10), events.recv())
            .await
            .is_err()
    );
    drop(owner);
    assert_eq!(bounded_event(&mut events).await, Some(Err(Error::Closed)));
    assert_eq!(bounded_event(&mut events).await, None);
    assert_eq!(client.abort().await, Err(Error::Closed));
}
#[tokio::test]
async fn transport_reader_failure_is_explicit_and_not_protocol_or_eof() {
    struct Broken;
    impl tokio::io::AsyncRead for Broken {
        fn poll_read(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            _: &mut tokio::io::ReadBuf<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("synthetic read failure")))
        }
    }
    let (_c, mut e, mut o) = PiConnection::connect(
        Transport::new(Broken, tokio::io::sink(), PI_VERSION).unwrap(),
        Config::default(),
    )
    .unwrap();
    assert!(matches!(terminal(&mut o).await, Error::Transport(_)));
    assert!(matches!(e.recv().await, Some(Err(Error::Transport(_)))));
}
#[tokio::test]
async fn close_drains_admitted_write_and_response_before_stdin_eof() {
    let (c, _e, mut o, mut r, mut w) = setup(Config::default());
    let command = tokio::spawn(async move { c.abort().await });
    let v = next(&mut r).await;
    let peer = tokio::spawn(async move {
        reply(&mut w, &v, None).await;
        assert!(r.next().await.unwrap().is_none());
    });
    assert_eq!(o.close().await, Error::Closed);
    command.await.unwrap().unwrap();
    peer.await.unwrap();
}
#[tokio::test]
async fn owner_drop_cleans_transport_but_client_clone_drop_is_inert() {
    let (c, _e, o, mut r, mut w) = setup(Config::default());
    drop(c.clone());
    let a = c.clone();
    let t = tokio::spawn(async move { a.abort().await });
    let v = next(&mut r).await;
    reply(&mut w, &v, None).await;
    t.await.unwrap().unwrap();
    drop(o);
    assert_eq!(c.abort().await, Err(Error::Closed));
    assert!(
        tokio::time::timeout(Duration::from_secs(2), r.next())
            .await
            .unwrap()
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn orderly_close_drains_writes_and_blocks_later_submissions() {
    let (c, _e, mut o, mut r, w) = setup(Config::default());
    let peer = tokio::spawn(async move {
        assert!(r.next().await.unwrap().is_none());
        drop(w);
    });
    assert_eq!(o.close().await, Error::Closed);
    assert_eq!(c.abort().await, Err(Error::Closed));
    peer.await.unwrap();
}
#[tokio::test(start_paused = true)]
async fn close_deadline_is_explicit() {
    let (c, _e, mut o, _r, _w) = setup(Config {
        close_timeout: Duration::from_millis(10),
        ..Default::default()
    });
    assert_eq!(o.close().await, Error::CloseTimeout);
    assert_eq!(c.abort().await, Err(Error::CloseTimeout));
}
