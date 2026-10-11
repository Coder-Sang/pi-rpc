#![cfg(feature = "session_file")]
use pi_rpc::{
    protocol::{ContentBlock, EntryData, Extra, MessageContent, SessionEntry},
    session_file::{SessionFile, SessionFileBuilder, SessionLimits, user_message},
};
use serde_json::json;
fn file() -> SessionFile {
    SessionFile::new("s", "t", "/p")
}
#[test]
fn all_header_reserved_fields_are_rejected_before_export_or_builder_admission() {
    for key in ["type", "version", "id", "timestamp", "cwd", "parentSession"] {
        let mut file = file();
        file.header.extra.insert(key.into(), json!(2));
        assert!(file.validate().is_err(), "{key}");
        assert!(file.encode(SessionLimits::default()).is_err(), "{key}");
        assert!(
            SessionFileBuilder::from_session_file(file).is_err(),
            "{key}"
        );
    }
}
#[test]
fn role_collision_rejects_both_append_apis_without_reserving_id() {
    let mut builder = SessionFileBuilder::new("s", "t", "/p").unwrap();
    let mut file = file();
    let good = || user_message(MessageContent::Text("test".into()), 1).unwrap();
    let mut bad = good();
    bad.fields.insert("role".into(), json!("system"));
    assert!(builder.append_message("a", None, "t", bad.clone()).is_err());
    assert!(file.append_message("a", None, "t", bad).is_err());
    builder.append_message("a", None, "t", good()).unwrap();
    file.append_message("a", None, "t", good()).unwrap();
    assert_eq!(builder.finish().unwrap(), file);
}
#[test]
fn entry_envelope_and_variant_fields_cannot_be_shadowed() {
    for (template, keys) in [
        (
            json!({"type":"custom", "customType":"plugin"}),
            vec!["type", "id", "parentId", "timestamp", "customType"],
        ),
        (
            json!({"type":"compaction", "summary":"s", "firstKeptEntryId":"a", "tokensBefore":1}),
            vec!["summary", "firstKeptEntryId", "tokensBefore"],
        ),
        (
            json!({"type":"branch_summary", "fromId":"root", "summary":"s"}),
            vec!["fromId", "summary"],
        ),
        (
            json!({"type":"custom_message", "customType":"plugin", "content":"test", "display":true}),
            vec!["customType", "content", "display"],
        ),
        (
            json!({"type":"usage", "kind":"k", "provider":"p", "model":"m", "usage":{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}}),
            vec!["kind", "provider", "model", "usage"],
        ),
    ] {
        for key in keys {
            let mut data: EntryData = serde_json::from_value(template.clone()).unwrap();
            match &mut data {
                EntryData::Custom { extra, .. }
                | EntryData::Compaction { extra, .. }
                | EntryData::BranchSummary { extra, .. }
                | EntryData::CustomMessage { extra, .. }
                | EntryData::Usage { extra, .. } => {
                    extra.insert(key.into(), json!(null));
                }
                _ => unreachable!(),
            }
            let mut file = file();
            file.entries.push(SessionEntry {
                id: "a".into(),
                parent_id: None,
                timestamp: "t".into(),
                data,
            });
            assert!(file.encode(SessionLimits::default()).is_err(), "{key}");
        }
    }
}
#[test]
fn nested_content_and_usage_collisions_are_rejected() {
    for key in ["type", "text", "textSignature"] {
        let mut extra = Extra::new();
        extra.insert(key.into(), json!(null));
        let content = MessageContent::Blocks(vec![ContentBlock::Text {
            text: "test".into(),
            text_signature: None,
            extra,
        }]);
        assert!(user_message(content.clone(), 1).is_err());
        let mut file = file();
        file.entries.push(SessionEntry {
            id: "a".into(),
            parent_id: None,
            timestamp: "t".into(),
            data: EntryData::CustomMessage {
                custom_type: "plugin".into(),
                content,
                display: true,
                extra: Extra::new(),
            },
        });
        assert!(file.validate().is_err());
    }
    let mut data: EntryData = serde_json::from_value(json!({"type":"usage", "kind":"k", "provider":"p", "model":"m", "usage":{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}})).unwrap();
    if let EntryData::Usage { usage, .. } = &mut data {
        usage.cost.extra.insert("total".into(), json!(2));
    }
    let mut file = file();
    file.entries.push(SessionEntry {
        id: "a".into(),
        parent_id: None,
        timestamp: "t".into(),
        data,
    });
    assert!(file.validate().is_err());
}
#[test]
fn nonreserved_metadata_roundtrips_unchanged() {
    let mut file = file();
    file.header
        .extra
        .insert("plugin".into(), json!({"version":2}));
    let mut extra = Extra::new();
    extra.insert("providerMetadata".into(), json!({"role":"opaque"}));
    file.entries.push(SessionEntry {
        id: "a".into(),
        parent_id: None,
        timestamp: "t".into(),
        data: EntryData::Custom {
            custom_type: "plugin".into(),
            extra,
        },
    });
    let file = SessionFileBuilder::from_session_file(file)
        .unwrap()
        .finish()
        .unwrap();
    let encoded = file.encode(SessionLimits::default()).unwrap();
    assert_eq!(
        SessionFile::parse(&encoded, SessionLimits::default()).unwrap(),
        file
    );
}

