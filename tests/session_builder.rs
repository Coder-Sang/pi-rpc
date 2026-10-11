#![cfg(feature = "session_file")]
use pi_rpc::{
    Error,
    protocol::{EntryData, Extra, Message, MessageContent, SessionEntry},
    session_file::{SessionFile, SessionFileBuilder, SessionLimits, user_message},
};
const ISO: &str = "2026-10-11T00:00:00Z";
fn message(text: &str) -> Message {
    user_message(MessageContent::Text(text.into()), 1).unwrap()
}

#[test]
fn builder_matches_existing_api_bytes_and_branches() {
    let mut expected = SessionFile::new("s", ISO, "/p");
    let mut builder = SessionFileBuilder::new("s", ISO, "/p").unwrap();
    for (id, parent) in [
        ("a", None),
        ("b", Some("a")),
        ("c", Some("a")),
        ("root2", None),
    ] {
        let parent = parent.map(str::to_owned);
        expected
            .append_message(id, parent.clone(), ISO, message(id))
            .unwrap();
        builder
            .append_message(id, parent, ISO, message(id))
            .unwrap();
    }
    let file = builder.finish().unwrap();
    assert_eq!(file, expected);
    assert_eq!(
        file.encode(SessionLimits::default()).unwrap(),
        expected.encode(SessionLimits::default()).unwrap()
    );
    assert_eq!(
        file.branch(Some("c"))
            .unwrap()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "c"]
    );
}

#[test]
fn failed_appends_do_not_reserve_ids_or_change_history() {
    let mut builder = SessionFileBuilder::new("s", ISO, "/p").unwrap();
    builder
        .append_message("a", None, ISO, message("first"))
        .unwrap();
    for (id, parent, timestamp, msg) in [
        ("a", None, ISO, message("duplicate")),
        ("", None, ISO, message("empty id")),
        ("bad", Some("missing".into()), ISO, message("orphan")),
        ("bad", Some("bad".into()), ISO, message("self parent")),
        ("bad", None, "", message("timestamp")),
        (
            "bad",
            None,
            ISO,
            Message {
                role: "assistant".into(),
                fields: Extra::new(),
            },
        ),
    ] {
        assert!(matches!(
            builder.append_message(id, parent, timestamp, msg),
            Err(Error::Protocol(_))
        ));
    }
    // A failed ID may neither become a parent nor prevent a later valid append.
    assert!(
        builder
            .append_message("child", Some("bad".into()), ISO, message("child"))
            .is_err()
    );
    builder
        .append_message("bad", Some("a".into()), ISO, message("valid"))
        .unwrap();
    builder
        .append_message("child", Some("bad".into()), ISO, message("valid child"))
        .unwrap();
    let file = builder.finish().unwrap();
    assert_eq!(file.entries.len(), 3);
    assert_eq!(
        file.entries[0].data,
        EntryData::Message {
            message: message("first")
        }
    );
}

#[test]
fn existing_native_records_and_header_metadata_are_preserved() {
    let mut file = SessionFile::new("s", ISO, "/p");
    file.header.parent_session = Some("parent.jsonl".into());
    file.header
        .extra
        .insert("opaque".into(), serde_json::json!({"native":true}));
    file.append_message("a", None, ISO, message("stored"))
        .unwrap();
    file.entries.push(SessionEntry {
        id: "checkpoint".into(),
        parent_id: Some("a".into()),
        timestamp: ISO.into(),
        data: EntryData::Compaction {
            summary: "summary".into(),
            first_kept_entry_id: "a".into(),
            tokens_before: 100.0,
            extra: Extra::new(),
        },
    });
    let mut expected = file.clone();
    expected
        .append_message(
            "b",
            Some("checkpoint".into()),
            ISO,
            message("after checkpoint"),
        )
        .unwrap();
    let mut builder = SessionFileBuilder::from_session_file(file).unwrap();
    assert!(
        builder
            .append_message("a", None, ISO, message("duplicate existing"))
            .is_err()
    );
    builder
        .append_message(
            "b",
            Some("checkpoint".into()),
            ISO,
            message("after checkpoint"),
        )
        .unwrap();
    assert_eq!(builder.finish().unwrap(), expected);
}

#[test]
fn constructors_reject_invalid_headers_and_existing_history() {
    for (id, timestamp, cwd) in [("", ISO, "/p"), ("s", "", "/p"), ("s", ISO, "")] {
        assert!(SessionFileBuilder::new(id, timestamp, cwd).is_err());
    }
    let mut file = SessionFile::new("s", ISO, "/p");
    file.append_message("a", None, ISO, message("stored"))
        .unwrap();
    file.entries[0].parent_id = Some("missing".into());
    assert!(SessionFileBuilder::from_session_file(file).is_err());
    let mut file = SessionFile::new("s", ISO, "/p");
    file.header.version = 2;
    assert!(SessionFileBuilder::from_session_file(file).is_err());
}

#[test]
fn old_mutable_api_still_detects_corruption_and_rolls_back() {
    let mut file = SessionFile::new("s", ISO, "/p");
    file.append_message("a", None, ISO, message("stored"))
        .unwrap();
    file.entries[0].timestamp.clear();
    assert!(
        file.append_message("b", Some("a".into()), ISO, message("new"))
            .is_err()
    );
    assert_eq!(file.entries.len(), 1);
}

#[test]
fn bulk_builder_handles_ten_thousand_messages_with_native_roundtrip() {
    let mut builder = SessionFileBuilder::new("s", ISO, "/p").unwrap();
    for i in 0..10000 {
        builder
            .append_message(
                i.to_string(),
                if i == 0 {
                    None
                } else {
                    Some((i - 1).to_string())
                },
                ISO,
                message("native"),
            )
            .unwrap();
    }
    let file = builder.finish().unwrap();
    assert_eq!(file.entries.len(), 10000);
    let bytes = file.encode(SessionLimits::default()).unwrap();
    assert_eq!(
        SessionFile::parse(&bytes, SessionLimits::default()).unwrap(),
        file
    );
}

#[test]
fn long_mixed_history_validates_and_roundtrips() {
    let mut file = SessionFile::new("mixed", ISO, "/p");
    file.append_message("0", None, ISO, message("root"))
        .unwrap();
    for i in 1..10_000 {
        let data = if i % 2 == 0 {
            EntryData::Compaction {
                summary: "s".into(),
                first_kept_entry_id: "0".into(),
                tokens_before: 1.0,
                extra: Extra::new(),
            }
        } else {
            EntryData::ContextEdit {
                target_id: "0".into(),
                replacement: None,
            }
        };
        file.entries.push(SessionEntry {
            id: i.to_string(),
            parent_id: Some((i - 1).to_string()),
            timestamp: ISO.into(),
            data,
        });
    }
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
