use super::*;
use crate::{codec, protocol::*};
use serde_json::Value;

#[tokio::test]
#[ignore = "manual Release encoding measurement; no timing assertions"]
async fn image_encoding_measurement() {
    for count in [1, 4] {
        let command = Command::Prompt {
            message: "benchmark".into(),
            streaming_behavior: None,
            images: Some(
                (0..count)
                    .map(|_| ImageContent {
                        r#type: ImageType::Image,
                        data: "A".repeat(9 * 1024 * 1024 / 2 - 4096),
                        mime_type: "image/png".into(),
                        extra: Default::default(),
                    })
                    .collect(),
            ),
        };
        let (client, _events, owner) = budget_fixture(Config::default());
        let mut single = Vec::new();
        let mut reserved = Vec::new();
        let mut peak = 0;
        for _ in 0..7 {
            let start = std::time::Instant::now();
            drop(codec::encode(&command, client.shared.config.max_record_bytes).unwrap());
            single.push(start.elapsed().as_micros());
            let start = std::time::Instant::now();
            let prepared = client.prepare_record(&command).unwrap();
            peak = client.shared.config.max_writer_bytes - client.budget.available_permits();
            drop(prepared);
            reserved.push(start.elapsed().as_micros());
        }
        single.sort_unstable();
        reserved.sort_unstable();
        println!(
            "images={count} single_median_us={} reserved_median_us={} reserved_peak_bytes={peak}",
            single[3], reserved[3]
        );
        drop(owner);
    }
}

fn budget_fixture(config: Config) -> (PiClient, PiEvents, PiConnection) {
    PiConnection::connect(
        Transport::new(tokio::io::empty(), tokio::io::sink(), crate::PI_VERSION).unwrap(),
        config,
    )
    .unwrap()
}

struct Counted<'a> {
    calls: &'a std::sync::atomic::AtomicUsize,
}
impl serde::Serialize for Counted<'_> {
    fn serialize<S: serde::Serializer>(
        &self,
        serializer: S,
    ) -> std::result::Result<S::Ok, S::Error> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        serializer.serialize_bool(true)
    }
}

#[tokio::test]
async fn dropped_unsubmitted_and_submitted_futures_release_pending_slots() {
    let (local, _peer) = tokio::io::duplex(1);
    let (r, w) = tokio::io::split(local);
    let (client, _events, owner) = PiConnection::connect(
        Transport::new(r, w, crate::PI_VERSION).unwrap(),
        Config {
            max_pending_commands: 1,
            ..Config::default()
        },
    )
    .unwrap();
    drop(client.command(Command::GetState));
    assert_eq!(client.admission.available_permits(), 1);
    let mut command = Box::pin(client.command(Command::GetState));
    assert!(
        std::future::poll_fn(|cx| std::task::Poll::Ready(command.as_mut().poll(cx)))
            .await
            .is_pending()
    );
    assert_eq!(client.admission.available_permits(), 0);
    assert_eq!(client.shared.pending_count(), 1);
    drop(command);
    assert_eq!(client.admission.available_permits(), 1);
    assert!(client.shared.pending_count() == 0);
    // The admitted write still owns its budget after the waiter is dropped.
    assert!(client.budget.available_permits() < client.shared.config.max_writer_bytes);
    drop(owner);
}

