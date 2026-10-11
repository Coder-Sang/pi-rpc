#![cfg(feature = "session_file")]
use pi_rpc::{
    Error,
    protocol::{EntryData, Extra, SessionEntry, Usage},
    session_file::{SessionFile, SessionFileBuilder, SessionLimits},
};
use serde_json::json;
fn file(field: usize, value: f64) -> SessionFile {
    let data = if field == 0 {
        EntryData::Compaction {
            summary: "s".into(),
            first_kept_entry_id: "a".into(),
            tokens_before: value,
            extra: Extra::new(),
        }
    } else {
        let mut usage:Usage=serde_json::from_value(json!({"input":1,"output":1,"cacheRead":1,"cacheWrite":1,"totalTokens":4,"cost":{"input":1,"output":1,"cacheRead":1,"cacheWrite":1,"total":4}})).unwrap();
        let target = match field {
            1 => &mut usage.input,
            2 => &mut usage.output,
            3 => &mut usage.cache_read,
            4 => &mut usage.cache_write,
            5 => &mut usage.total_tokens,
            6 => &mut usage.cost.input,
            7 => &mut usage.cost.output,
            8 => &mut usage.cost.cache_read,
            9 => &mut usage.cost.cache_write,
            10 => &mut usage.cost.total,
            _ => unreachable!(),
        };
        *target = value;
        EntryData::Usage {
            kind: "k".into(),
            provider: "p".into(),
            model: "m".into(),
            usage,
            extra: Extra::new(),
        }
    };
    let mut file = SessionFile::new("s", "t", "/p");
    file.entries.push(SessionEntry {
        id: "a".into(),
        parent_id: None,
        timestamp: "t".into(),
        data,
    });
    file
}
#[test]
fn every_typed_float_rejects_nan_and_both_infinities() {
    for field in 0..=10 {
        for number in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let file = file(field, number);
            assert!(
                matches!(file.validate(),Err(Error::Protocol(s)) if s.contains("non-finite")),
                "field={field}"
            );
            assert!(
                matches!(file.encode(SessionLimits::default()),Err(Error::Protocol(s)) if s.contains("non-finite")),
                "field={field}"
            );
            assert!(
                SessionFileBuilder::from_session_file(file).is_err(),
                "field={field}"
            );
        }
    }
}
#[test]
fn finite_numbers_roundtrip_without_normalization() {
    for field in 0..=10 {
        for number in [
            0.0,
            -0.0,
            1.5,
            -1.5,
            1e100,
            f64::MAX,
            -f64::MAX,
            f64::MIN_POSITIVE,
        ] {
            let file = file(field, number);
            let file = SessionFileBuilder::from_session_file(file)
                .unwrap()
                .finish()
                .unwrap();
            let bytes = file.encode(SessionLimits::default()).unwrap();
            assert_eq!(
                SessionFile::parse(&bytes, SessionLimits::default()).unwrap(),
                file
            );
        }
    }
}
