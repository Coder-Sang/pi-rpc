use super::*;

#[tokio::test]
async fn fractional_dialog_timeouts_keep_responses_and_events_readable() {
    for ui_policy in [pi_rpc::UiPolicy::Cancel, pi_rpc::UiPolicy::Manual] {
        let (client, mut events, mut owner, mut reader, mut writer) =
            super::streaming::setup_fragmented(Config {
                ui_policy,
                ..Config::default()
            });
        let command = tokio::spawn(async move { client.abort().await });
        let request = next(&mut reader).await;
        let dialogs = [
            json!({"type":"extension_ui_request","id":"select","method":"select","title":"t","options":["ok"],"timeout":1500.5}),
            json!({"type":"extension_ui_request","id":"confirm","method":"confirm","title":"t","message":"m","timeout":0.75}),
            json!({"type":"extension_ui_request","id":"input","method":"input","title":"t","timeout":42.9}),
        ];
        let mut batch = Vec::new();
        for dialog in &dialogs {
            batch.extend(codec::encode(dialog, 65536).unwrap());
        }
        batch.extend(
            codec::encode(
                &json!({"type":"response","id":request["id"],"command":"abort","success":true}),
                65536,
            )
            .unwrap(),
        );
        batch.extend_from_slice(b"{\"type\":\"agent_start\"}\r\n");
        // Split decimals, escapes and JSONL boundaries without scheduler assumptions.
        for chunk in batch.chunks(3) {
            writer.write_all(chunk).await.unwrap();
        }
        tokio::time::timeout(Duration::from_secs(2), command)
            .await
            .unwrap()
            .unwrap()
            .unwrap();
        for (dialog, expected) in dialogs.iter().zip([1500, 0, 42]) {
            let Some(Ok(Event::Ui(request))) = bounded_event(&mut events).await else {
                panic!("expected UI event");
            };
            assert_eq!(request.id, dialog["id"]);
            let timeout = match request.method {
                UiMethod::Select { timeout, .. }
                | UiMethod::Confirm { timeout, .. }
                | UiMethod::Input { timeout, .. } => timeout,
                _ => panic!("expected timed dialog"),
            };
            assert_eq!(timeout, Some(expected));
            if ui_policy == pi_rpc::UiPolicy::Cancel {
                assert_eq!(
                    next(&mut reader).await,
                    json!({"type":"extension_ui_response","id":dialog["id"],"cancelled":true})
                );
            }
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
