use pi_rpc::{Error, codec};
use serde::{Serialize, ser::SerializeSeq};
use std::sync::atomic::{AtomicUsize, Ordering};

#[test]
fn byte_budget_stops_streaming_serialization() {
    struct Generated<'a>(&'a AtomicUsize);
    impl Serialize for Generated<'_> {
        fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
            let mut seq = serializer.serialize_seq(Some(500_000))?;
            for _ in 0..500_000 {
                self.0.fetch_add(1, Ordering::Relaxed);
                seq.serialize_element(&true)?;
            }
            seq.end()
        }
    }
    let count = AtomicUsize::new(0);
    assert_eq!(
        codec::encode(&Generated(&count), 32),
        Err(Error::Capacity("record bytes"))
    );
    assert!(count.load(Ordering::Relaxed) < 10);
}

#[test]
fn exact_budget_excludes_lf_and_handles_escaped_strings() {
    let value = "a\\\"\nb";
    let expected = serde_json::to_vec(value).unwrap();
    assert_eq!(
        codec::encode(&value, expected.len() - 1),
        Err(Error::Capacity("record bytes"))
    );
    let mut framed = expected.clone();
    framed.push(b'\n');
    assert_eq!(codec::encode(&value, expected.len()).unwrap(), framed);
    assert_eq!(
        codec::encode(&true, 0),
        Err(Error::Capacity("record bytes"))
    );
    assert_eq!(codec::encode(&true, usize::MAX).unwrap(), b"true\n");
}

#[test]
fn serializer_failure_remains_protocol_error() {
    struct Invalid;
    impl Serialize for Invalid {
        fn serialize<S: serde::Serializer>(&self, _: S) -> Result<S::Ok, S::Error> {
            Err(serde::ser::Error::custom("invalid"))
        }
    }
    assert!(matches!(
        codec::encode(&Invalid, 32),
        Err(Error::Protocol(_))
    ));
}
