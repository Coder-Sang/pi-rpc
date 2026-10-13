use super::*;

fn dialog(method: &str) -> Value {
    json!({"type":"extension_ui_request","id":"u","method":method,"title":"t","message":"m","options":["ok"],"extensionField":true})
}

fn timeout(request: &UiRequest) -> Option<u64> {
    match request.method {
        UiMethod::Select { timeout, .. }
        | UiMethod::Confirm { timeout, .. }
        | UiMethod::Input { timeout, .. } => timeout,
        _ => panic!("expected timed dialog"),
    }
}

#[test]
fn dialog_timeouts_accept_fractional_milliseconds_without_losing_integer_precision() {
    for method in ["select", "confirm", "input"] {
        for (number, expected) in [
            ("0", 0),
            ("-0.0", 0),
            ("0.75", 0),
            ("1500.5", 1500),
            ("1.5e3", 1500),
            ("9007199254740993", 9_007_199_254_740_993),
            ("18446744073709551615", u64::MAX),
            ("18446744073709549568.0", u64::MAX - 2047),
        ] {
            let mut wire = dialog(method);
            wire["timeout"] = serde_json::from_str(number).unwrap();
            let request: UiRequest = serde_json::from_value(wire.clone()).unwrap();
            assert_eq!(timeout(&request), Some(expected), "{method}: {number}");
            assert_eq!(request.extra["extensionField"], true);
            // UiRequest's existing nested flatten keeps the original method
            // fields in extra, including the unrounded wire timeout.
            assert_eq!(request.extra["timeout"], wire["timeout"]);
            assert_eq!(
                serde_json::to_value(&request.method).unwrap()["timeout"],
                expected
            );
            let Record::Event(Event::Ui(request)) = decode_record(wire).unwrap() else {
                panic!("expected UI event");
            };
            assert_eq!(timeout(&request), Some(expected));
        }
        for present in [false, true] {
            let mut wire = dialog(method);
            if present {
                wire["timeout"] = Value::Null;
            }
            let Record::Event(Event::Ui(request)) = decode_record(wire).unwrap() else {
                panic!("expected UI event");
            };
            assert_eq!(timeout(&request), None);
        }
    }
}

#[test]
fn dialog_timeouts_reject_negative_out_of_range_and_non_number_values() {
    for method in ["select", "confirm", "input"] {
        for bad in [
            json!(-0.25),
            json!(-1),
            json!("1500.5"),
            json!(true),
            json!([]),
            json!({}),
            serde_json::from_str("18446744073709551616").unwrap(),
        ] {
            let mut wire = dialog(method);
            wire["timeout"] = bad;
            assert!(serde_json::from_value::<UiRequest>(wire.clone()).is_err());
            assert!(matches!(decode_record(wire), Err(Error::Protocol(_))));
        }
    }
}

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
