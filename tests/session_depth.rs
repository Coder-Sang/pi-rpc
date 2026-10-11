#![cfg(feature = "session_file")]
use pi_rpc::{
    Error,
    protocol::{EntryData, Extra, Message, SessionEntry},
    session_file::{SessionFile, SessionFileBuilder, SessionLimits},
};
use serde_json::{Value, json};
fn message(arrays: usize) -> Message {
    let mut value = Value::Null;
    for _ in 0..arrays {
        value = Value::Array(vec![value]);
    }
    let mut fields = Extra::new();
    fields.insert("content".into(), json!("test"));
    fields.insert("timestamp".into(), json!(1));
    fields.insert("opaque".into(), value);
    Message {
        role: "user".into(),
        fields,
    }
}
#[test]
fn builder_accepts_512_and_rejects_513_complete_record_depth() {
    let mut builder = SessionFileBuilder::new("s", "t", "/p").unwrap();
    assert!(
        matches!(builder.append_message("a",None,"t",message(511)),Err(Error::Protocol(s)) if s.contains("nesting depth"))
    );
    builder
        .append_message("a", None, "t", message(510))
        .unwrap();
    let file = builder.finish().unwrap();
    file.validate().unwrap();
    let wire = file.encode(SessionLimits::default()).unwrap();
    assert_eq!(
        SessionFile::parse(&wire, SessionLimits::default()).unwrap(),
        file
    );
}
#[test]
fn mutable_append_and_existing_file_admission_use_complete_record_depth() {
    let mut file = SessionFile::new("s", "t", "/p");
    assert!(file.append_message("a", None, "t", message(511)).is_err());
    assert!(file.entries.is_empty());
    file.append_message("a", None, "t", message(510)).unwrap();
    let valid = file.clone();
    if let EntryData::Message { message: original } = &mut file.entries[0].data {
        *original = message(511);
    }
    assert!(matches!(file.validate(),Err(Error::Protocol(s)) if s.contains("nesting depth")));
    assert!(file.encode(SessionLimits::default()).is_err());
    assert!(SessionFileBuilder::from_session_file(file).is_err());
    assert_eq!(
        SessionFileBuilder::from_session_file(valid.clone())
            .unwrap()
            .finish()
            .unwrap(),
        valid
    );
}
#[test]
fn header_and_nonmessage_entry_have_one_root_container() {
    let mut file = SessionFile::new("s", "t", "/p");
    let mut value = Value::Null;
    for _ in 0..511 {
        value = Value::Array(vec![value]);
    }
    file.header.extra.insert("opaque".into(), value.clone());
    let mut extra = Extra::new();
    extra.insert("opaque".into(), value);
    file.entries.push(SessionEntry {
        id: "a".into(),
        parent_id: None,
        timestamp: "t".into(),
        data: EntryData::Custom {
            custom_type: "test".into(),
            extra,
        },
    });
    let wire = file.encode(SessionLimits::default()).unwrap();
    assert_eq!(
        SessionFile::parse(&wire, SessionLimits::default()).unwrap(),
        file
    );
}
