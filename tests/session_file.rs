#![cfg(feature = "session_file")]
use pi_rpc::{Error, protocol::*, session_file::*};
use serde_json::{Value, json};
const ISO: &str = "2026-10-02T00:00:00.000Z";
#[test]
fn native_session_message_durations_are_preserved_and_strictly_validated() {
    let usage = json!({"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":0,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}});
    for base in [
        json!({"role":"assistant","content":[],"api":"open","provider":"azure","model":"m","usage":usage,"stopReason":"stop","timestamp":1,"opaque":true}),
        json!({"role":"toolResult","content":[],"toolCallId":"t","toolName":"open","isError":false,"timestamp":1,"details":{"durationMs":"opaque"}}),
    ] {
        for duration in [None, Some(json!(0)), Some(json!(8.25))] {
            let mut message = base.clone();
            if let Some(duration) = duration {
                message["durationMs"] = duration;
            }
            let mut file = SessionFile::new("s", ISO, "/p");
            add(
                &mut file,
                "e",
                None,
                json!({"type":"message","message":message.clone()}),
            );
            let bytes = file.encode(SessionLimits::default()).unwrap();
            let parsed = SessionFile::parse(&bytes, SessionLimits::default()).unwrap();
            let EntryData::Message { message: decoded } = &parsed.entries[0].data else {
                panic!("expected message")
            };
            assert_eq!(serde_json::to_value(decoded).unwrap(), message);
            assert_eq!(parsed.encode(SessionLimits::default()).unwrap(), bytes);
        }
        for bad in [Value::Null, json!("8"), json!(false), json!([]), json!({})] {
            let mut message = base.clone();
            message["durationMs"] = bad;
            let file = SessionFile::new("s", ISO, "/p");
            let mut bytes = file.encode(SessionLimits::default()).unwrap();
            bytes.extend(pi_rpc::codec::encode(&json!({"type":"message","id":"e","parentId":null,"timestamp":ISO,"message":message}), 4096).unwrap());
            assert!(matches!(
                SessionFile::parse(&bytes, SessionLimits::default()),
                Err(Error::Protocol(_))
            ));
        }
    }
}
#[test]
fn native_parser_rejects_absent_parent_and_context_replacement_but_accepts_null() {
    let mut f = SessionFile::new("s", ISO, "/p");
    f.append_message(
        "a",
        None,
        ISO,
        user_message(MessageContent::Text("native".into()), 1).unwrap(),
    )
    .unwrap();
    add(
        &mut f,
        "edit",
        Some("a"),
        json!({"type":"context_edit","targetId":"a","replacement":null}),
    );
    let original = f.encode(SessionLimits::default()).unwrap();
    assert!(SessionFile::parse(&original, SessionLimits::default()).is_ok());
    for (index, key) in [(1, "parentId"), (2, "replacement")] {
        let mut records: Vec<Value> = original
            .split(|b| *b == b'\n')
            .filter(|b| !b.is_empty())
            .map(|b| serde_json::from_slice(b).unwrap())
            .collect();
        records[index].as_object_mut().unwrap().remove(key);
        let mut bytes = Vec::new();
        for v in records {
            bytes.extend(pi_rpc::codec::encode(&v, 4096).unwrap());
        }
        assert!(matches!(
            SessionFile::parse(&bytes, SessionLimits::default()),
            Err(Error::Protocol(_))
        ));
    }
}
fn add(file: &mut SessionFile, id: &str, parent: Option<&str>, data: Value) {
    let mut v = data;
    v["id"] = id.into();
    v["parentId"] = parent.map(Value::from).unwrap_or(Value::Null);
    v["timestamp"] = ISO.into();
    file.entries.push(serde_json::from_value(v).unwrap());
}
#[test]
fn native_session_all_entries_round_trip_with_system_and_opaque_metadata() {
    let mut f = SessionFile::new("session-id", ISO, "/project");
    f.header.parent_session = Some("parent.jsonl".into());
    add(
        &mut f,
        "system",
        None,
        json!({"type":"message","message":{"role":"system","content":"base","sections":{"skills":"<skills>","remove":null},"toolsAdded":[{"name":"tool","description":"d","parameters":{"open":true}}],"timestamp":123}}),
    );
    f.append_message(
        "user",
        Some("system".into()),
        ISO,
        user_message(MessageContent::Text("not a prompt request".into()), 123).unwrap(),
    )
    .unwrap();
    add(
        &mut f,
        "model",
        Some("user"),
        json!({"type":"model_change","provider":"native","modelId":"virtual"}),
    );
    add(
        &mut f,
        "thinking",
        Some("model"),
        json!({"type":"thinking_level_change","thinkingLevel":"max"}),
    );
    let usage = json!({"input":1,"output":2,"cacheRead":3,"cacheWrite":4,"totalTokens":10,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}});
    add(
        &mut f,
        "usage",
        Some("thinking"),
        json!({"type":"usage","kind":"future-operation","provider":"p","model":"m","usage":usage,"note":"native"}),
    );
    add(
        &mut f,
        "compact",
        Some("usage"),
        json!({"type":"compaction","summary":"s","firstKeptEntryId":"user","tokensBefore":100,"systemMessage":{"role":"system","content":"checkpoint","timestamp":123},"details":{"extension":[1,2]},"usage":usage}),
    );
    add(
        &mut f,
        "branch",
        Some("compact"),
        json!({"type":"branch_summary","fromId":"usage","summary":"s","details":{"opaque":true}}),
    );
    add(
        &mut f,
        "custom",
        Some("branch"),
        json!({"type":"custom","customType":"state","data":{"anything":true}}),
    );
    add(
        &mut f,
        "custom-message",
        Some("custom"),
        json!({"type":"custom_message","customType":"visible","content":"native","display":true,"details":{"anything":true}}),
    );
    add(
        &mut f,
        "edit",
        Some("custom-message"),
        json!({"type":"context_edit","targetId":"user","replacement":{"content":"replacement"}}),
    );
    add(
        &mut f,
        "label",
        Some("edit"),
        json!({"type":"label","targetId":"user","label":"checkpoint"}),
    );
    add(
        &mut f,
        "info",
        Some("label"),
        json!({"type":"session_info","name":"native session"}),
    );
    let bytes = f.encode(SessionLimits::default()).unwrap();
    let parsed = SessionFile::parse(&bytes, SessionLimits::default()).unwrap();
    assert_eq!(parsed, f);
    assert_eq!(parsed.branch(Some("info")).unwrap().len(), 12);
    let EntryData::Message { message } = &parsed.entries[0].data else {
        panic!()
    };
    assert_eq!(message.fields["sections"]["remove"], Value::Null);
}
#[test]
fn branching_multiple_roots_and_native_context_references() {
    let mut f = SessionFile::new("s", ISO, "/p");
    for (id, parent) in [
        ("a", None),
        ("b", Some("a")),
        ("c", Some("a")),
        ("root2", None),
    ] {
        f.append_message(
            id,
            parent.map(str::to_owned),
            ISO,
            user_message(MessageContent::Text(id.into()), 1).unwrap(),
        )
        .unwrap();
    }
    assert_eq!(
        f.branch(Some("c"))
            .unwrap()
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "c"]
    );
    assert!(f.branch(None).unwrap().is_empty());
    add(
        &mut f,
        "omit",
        Some("c"),
        json!({"type":"context_edit","targetId":"a","replacement":null}),
    );
    f.validate().unwrap();
    add(
        &mut f,
        "retain-none",
        Some("omit"),
        json!({"type":"compaction","summary":"s","firstKeptEntryId":"retain-none","tokensBefore":10}),
    );
    f.validate().unwrap();
    add(
        &mut f,
        "bad-edit",
        Some("retain-none"),
        json!({"type":"context_edit","targetId":"b","replacement":null}),
    );
    assert!(f.validate().is_err());
}
#[test]
fn reject_legacy_duplicates_orphans_cycles_and_malformed_native_messages() {
    let f = SessionFile::new("s", ISO, "/p");
    let bytes = f.encode(SessionLimits::default()).unwrap();
    for version in [1, 2, 4] {
        let mut header = serde_json::to_value(&f.header).unwrap();
        header["version"] = version.into();
        let bytes = pi_rpc::codec::encode(&header, 4096).unwrap();
        assert!(SessionFile::parse(&bytes, SessionLimits::default()).is_err());
    }
    assert!(SessionFile::parse(&bytes[..bytes.len() - 1], SessionLimits::default()).is_err());
    let mut f = f;
    f.append_message(
        "a",
        None,
        ISO,
        user_message(MessageContent::Text("x".into()), 1).unwrap(),
    )
    .unwrap();
    assert!(
        f.append_message(
            "a",
            None,
            ISO,
            user_message(MessageContent::Text("x".into()), 1).unwrap()
        )
        .is_err()
    );
    assert_eq!(f.entries.len(), 1);
    assert!(
        f.append_message(
            "b",
            Some("missing".into()),
            ISO,
            user_message(MessageContent::Text("x".into()), 1).unwrap()
        )
        .is_err()
    );
    let invalid = Message {
        role: "assistant".into(),
        fields: Extra::new(),
    };
    assert!(
        f.append_message("b", Some("a".into()), ISO, invalid)
            .is_err()
    );
    f.entries[0].parent_id = Some("a".into());
    assert!(f.validate().is_err());
    assert!(f.branch(Some("a")).is_err());
}
#[test]
fn native_file_record_and_entry_limits_and_crlf() {
    let mut f = SessionFile::new("s", ISO, "/p");
    f.append_message(
        "a",
        None,
        ISO,
        user_message(MessageContent::Text("x\u{2028}y".into()), 1).unwrap(),
    )
    .unwrap();
    let bytes = f.encode(SessionLimits::default()).unwrap();
    let crlf = String::from_utf8(bytes.clone())
        .unwrap()
        .replace('\n', "\r\n");
    assert_eq!(
        SessionFile::parse(crlf.as_bytes(), SessionLimits::default()).unwrap(),
        f
    );
    for limits in [
        SessionLimits {
            max_file_bytes: 10,
            ..Default::default()
        },
        SessionLimits {
            max_record_bytes: 10,
            ..Default::default()
        },
        SessionLimits {
            max_entries: 0,
            ..Default::default()
        },
    ] {
        assert!(matches!(f.encode(limits), Err(Error::Capacity(_))));
        assert!(matches!(
            SessionFile::parse(&bytes, limits),
            Err(Error::Capacity(_))
        ));
    }
}