#[tokio::test]
async fn queue_and_byte_rejection_precede_output_encoding() {
    use std::sync::atomic::AtomicUsize;
    let (client, _events, owner) = budget_fixture(Config {
        max_writer_records: 1,
        max_writer_bytes: 5,
        ..Config::default()
    });
    let held = client.prepare_record(&true).unwrap();
    let calls = AtomicUsize::new(0);
    assert!(matches!(
        client.prepare_record(&Counted { calls: &calls }),
        Err(Error::Capacity("writer records"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    drop(held);
    assert_eq!(client.budget.available_permits(), 5);
    drop(owner);
    let (client, _events, owner) = budget_fixture(Config {
        max_writer_records: 2,
        max_writer_bytes: 5,
        ..Config::default()
    });
    let held = client.prepare_record(&true).unwrap();
    assert!(matches!(
        client.prepare_record(&Counted { calls: &calls }),
        Err(Error::Capacity("writer bytes"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    drop(held);
    assert_eq!(client.budget.available_permits(), 5);
    assert_eq!(client.writer.capacity(), 2);
    drop(owner);
}

#[tokio::test]
async fn write_and_flush_errors_return_reserved_bytes() {
    struct FailingWriter {
        fail_flush: bool,
    }
    impl tokio::io::AsyncWrite for FailingWriter {
        fn poll_write(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
            bytes: &[u8],
        ) -> std::task::Poll<std::io::Result<usize>> {
            std::task::Poll::Ready(if self.fail_flush {
                Ok(bytes.len())
            } else {
                Err(std::io::Error::other("write failed"))
            })
        }
        fn poll_flush(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Err(std::io::Error::other("flush failed")))
        }
        fn poll_shutdown(
            self: std::pin::Pin<&mut Self>,
            _: &mut std::task::Context<'_>,
        ) -> std::task::Poll<std::io::Result<()>> {
            std::task::Poll::Ready(Ok(()))
        }
    }
    for fail_flush in [false, true] {
        let (input, _peer) = tokio::io::duplex(64);
        let (client, _events, mut owner) = PiConnection::connect(
            Transport::new(input, FailingWriter { fail_flush }, crate::PI_VERSION).unwrap(),
            Config {
                max_writer_bytes: 5,
                ..Config::default()
            },
        )
        .unwrap();
        let prepared = client.prepare_record(&true).unwrap();
        assert_eq!(client.budget.available_permits(), 0);
        let sent = client.shared.enqueue_reply(prepared).unwrap();
        let error = sent.await.unwrap().unwrap_err();
        assert!(matches!(&error, Error::Transport(_)));
        assert_eq!(owner.wait_terminal().await, error);
        assert_eq!(client.budget.available_permits(), 5);
    }
}

#[tokio::test]
async fn framed_output_budget_includes_lf() {
    let (client, _events, owner) = budget_fixture(Config {
        max_record_bytes: 4,
        max_writer_bytes: 4,
        ..Config::default()
    });
    assert!(matches!(
        client.prepare_record(&true),
        Err(Error::Capacity("writer bytes"))
    ));
    assert_eq!(client.budget.available_permits(), 4);
    drop(owner);
    let (client, _events, owner) = budget_fixture(Config {
        max_record_bytes: 4,
        max_writer_bytes: 5,
        ..Config::default()
    });
    let prepared = client.prepare_record(&true).unwrap();
    assert_eq!(prepared.bytes, b"true\n");
    assert_eq!(client.budget.available_permits(), 0);
    drop(prepared);
    assert_eq!(client.budget.available_permits(), 5);
    drop(owner);
}

#[tokio::test]
async fn concurrent_encoding_holds_budget_before_serializer_runs() {
    use std::sync::{Barrier, atomic::AtomicUsize};
    struct Paused {
        calls: AtomicUsize,
        entered: Arc<Barrier>,
        release: Arc<Barrier>,
    }
    impl serde::Serialize for Paused {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                self.entered.wait();
                self.release.wait();
            }
            serializer.serialize_bool(true)
        }
    }
    let (client, _events, owner) = budget_fixture(Config {
        max_writer_bytes: 5,
        max_writer_records: 2,
        ..Config::default()
    });
    let entered = Arc::new(Barrier::new(2));
    let release = Arc::new(Barrier::new(2));
    let worker = {
        let client = client.clone();
        let entered = entered.clone();
        let release = release.clone();
        std::thread::spawn(move || {
            client
                .prepare_record(&Paused {
                    calls: AtomicUsize::new(0),
                    entered,
                    release,
                })
                .unwrap()
        })
    };
    entered.wait();
    assert_eq!(client.budget.available_permits(), 0);
    let calls = AtomicUsize::new(0);
    assert!(matches!(
        client.prepare_record(&Counted { calls: &calls }),
        Err(Error::Capacity("writer bytes"))
    ));
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    release.wait();
    drop(worker.join().unwrap());
    assert_eq!(client.budget.available_permits(), 5);
    assert_eq!(client.writer.capacity(), 2);
    drop(owner);
}

#[tokio::test]
async fn second_pass_failure_panic_size_change_and_close_refund_admission() {
    use std::sync::atomic::AtomicUsize;
    struct Fault<'a> {
        calls: AtomicUsize,
        mode: usize,
        shared: &'a Shared,
    }
    impl serde::Serialize for Fault<'_> {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            if self.calls.fetch_add(1, Ordering::SeqCst) == 1 {
                match self.mode {
                    0 => return Err(serde::ser::Error::custom("injected encoding failure")),
                    1 => panic!("injected encoding panic"),
                    2 => return serializer.serialize_bool(false),
                    3 => return serializer.serialize_u8(0),
                    _ => {
                        self.shared.assert_unlocked();
                        self.shared.begin_close();
                    }
                }
            }
            serializer.serialize_bool(true)
        }
    }
    for mode in 0..5 {
        let (client, _events, owner) = budget_fixture(Config::default());
        let before = client.budget.available_permits();
        let slots = client.writer.capacity();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            client.send_one_way(|| {
                Ok(Fault {
                    calls: AtomicUsize::new(0),
                    mode,
                    shared: &client.shared,
                })
            })
        }));
        if mode == 1 {
            assert!(result.is_err());
        } else {
            assert!(result.unwrap().is_err());
        }
        assert_eq!(client.budget.available_permits(), before);
        assert_eq!(client.writer.capacity(), slots);
        assert!(client.shared.pending_count() == 0);
        drop(owner);
    }
}

