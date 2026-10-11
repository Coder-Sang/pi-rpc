#![cfg(feature = "session_file")]
use pi_rpc::{
    Error,
    protocol::{EntryData, Extra, SessionEntry},
    session_file::{SessionFile, SessionLimits},
};
use serde_json::Value;

#[test]
fn budgets_reject_before_full_semantic_validation() {
    let mut file = SessionFile::new("s", "t", "/p");
    let mut nested = Value::Null;
    for _ in 0..512 {
        nested = Value::Array(vec![nested]);
    }
    file.header.extra.insert("opaque".into(), nested);
    assert!(matches!(file.validate(), Err(Error::Protocol(_))));
    assert_eq!(
        file.encode(SessionLimits {
            max_record_bytes: 32,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("record bytes"))
    );
    assert_eq!(
        file.encode(SessionLimits {
            max_file_bytes: 32,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("session file bytes"))
    );
    file.entries.push(SessionEntry {
        id: "".into(),
        parent_id: None,
        timestamp: "".into(),
        data: EntryData::Custom {
            custom_type: "test".into(),
            extra: Extra::new(),
        },
    });
    assert_eq!(
        file.encode(SessionLimits {
            max_entries: 0,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("session entries"))
    );
}

#[test]
fn file_budget_includes_every_lf_and_bounds_each_remaining_record() {
    let mut file = SessionFile::new("s", "t", "/p");
    let header = file.encode(SessionLimits::default()).unwrap();
    let mut extra = Extra::new();
    extra.insert("opaque".into(), Value::String("x".repeat(1024 * 1024)));
    file.entries.push(SessionEntry {
        id: "a".into(),
        parent_id: None,
        timestamp: "t".into(),
        data: EntryData::Custom {
            custom_type: "test".into(),
            extra,
        },
    });
    assert_eq!(
        file.encode(SessionLimits {
            max_file_bytes: header.len() + 32,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("session file bytes"))
    );
    let full = file.encode(SessionLimits::default()).unwrap();
    assert_eq!(
        file.encode(SessionLimits {
            max_file_bytes: full.len(),
            ..SessionLimits::default()
        })
        .unwrap(),
        full
    );
    assert_eq!(
        file.encode(SessionLimits {
            max_file_bytes: full.len() - 1,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("session file bytes"))
    );
    assert_eq!(
        file.encode(SessionLimits {
            max_file_bytes: 0,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("session file bytes"))
    );
    assert_eq!(
        file.encode(SessionLimits {
            max_record_bytes: 0,
            ..SessionLimits::default()
        }),
        Err(Error::Capacity("record bytes"))
    );
}

#[test]
fn admitted_invalid_metadata_is_still_rejected_without_returning_bytes() {
    let mut file = SessionFile::new("s", "t", "/p");
    file.header.extra.insert("version".into(), Value::from(2));
    assert!(matches!(
        file.encode(SessionLimits::default()),
        Err(Error::Protocol(_))
    ));
}
