use super::*;

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