#[test]
fn each_content_block_rejects_its_own_reserved_fields() {
    for (template, keys) in [
        (
            json!({"type":"thinking","thinking":"test"}),
            vec!["type", "thinking", "thinkingSignature", "redacted"],
        ),
        (
            json!({"type":"image","data":"","mimeType":"image/png"}),
            vec!["type", "data", "mimeType"],
        ),
        (
            json!({"type":"toolCall","id":"i","name":"tool","arguments":{}}),
            vec!["type", "id", "name", "arguments"],
        ),
    ] {
        for key in keys {
            let mut block: ContentBlock = serde_json::from_value(template.clone()).unwrap();
            match &mut block {
                ContentBlock::Thinking { extra, .. }
                | ContentBlock::Image { extra, .. }
                | ContentBlock::ToolCall { extra, .. } => {
                    extra.insert(key.into(), json!(null));
                }
                _ => unreachable!(),
            }
            let content = MessageContent::Blocks(vec![block]);
            assert!(user_message(content.clone(), 1).is_err(), "{key}");
            let mut file = file();
            file.append_message(
                "root",
                None,
                "t",
                user_message(MessageContent::Text("test".into()), 1).unwrap(),
            )
            .unwrap();
            file.entries.push(SessionEntry {
                id: "edit".into(),
                parent_id: Some("root".into()),
                timestamp: "t".into(),
                data: EntryData::ContextEdit {
                    target_id: "root".into(),
                    replacement: Some(pi_rpc::protocol::ContextReplacement { content }),
                },
            });
            assert!(file.validate().is_err(), "{key}");
        }
    }
}

#[test]
fn each_usage_and_cost_reserved_field_is_rejected() {
    for (cost, keys) in [
        (
            false,
            vec![
                "input",
                "output",
                "cacheRead",
                "cacheWrite",
                "totalTokens",
                "cost",
            ],
        ),
        (
            true,
            vec!["input", "output", "cacheRead", "cacheWrite", "total"],
        ),
    ] {
        for key in keys {
            let mut data: EntryData = serde_json::from_value(json!({"type":"usage", "kind":"k", "provider":"p", "model":"m", "usage":{"input":1,"output":1,"cacheRead":0,"cacheWrite":0,"totalTokens":2,"cost":{"input":0,"output":0,"cacheRead":0,"cacheWrite":0,"total":0}}})).unwrap();
            if let EntryData::Usage { usage, .. } = &mut data {
                let extra = if cost {
                    &mut usage.cost.extra
                } else {
                    &mut usage.extra
                };
                extra.insert(key.into(), json!(null));
            }
            let mut file = file();
            file.entries.push(SessionEntry {
                id: "a".into(),
                parent_id: None,
                timestamp: "t".into(),
                data,
            });
            assert!(file.validate().is_err(), "{key}");
        }
    }
}
