use super::*;

#[test]
fn select_choices_and_one_way_ui_responses_are_checked() {
    let u: UiRequest = serde_json::from_value(
        json!({"id":"u","method":"select","title":"t","options":["allow","deny"]}),
    )
    .unwrap();
    assert!(UiAnswer::Value("other".into()).record(&u).is_err());
    assert_eq!(
        UiAnswer::Value("deny".into()).record(&u).unwrap()["id"],
        "u"
    );
    let u: UiRequest =
        serde_json::from_value(json!({"id":"n","method":"notify","message":"x"})).unwrap();
    assert!(UiAnswer::Cancel.record(&u).is_err());
}