#[test]
fn pi_1_3_tool_argument_error_round_trips_in_native_history() {
    let message = json!({"role":"assistant","content":[{"type":"toolCall","id":"call","name":"read",
        "arguments":{},"argumentsError":"Complete tool arguments were not a JSON object"}],
        "api":"synthetic","provider":"synthetic","model":"synthetic","timestamp":1,"stopReason":"toolUse",
        "usage":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"totalTokens":0,
        "cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}});
    let mut file = SessionFile::new("s", ISO, "/p");
    add(
        &mut file,
        "e",
        None,
        json!({"type":"message","message":message.clone()}),
    );
    let bytes = file.encode(SessionLimits::default()).unwrap();
    let parsed = SessionFile::parse(&bytes, SessionLimits::default()).unwrap();
    let EntryData::Message { message: decoded } = &parsed.entries[0].data else {
        panic!("expected message")
    };
    assert_eq!(serde_json::to_value(decoded).unwrap(), message);
    assert_eq!(parsed.encode(SessionLimits::default()).unwrap(), bytes);
}

#[path = "support/recovery.rs"]
mod recovery;

#[test]
fn pi_1_4_native_recovery_edits_keep_original_messages_and_large_summary() {
    let summary = "summary 中文\n".repeat(8192);
    let entries = recovery::entries(&summary);
    let mut file = SessionFile::new("recovery", "2026-10-27T00:00:00.000Z", "/project");
    file.entries = entries
        .iter()
        .cloned()
        .map(|entry| serde_json::from_value(entry).unwrap())
        .collect();
    let encoded = file.encode(SessionLimits::default()).unwrap();
    let parsed = SessionFile::parse(&encoded, SessionLimits::default()).unwrap();
    assert_eq!(parsed.entries.len(), 6);
    assert_eq!(
        serde_json::to_value(&parsed.entries).unwrap(),
        json!(entries)
    );
    for (index, target) in [(3, "attempt"), (4, "result")] {
        let EntryData::ContextEdit {
            target_id,
            replacement,
        } = &parsed.entries[index].data
        else {
            panic!("expected persisted omission");
        };
        assert_eq!(target_id, target);
        assert!(replacement.is_none());
    }
    let EntryData::Compaction {
        summary: decoded,
        extra,
        ..
    } = &parsed.entries[5].data
    else {
        panic!("expected compaction entry");
    };
    assert_eq!(decoded, &summary);
    assert_eq!(extra["details"], entries[5]["details"]);
    assert_eq!(extra["usage"], entries[5]["usage"]);
    assert_eq!(parsed.encode(SessionLimits::default()).unwrap(), encoded);
}
