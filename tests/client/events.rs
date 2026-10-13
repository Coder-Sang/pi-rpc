use super::*;

#[tokio::test]
async fn fractional_retry_delays_keep_pending_commands_and_following_events_alive() {
    let (client, mut events, mut owner, mut reader, mut writer) =
        super::streaming::setup_fragmented(Config::default());
    let command = tokio::spawn(async move { client.abort().await });
    let request = next(&mut reader).await;
    // Pi's retryDelayMs returns a fractional maxAgentDelayMs cap unchanged.
    let retries = [
        json!({"type":"auto_retry_start","attempt":1,"maxAttempts":3,"delayMs":1500.5,"errorMessage":"synthetic"}),
        json!({"type":"summarization_retry_scheduled","attempt":1,"maxAttempts":3,"delayMs":0.75,"errorMessage":"synthetic"}),
    ];
    let mut batch = Vec::new();
    for retry in &retries {
        batch.extend(codec::encode(retry, 65536).unwrap());
    }
    batch.extend(
        codec::encode(
            &json!({"type":"response","id":request["id"],"command":"abort","success":true}),
            65536,
        )
        .unwrap(),
    );
    batch.extend_from_slice(b"{\"type\":\"agent_start\"}\r\n");
    for chunk in batch.chunks(3) {
        writer.write_all(chunk).await.unwrap();
    }
    tokio::time::timeout(Duration::from_secs(2), command)
        .await
        .unwrap()
        .unwrap()
        .unwrap();
    for (kind, expected) in [
        ("auto_retry_start", 1500),
        ("summarization_retry_scheduled", 0),
    ] {
        let Some(Ok(Event::Known(event))) = bounded_event(&mut events).await else {
            panic!("expected retry event");
        };
        let delay = match &event {
            KnownEvent::AutoRetryStart { delay_ms, .. }
            | KnownEvent::SummarizationRetryScheduled { delay_ms, .. } => *delay_ms,
            _ => panic!("expected retry delay"),
        };
        assert_eq!(delay, expected);
        assert_eq!(serde_json::to_value(event).unwrap()["type"], kind);
    }
    assert_eq!(
        bounded_event(&mut events).await,
        Some(Ok(Event::Known(KnownEvent::AgentStart)))
    );
    assert!(owner.terminal_reason().is_none());
    writer.shutdown().await.unwrap();
    assert_eq!(bounded_event(&mut events).await, Some(Err(Error::Eof)));
    assert_eq!(bounded_event(&mut events).await, None);
    assert_eq!(owner.wait_terminal().await, Error::Eof);
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
