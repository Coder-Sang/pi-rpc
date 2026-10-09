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
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn event_consumer_gets_exactly_one_terminal_before_waiting_for_owner_3000_runs() {
    for iteration in 0..3000 {
        let (_, mut events, mut owner) = PiConnection::connect(
            Transport::new(
                EventThenFailure { buffered: false },
                tokio::io::sink(),
                PI_VERSION,
            )
            .unwrap(),
            Config::default(),
        )
        .unwrap();
        let expected = Error::Transport("event-first transport error".into());
        // Crucially do not wait_terminal first: that hid the race in earlier tests.
        assert_eq!(
            bounded_event(&mut events).await,
            Some(Err(expected.clone())),
            "iteration {iteration}"
        );
        assert_eq!(terminal(&mut owner).await, expected);
        assert_eq!(bounded_event(&mut events).await, None);
        assert_eq!(bounded_event(&mut events).await, None);
    }
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn buffered_event_precedes_exactly_one_terminal_error_1000_runs() {
    for iteration in 0..1000 {
        let (_, mut events, mut owner) = PiConnection::connect(
            Transport::new(
                EventThenFailure { buffered: true },
                tokio::io::sink(),
                PI_VERSION,
            )
            .unwrap(),
            Config::default(),
        )
        .unwrap();
        assert_eq!(
            bounded_event(&mut events).await,
            Some(Ok(Event::Known(KnownEvent::AgentStart))),
            "iteration {iteration}"
        );
        let expected = Error::Transport("event-first transport error".into());
        assert_eq!(
            bounded_event(&mut events).await,
            Some(Err(expected.clone())),
            "iteration {iteration}"
        );
        assert_eq!(terminal(&mut owner).await, expected);
        assert_eq!(bounded_event(&mut events).await, None);
    }
}
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
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn wrong_command_same_id_reliably_delivers_protocol_on_multiple_threads() {
    for _ in 0..100 {
        let (c, _e, mut o, mut r, mut w) = setup(Config::default());
        let task = tokio::spawn(async move { c.abort().await });
        let v = next(&mut r).await;
        send(
            &mut w,
            json!({"type":"response","id":v["id"],"command":"abort_bash","success":true}),
        )
        .await;
        let caller = task.await.unwrap();
        let owner = terminal(&mut o).await;
        assert!(matches!(caller, Err(Error::Protocol(_))));
        assert_eq!(caller, Err(owner));
    }
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

#[tokio::test]
async fn fragments_coalescing_crlf_unicode_and_read_before_submission() {
    let (c, mut events, o, mut r, mut w) = setup(Config::default());
    let raw = json!({"type":"future_event","text":"A\u{2028}B\u{2029}C🦀"});
    let bytes = codec::encode(&raw, 65536).unwrap();
    for b in bytes {
        w.write_all(&[b]).await.unwrap();
    }
    w.write_all(b"{\"type\":\"agent_start\"}\r\n{\"type\":\"turn_start\"}\n")
        .await
        .unwrap();
    assert_eq!(events.recv().await.unwrap().unwrap(), Event::Unknown(raw));
    assert_eq!(
        events.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::AgentStart)
    );
    assert_eq!(
        events.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::TurnStart)
    );
    let t = tokio::spawn(async move { c.abort().await });
    let v = next(&mut r).await;
    assert_eq!(v["type"], "abort");
    reply(&mut w, &v, None).await;
    t.await.unwrap().unwrap();
    drop(o);
}
#[tokio::test]
async fn out_of_order_responses_and_abort_not_blocked_by_execution() {
    let (c, _e, o, mut r, mut w) = setup(Config::default());
    let a = c.clone();
    let slow = tokio::spawn(async move { a.bash("not actually executed", None).await });
    let bash = next(&mut r).await;
    let a = c.clone();
    let abort = tokio::spawn(async move { a.abort().await });
    let abort_cmd = next(&mut r).await;
    reply(&mut w, &abort_cmd, None).await;
    abort.await.unwrap().unwrap();
    assert!(!slow.is_finished());
    reply(
        &mut w,
        &bash,
        Some(json!({"output":"done","cancelled":true,"truncated":false})),
    )
    .await;
    assert!(slow.await.unwrap().unwrap().cancelled);
    drop(o);
}
#[tokio::test(start_paused = true)]
async fn timeout_late_response_and_no_auto_retry() {
    let (c, mut events, o, mut r, mut w) = setup(Config::default());
    let a = c.clone();
    let t = tokio::spawn(async move {
        a.command_with_timeout(Command::Abort, Duration::from_millis(20))
            .await
    });
    let v = next(&mut r).await;
    tokio::time::advance(Duration::from_millis(21)).await;
    assert_eq!(t.await.unwrap(), Err(Error::Timeout));
    reply(&mut w, &v, None).await;
    send(&mut w, json!({"type":"agent_settled","aborted":false})).await;
    assert_eq!(
        events.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::AgentSettled { aborted: false })
    );
    let a = c.clone();
    let t = tokio::spawn(async move { a.abort().await });
    let v2 = next(&mut r).await;
    assert_ne!(v["id"], v2["id"]);
    reply(&mut w, &v2, None).await;
    t.await.unwrap().unwrap();
    drop(o);
}
#[tokio::test]
async fn cancellation_does_not_withdraw_and_releases_pending_slot() {
    let config = Config {
        max_pending_commands: 1,
        ..Default::default()
    };
    let (c, _events, o, mut r, mut w) = setup(config);
    let a = c.clone();
    let t = tokio::spawn(async move { a.abort().await });
    let v = next(&mut r).await;
    assert_eq!(c.abort().await, Err(Error::Capacity("pending commands")));
    t.abort();
    let _ = t.await;
    reply(&mut w, &v, None).await;
    let a = c.clone();
    let t = tokio::spawn(async move { a.abort().await });
    let v2 = next(&mut r).await;
    reply(&mut w, &v2, None).await;
    t.await.unwrap().unwrap();
    drop(o);
}
#[tokio::test]
async fn prompt_acceptance_and_cancelled_session_results() {
    let (c, mut events, o, mut r, mut w) = setup(Config::default());
    for disposition in [
        PromptDisposition::Started,
        PromptDisposition::Queued,
        PromptDisposition::Handled,
    ] {
        let a = c.clone();
        let t = tokio::spawn(async move { a.prompt("synthetic input").await });
        let v = next(&mut r).await;
        if disposition == PromptDisposition::Started {
            send(
                &mut w,
                json!({"type":"agent_end","messages":[],"willRetry":true}),
            )
            .await;
        }
        reply(&mut w, &v, Some(json!({"disposition":disposition}))).await;
        assert_eq!(t.await.unwrap().unwrap(), disposition);
    }
    assert!(matches!(
        events.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::AgentEnd {
            will_retry: true,
            ..
        })
    ));
    for cmd in [
        Command::NewSession {
            parent_session: None,
        },
        Command::SwitchSession {
            session_path: "synthetic".into(),
        },
        Command::Clone,
    ] {
        let a = c.clone();
        let t = tokio::spawn(async move { a.typed::<SessionChange>(cmd).await });
        let v = next(&mut r).await;
        reply(&mut w, &v, Some(json!({"cancelled":true}))).await;
        assert!(t.await.unwrap().unwrap().cancelled);
    }
    let a = c.clone();
    let t = tokio::spawn(async move { a.fork("entry").await });
    let v = next(&mut r).await;
    reply(
        &mut w,
        &v,
        Some(json!({"text":"original","cancelled":true})),
    )
    .await;
    assert!(t.await.unwrap().unwrap().cancelled);
    drop(o);
}
#[tokio::test]
async fn rejection_is_nonterminal_and_missing_id_response_is_observable() {
    let (c, mut events, o, mut r, mut w) = setup(Config::default());
    let a = c.clone();
    let t = tokio::spawn(async move { a.abort().await });
    let v = next(&mut r).await;
    send(&mut w,json!({"type":"response","id":v["id"],"command":"abort","success":false,"error":"synthetic rejection"})).await;
    assert!(matches!(t.await.unwrap(), Err(Error::Rejected { .. })));
    assert!(o.terminal_reason().is_none());
    send(
        &mut w,
        json!({"type":"response","command":"parse","success":false,"error":"invalid input"}),
    )
    .await;
    assert!(matches!(
        events.recv().await.unwrap().unwrap(),
        Event::UncorrelatedResponse(_)
    ));
    drop(o);
}
#[tokio::test]
async fn last_assistant_text_handles_actual_empty_session_response() {
    let (c, _e, o, mut r, mut w) = setup(Config::default());
    for (data, expected) in [
        (json!({}), None),
        (json!({"text":null}), None),
        (json!({"text":"answer"}), Some("answer")),
    ] {
        let client = c.clone();
        let pending = tokio::spawn(async move { client.get_last_assistant_text().await });
        let command = next(&mut r).await;
        assert_eq!(command["type"], "get_last_assistant_text");
        reply(&mut w, &command, Some(data)).await;
        assert_eq!(pending.await.unwrap().unwrap().text.as_deref(), expected);
    }
    drop(o);
}
#[tokio::test]
async fn upgraded_events_deliver_aborted_and_optional_tool_duration() {
    let (_c, mut e, o, _r, mut w) = setup(Config::default());
    for aborted in [false, true] {
        send(&mut w, json!({"type":"agent_settled","aborted":aborted})).await;
        assert_eq!(
            e.recv().await.unwrap().unwrap(),
            Event::Known(KnownEvent::AgentSettled { aborted })
        );
    }
    for duration in [None, Some(12.5)] {
        let mut wire = json!({"type":"tool_execution_end","toolCallId":"t","toolName":"open","result":{"durationMs":"open JSON"},"isError":false,"extensionField":true});
        if let Some(duration) = duration {
            wire["durationMs"] = json!(duration);
        }
        send(&mut w, wire).await;
        assert!(
            matches!(e.recv().await.unwrap().unwrap(), Event::Known(KnownEvent::ToolExecutionEnd { duration_ms, extra, result, .. }) if duration_ms == duration && extra["extensionField"] == true && result["durationMs"] == "open JSON")
        );
    }
    drop(o);
}
#[tokio::test]
async fn malformed_upgrade_fields_fail_owner_events_and_pending_command() {
    let tool = json!({"type":"tool_execution_end","toolCallId":"t","toolName":"open","result":null,"isError":false});
    let mut cases = vec![json!({"type":"agent_settled"})];
    for bad in [Value::Null, json!("secret"), json!(0), json!([]), json!({})] {
        cases.push(json!({"type":"agent_settled","aborted":bad}));
    }
    for bad in [
        Value::Null,
        json!("secret"),
        json!(false),
        json!([]),
        json!({}),
    ] {
        let mut wire = tool.clone();
        wire["durationMs"] = bad;
        cases.push(wire);
    }
    for wire in cases {
        let (c, mut e, mut o, mut r, mut w) = setup(Config::default());
        let pending = tokio::spawn(async move { c.abort().await });
        let _ = next(&mut r).await;
        send(&mut w, wire).await;
        let err = terminal(&mut o).await;
        assert!(matches!(err, Error::Protocol(_)));
        assert!(!err.to_string().contains("secret"));
        assert_eq!(pending.await.unwrap(), Err(err.clone()));
        assert_eq!(e.recv().await, Some(Err(err)));
        assert!(e.recv().await.is_none());
    }
}
#[tokio::test]
async fn malformed_known_event_fails_owner_and_pending_commands() {
    let (c, mut e, mut o, mut r, mut w) = setup(Config::default());
    let t = tokio::spawn(async move { c.abort().await });
    let _ = next(&mut r).await;
    send(&mut w,json!({"type":"message_update","assistantMessageEvent":{"type":"text_delta","delta":"secret"}})).await;
    let err = terminal(&mut o).await;
    assert!(matches!(err, Error::Protocol(_)));
    assert!(!err.to_string().contains("secret"));
    assert_eq!(t.await.unwrap(), Err(err.clone()));
    assert_eq!(e.recv().await, Some(Err(err)));
    assert!(e.recv().await.is_none());
}
#[tokio::test]
async fn mismatched_response_command_fails_original_waiter() {
    let (c, _e, mut o, mut r, mut w) = setup(Config::default());
    let t = tokio::spawn(async move { c.abort().await });
    let v = next(&mut r).await;
    send(
        &mut w,
        json!({"type":"response","id":v["id"],"command":"abort_bash","success":true}),
    )
    .await;
    assert!(matches!(t.await.unwrap(), Err(Error::Protocol(_))));
    assert!(matches!(terminal(&mut o).await, Error::Protocol(_)));
}
#[tokio::test]
async fn event_count_and_byte_overflow_are_terminal_not_silent() {
    for config in [
        Config {
            max_event_count: 1,
            ..Default::default()
        },
        Config {
            max_event_bytes: 25,
            ..Default::default()
        },
    ] {
        let (c, mut e, mut o, mut r, mut w) = setup(config);
        let t = tokio::spawn(async move { c.abort().await });
        let _ = next(&mut r).await;
        w.write_all(b"{\"type\":\"agent_start\"}\n{\"type\":\"turn_start\"}\n")
            .await
            .unwrap();
        assert_eq!(terminal(&mut o).await, Error::EventOverflow);
        assert_eq!(t.await.unwrap(), Err(Error::EventOverflow));
        assert!(e.recv().await.unwrap().is_ok());
        assert_eq!(e.recv().await, Some(Err(Error::EventOverflow)));
        assert_eq!(e.recv().await, None);
    }
}
#[tokio::test]
async fn consuming_events_releases_byte_budget() {
    let (_c, mut e, o, _r, mut w) = setup(Config {
        max_event_count: 1,
        max_event_bytes: 25,
        ..Default::default()
    });
    for _ in 0..10 {
        send(&mut w, json!({"type":"agent_start"})).await;
        assert!(e.recv().await.unwrap().is_ok());
    }
    assert!(o.terminal_reason().is_none());
    drop(o);
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
#[tokio::test]
async fn unattended_dialogs_cancel_and_one_way_notifications_need_no_ack() {
    let (_c, mut e, o, mut r, mut w) = setup(Config::default());
    let dialogs = [
        json!({"method":"select","title":"pick","options":["yes","no"]}),
        json!({"method":"confirm","title":"danger","message":"approve?"}),
        json!({"method":"input","title":"input"}),
        json!({"method":"editor","title":"edit"}),
    ];
    for (i, mut v) in dialogs.into_iter().enumerate() {
        v["type"] = "extension_ui_request".into();
        v["id"] = format!("ui-{i}").into();
        v["extraField"] = json!({"extension":true});
        send(&mut w, v).await;
        let answer = next(&mut r).await;
        assert_eq!(
            answer,
            json!({"type":"extension_ui_response","id":format!("ui-{i}"),"cancelled":true})
        );
        let Event::Ui(u) = e.recv().await.unwrap().unwrap() else {
            panic!()
        };
        assert!(u.extra.contains_key("extraField"));
    }
    for mut v in [
        json!({"method":"notify","message":"notice"}),
        json!({"method":"setStatus","statusKey":"k"}),
        json!({"method":"setWidget","widgetKey":"k"}),
        json!({"method":"setTitle","title":"t"}),
        json!({"method":"set_editor_text","text":"t"}),
    ] {
        v["type"] = "extension_ui_request".into();
        v["id"] = "one-way".into();
        send(&mut w, v).await;
        assert!(matches!(e.recv().await.unwrap().unwrap(), Event::Ui(_)));
    }
    assert!(
        tokio::time::timeout(Duration::from_millis(20), r.next())
            .await
            .is_err()
    );
    drop(o);
}
#[tokio::test]
async fn manual_ui_response_uses_id_without_pending_slot_or_ack() {
    let (c, mut e, o, mut r, mut w) = setup(Config {
        ui_policy: pi_rpc::UiPolicy::Manual,
        max_pending_commands: 1,
        ..Default::default()
    });
    let a = c.clone();
    let pending = tokio::spawn(async move { a.abort().await });
    let cmd = next(&mut r).await;
    send(&mut w,json!({"type":"extension_ui_request","id":"ui-1","method":"confirm","title":"t","message":"m"})).await;
    let Event::Ui(u) = e.recv().await.unwrap().unwrap() else {
        panic!()
    };
    assert!(matches!(
        c.respond_ui(&u, UiAnswer::Value("bad".into())).await,
        Err(Error::Protocol(_))
    ));
    c.respond_ui(&u, UiAnswer::Confirmed(false)).await.unwrap();
    assert_eq!(
        next(&mut r).await,
        json!({"type":"extension_ui_response","id":"ui-1","confirmed":false})
    );
    reply(&mut w, &cmd, None).await;
    pending.await.unwrap().unwrap();
    drop(o);
}
#[tokio::test]
async fn stop_clear_then_abort_race_preserves_nonempty_queue_after_settled() {
    let (c, mut e, o, mut r, mut w) = setup(Config::default());
    let a = c.clone();
    let clear = tokio::spawn(async move { a.clear_queue().await });
    let cmd = next(&mut r).await;
    assert_eq!(cmd["type"], "clear_queue");
    reply(
        &mut w,
        &cmd,
        Some(json!({"steering":["removed"],"followUp":[]})),
    )
    .await;
    assert_eq!(clear.await.unwrap().unwrap().steering, vec!["removed"]);
    // An extension/in-flight submission can enqueue after clear_queue completes.
    send(
        &mut w,
        json!({"type":"queue_update","steering":[],"followUp":["raced"]}),
    )
    .await;
    let abort = tokio::spawn(async move { c.abort().await });
    let cmd = next(&mut r).await;
    assert_eq!(cmd["type"], "abort");
    send(
        &mut w,
        json!({"type":"agent_end","messages":[],"willRetry":false}),
    )
    .await;
    send(&mut w, json!({"type":"agent_settled","aborted":false})).await;
    reply(&mut w, &cmd, None).await;
    abort.await.unwrap().unwrap();
    assert!(
        matches!(e.recv().await.unwrap().unwrap(),Event::Known(KnownEvent::QueueUpdate{follow_up,..}) if follow_up==vec!["raced"])
    );
    assert!(matches!(
        e.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::AgentEnd { .. })
    ));
    assert_eq!(
        e.recv().await.unwrap().unwrap(),
        Event::Known(KnownEvent::AgentSettled { aborted: false })
    );
    drop(o);
}
#[tokio::test]
async fn bounded_writer_bytes_and_count_under_stalled_peer() {
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
            let a = c.clone();
            let t = tokio::spawn(async move { a.abort().await });
            tokio::task::yield_now().await;
            Some(t)
        } else {
            None
        };
        assert_eq!(c.abort().await, Err(Error::Capacity(expected)));
        drop(o);
        assert_eq!(first.await.unwrap(), Err(Error::Closed));
        if let Some(t) = second {
            assert_eq!(t.await.unwrap(), Err(Error::Closed));
        }
    }
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