#[tokio::test]
async fn one_way_admission_skips_construction_after_close_or_terminal() {
    for terminal in [None, Some(Error::Transport("failed writer".into()))] {
        let (local, _peer) = tokio::io::duplex(1024);
        let (r, w) = tokio::io::split(local);
        let (client, _events, owner) = PiConnection::connect(
            Transport::new(r, w, crate::PI_VERSION).unwrap(),
            Config::default(),
        )
        .unwrap();
        let expected = terminal.clone().unwrap_or(Error::Closed);
        if let Some(error) = terminal {
            client.shared.finish(error);
        } else {
            client.shared.begin_close();
        }
        assert_eq!(
            client
                .send_one_way(|| -> Result<Value> { panic!("record must not be constructed") })
                .unwrap_err(),
            expected
        );
        drop(owner);
    }
}

#[tokio::test]
async fn one_way_admission_rechecks_close_and_terminal_after_serialization() {
    struct Race<'a> {
        shared: &'a Shared,
        terminal: Option<Error>,
    }
    impl serde::Serialize for Race<'_> {
        fn serialize<S: serde::Serializer>(
            &self,
            serializer: S,
        ) -> std::result::Result<S::Ok, S::Error> {
            // This callback deterministically puts the state change between
            // preflight and enqueue, and asserts encoding releases the lock.
            self.shared.assert_unlocked();
            if let Some(error) = &self.terminal {
                self.shared.finish(error.clone());
            } else {
                self.shared.begin_close();
            }
            serializer.serialize_unit()
        }
    }
    for terminal in [None, Some(Error::Transport("racing writer failure".into()))] {
        let (local, _peer) = tokio::io::duplex(1024);
        let (r, w) = tokio::io::split(local);
        let (client, _events, owner) = PiConnection::connect(
            Transport::new(r, w, crate::PI_VERSION).unwrap(),
            Config::default(),
        )
        .unwrap();
        let records = client.writer.capacity();
        let bytes = client.budget.available_permits();
        let expected = terminal.clone().unwrap_or(Error::Closed);
        assert_eq!(
            client
                .send_one_way(|| Ok(Race {
                    shared: &client.shared,
                    terminal
                }))
                .unwrap_err(),
            expected
        );
        assert_eq!(client.writer.capacity(), records);
        assert_eq!(client.budget.available_permits(), bytes);
        assert!(client.shared.pending_count() == 0);
        drop(owner);
    }
}

#[tokio::test]
async fn closed_event_channel_waits_for_publication_and_cancelled_recv_keeps_error() {
    let (tx, rx) = mpsc::channel(1);
    let (terminal, terminal_rx) = watch::channel(None);
    let mut events = PiEvents {
        rx,
        terminal: terminal_rx,
        error_delivered: false,
    };
    drop(tx); // Deterministically create the reader/supervisor publication gap.
    assert!(
        tokio::time::timeout(Duration::from_millis(5), events.recv())
            .await
            .is_err()
    );
    terminal.send_replace(Some(Error::Transport("delayed terminal".into())));
    assert_eq!(
        events.recv().await,
        Some(Err(Error::Transport("delayed terminal".into())))
    );
    assert_eq!(events.recv().await, None);
}

#[tokio::test]
async fn terminal_authority_disappearing_after_event_close_returns_closed_once() {
    let (tx, rx) = mpsc::channel(1);
    let (terminal, terminal_rx) = watch::channel(None);
    let mut events = PiEvents {
        rx,
        terminal: terminal_rx,
        error_delivered: false,
    };
    drop(tx);
    drop(terminal);
    assert_eq!(events.recv().await, Some(Err(Error::Closed)));
    assert_eq!(events.recv().await, None);
}
