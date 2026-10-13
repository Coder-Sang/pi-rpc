use super::*;

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
