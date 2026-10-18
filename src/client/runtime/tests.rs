use super::*;
use crate::{
    client::{Config, PiConnection},
    transport::Transport,
};
use std::{
    future::Future,
    pin::Pin,
    sync::Mutex,
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncWrite, ReadBuf};

// A direct read-loop future lets the test deterministically leave its partial
// record buffered while EOF becomes ready and the executor clock passes exit.
async fn deadline_case(initial: &[u8], tail: &[u8], eof: bool, limit: usize) -> Result<()> {
    let (local, _live_peer) = tokio::io::duplex(64);
    let (r, w) = tokio::io::split(local);
    let config = Config {
        max_record_bytes: limit,
        ..Config::default()
    };
    let (client, _events, owner) =
        PiConnection::connect(Transport::new(r, w, crate::PI_VERSION).unwrap(), config).unwrap();
    let slots = client.writer.capacity();
    let (reader, mut peer) = tokio::io::duplex(16384);
    peer.write_all(initial).await.unwrap();
    let (_exit_tx, exit) = watch::channel(Some(Error::ProcessExit(Some(7))));
    let (events, mut rx) = mpsc::channel(8);
    let read = read_loop(
        Box::new(reader),
        Some(exit),
        client.clone(),
        events,
        Arc::new(Semaphore::new(65536)),
    );
    tokio::pin!(read);
    std::future::poll_fn(|cx| {
        assert!(read.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    peer.write_all(tail).await.unwrap();
    let peer = if eof {
        drop(peer);
        None
    } else {
        Some(peer)
    };
    tokio::time::advance(Duration::from_millis(51)).await;
    let now = tokio::time::Instant::now();
    let result = read.await;
    assert_eq!(
        tokio::time::Instant::now(),
        now,
        "final validation must not wait"
    );
    assert!(
        rx.try_recv().is_err(),
        "expired drain must not dispatch events"
    );
    assert_eq!(
        client.writer.capacity(),
        slots,
        "expired UI must not enqueue"
    );
    drop(peer);
    drop(owner);
    result
}

#[tokio::test(start_paused = true)]
async fn ready_protocol_errors_precede_elapsed_exit_deadline() {
    for (initial, tail, eof, limit) in [
        (&b"{\"type\":"[..], &b""[..], true, 4096),
        (&b"{\"type\":"[..], &b"invalid}\n"[..], true, 4096),
        (&b"{\"type\":"[..], &b"\"message_end\"}\n"[..], true, 4096),
        (&b"{\"type\":"[..], &b"\xff}\n"[..], true, 4096),
        (&b"{\"type\":"[..], &b"123456789"[..], false, 8),
        (
            &b""[..],
            &b"{\"type\":\"agent_start\"}\n{invalid}\n"[..],
            true,
            4096,
        ),
    ] {
        assert!(matches!(
            deadline_case(initial, tail, eof, limit).await,
            Err(Error::Protocol(_))
        ));
    }
}

#[tokio::test(start_paused = true)]
async fn exit_deadline_checks_eof_at_the_exact_ready_byte_budget() {
    for tail_bytes in [4095, 4096, 4097] {
        let tail = vec![b' '; tail_bytes];
        for eof in [false, true] {
            let result = deadline_case(b"{\"type\":", &tail, eof, 8192).await;
            if eof && tail_bytes <= 4096 {
                assert_eq!(
                    result,
                    Err(Error::Protocol("unterminated record at EOF".into())),
                    "{tail_bytes} ready bytes followed by EOF"
                );
            } else {
                assert_eq!(
                    result,
                    Err(Error::ProcessExit(Some(7))),
                    "{tail_bytes} ready bytes, EOF={eof}"
                );
            }
        }
    }
    // A complete record at the boundary remains valid. An LF outside the
    // validation allowance is only lookahead and must not dispatch the record.
    for prefix in [
        &b"{\"type\":\"agent_start\"}"[..],
        &b"{\"type\":\"extension_ui_request\",\"id\":\"ui\",\"method\":\"input\",\"title\":\"Title\"}"[..],
        &b"{\"type\":\"response\",\"command\":\"abort\",\"success\":true}"[..],
    ] {
        for record_bytes in [4095, 4096, 4097] {
            for ending in [&b"\n"[..], &b"\r\n"[..]] {
                let mut record = prefix.to_vec();
                record.resize(record_bytes - ending.len(), b' ');
                record.extend_from_slice(ending);
                assert_eq!(
                    deadline_case(b"", &record, true, 8192).await,
                    Err(Error::ProcessExit(Some(7))),
                    "complete {record_bytes}-byte record at cutoff"
                );
            }
        }
    }
}

#[tokio::test(start_paused = true)]
async fn elapsed_exit_keeps_cutoff_for_valid_ui_and_unfinished_live_pipes() {
    for (initial, tail, eof) in [
        (&b""[..], &b""[..], true),
        (&b"{\"type\":"[..], &b""[..], false),
        (&b""[..], &b"{\"type\":\"agent_start\"}\n"[..], true),
        (&b""[..], &b"{\"type\":\"extension_ui_request\",\"id\":\"ui\",\"method\":\"input\",\"title\":\"Title\"}\n"[..], true),
    ] {
        assert_eq!(deadline_case(initial, tail, eof, 4096).await,
            Err(Error::ProcessExit(Some(7))));
    }
}

struct PendingRead {
    reader: tokio::io::DuplexStream,
    pending: Option<tokio::sync::oneshot::Sender<()>>,
    ready: Pin<Box<tokio::time::Sleep>>,
    reads: Arc<Mutex<Vec<(usize, usize, tokio::time::Instant)>>>,
}
impl AsyncRead for PendingRead {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buffer: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let after_initial = self.pending.is_none();
        // The initial event/partial record is read normally. Once its Pending
        // barrier fires, hold all tail data until the paused exit deadline.
        if after_initial && self.ready.as_mut().poll(cx).is_pending() {
            return Poll::Pending;
        }
        let limit = buffer.remaining();
        let before = buffer.filled().len();
        let result = Pin::new(&mut self.reader).poll_read(cx, buffer);
        if after_initial && matches!(&result, Poll::Ready(Ok(()))) {
            self.reads.lock().unwrap().push((
                limit,
                buffer.filled().len() - before,
                tokio::time::Instant::now(),
            ));
        }
        if result.is_pending()
            && let Some(sender) = self.pending.take()
        {
            let _ = sender.send(());
        }
        result
    }
}
struct Flushed(Option<tokio::sync::oneshot::Sender<()>>);
impl AsyncWrite for Flushed {
    fn poll_write(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
        bytes: &[u8],
    ) -> Poll<std::io::Result<usize>> {
        Poll::Ready(Ok(bytes.len()))
    }
    fn poll_flush(mut self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        if let Some(sender) = self.0.take() {
            let _ = sender.send(());
        }
        Poll::Ready(Ok(()))
    }
    fn poll_shutdown(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<std::io::Result<()>> {
        Poll::Ready(Ok(()))
    }
}

#[tokio::test(start_paused = true)]
async fn exit_deadline_protocol_failure_reaches_pending_owner_and_events_once() {
    for tail_bytes in [0, 4096] {
        let started = tokio::time::Instant::now();
        let cutoff = started + Duration::from_millis(50);
        let checked_at = started + Duration::from_millis(51);
        // Queue the entire tail without backpressure. The read gate, not pipe
        // capacity or executor scheduling, keeps it unread until the cutoff.
        let (reader, mut peer) = tokio::io::duplex(8192);
        peer.write_all(b"{\"type\":\"agent_start\"}\n{\"type\":")
            .await
            .unwrap();
        let (pending_tx, pending) = tokio::sync::oneshot::channel();
        let (flushed_tx, flushed) = tokio::sync::oneshot::channel();
        let reads = Arc::new(Mutex::new(Vec::new()));
        let (_exit_tx, exit) = watch::channel(Some(Error::ProcessExit(Some(0))));
        let mut transport = Transport::new(
            PendingRead {
                reader,
                pending: Some(pending_tx),
                ready: Box::pin(tokio::time::sleep_until(cutoff)),
                reads: reads.clone(),
            },
            Flushed(Some(flushed_tx)),
            crate::PI_VERSION,
        )
        .unwrap();
        transport.exit = Some(exit);
        let (client, mut events, mut owner) =
            PiConnection::connect(transport, Config::default()).unwrap();
        let command_client = client.clone();
        let command = tokio::spawn(async move { command_client.get_state().await });
        pending.await.unwrap();
        flushed.await.unwrap();
        peer.write_all(&vec![b' '; tail_bytes]).await.unwrap();
        drop(peer);
        assert_eq!(tokio::time::Instant::now(), started);
        assert!(reads.lock().unwrap().is_empty(), "tail must remain unread");
        assert_eq!(client.shared.pending_count(), 1);
        assert_eq!(
            client.admission.available_permits(),
            Config::default().max_pending_commands - 1
        );
        tokio::time::advance(Duration::from_millis(51)).await;
        let terminal = owner.wait_terminal().await;
        assert!(matches!(terminal, Error::Protocol(_)), "{terminal:?}");
        assert_eq!(tokio::time::Instant::now(), checked_at);
        let expected_reads = if tail_bytes == 0 {
            vec![(4096, 0, checked_at)]
        } else {
            // A normal JsonlReader::next EOF read would request 4096 again.
            // The one-byte request proves the terminal EOF probe was reached.
            vec![(4096, 4096, checked_at), (1, 0, checked_at)]
        };
        assert_eq!(*reads.lock().unwrap(), expected_reads);
        assert_eq!(command.await.unwrap(), Err(terminal.clone()));
        assert_eq!(
            events.recv().await,
            Some(Ok(Event::Known(crate::protocol::KnownEvent::AgentStart)))
        );
        assert_eq!(events.recv().await, Some(Err(terminal.clone())));
        assert_eq!(events.recv().await, None);
        assert_eq!(client.shared.pending_count(), 0);
        assert_eq!(
            client.admission.available_permits(),
            Config::default().max_pending_commands
        );
        assert_eq!(
            client.budget.available_permits(),
            Config::default().max_writer_bytes
        );
        assert_eq!(owner.close().await, terminal);
    }
}
