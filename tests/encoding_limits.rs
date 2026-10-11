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

#[test]
fn generated_depth_is_bounded_on_small_stack_in_isolated_process() {
    const CHILD: &str = "PI_RPC_GENERATED_DEPTH_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
            struct Generated<'a>(usize, &'a AtomicUsize, bool);
            impl Serialize for Generated<'_> {
                fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
                    self.1.fetch_add(1, Ordering::Relaxed);
                    if self.0 == 0 { return serializer.serialize_unit(); }
                    if self.2 {
                        use serde::ser::SerializeStruct;
                        let mut object = serializer.serialize_struct("Generated", 1)?;
                        object.serialize_field("child", &Generated(self.0 - 1, self.1, true))?;
                        object.end()
                    } else {
                        let mut seq = serializer.serialize_seq(Some(1))?;
                        seq.serialize_element(&Generated(self.0 - 1, self.1, false))?;
                        seq.end()
                    }
                }
            }
            let count = AtomicUsize::new(0);
            for object in [false, true] {
                let bytes = codec::encode(&Generated(512, &count, object), 4 * 1024 * 1024).unwrap();
                assert_eq!(bytes.len(), if object { 5125 } else { 1029 });
                for depth in [513, 50_000] {
                    count.store(0, Ordering::Relaxed);
                    assert!(matches!(codec::encode(&Generated(depth, &count, object), 4 * 1024 * 1024), Err(Error::Protocol(s)) if s.contains("nesting depth")));
                    assert_eq!(count.load(Ordering::Relaxed), 513);
                }
            }
        }).unwrap().join().unwrap();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "generated_depth_is_bounded_on_small_stack_in_isolated_process",
            "--nocapture",
        ])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "generated depth child failed: {}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn streaming_depth_ignores_brackets_and_fragmented_escapes_in_strings() {
    let text = "[{}]\\\"\n".repeat(1000);
    let value = serde_json::json!({"text":text,"more":[{"x":"\\"}]});
    let mut expected = serde_json::to_vec(&value).unwrap();
    expected.push(b'\n');
    assert_eq!(codec::encode(&value, 65536).unwrap(), expected);
}

#[tokio::test]
async fn jsonl_tool_duration_preserves_float_bits_through_typed_decoding() {
    use pi_rpc::protocol::{Event, KnownEvent, Record, decode_record};
    for bits in [
        0x3ff9_4868_b3b0_4fc6,
        0x20fd_5db4_3a77_6693,
        1,
        0x000f_ffff_ffff_ffff,
        (-0.0f64).to_bits(),
    ] {
        let number = f64::from_bits(bits);
        let wire = format!(
            "{{\"type\":\"tool_execution_end\",\"toolCallId\":\"t\",\"toolName\":\"tool\",\"result\":null,\"isError\":false,\"durationMs\":{number:?}}}\n"
        );
        let (value, _) = codec::JsonlReader::new(wire.as_bytes(), 65536)
            .next()
            .await
            .unwrap()
            .unwrap();
        let Record::Event(Event::Known(event)) = decode_record(value).unwrap() else {
            panic!("expected tool event");
        };
        let KnownEvent::ToolExecutionEnd { duration_ms, .. } = &event else {
            panic!("expected tool execution end");
        };
        assert_eq!(duration_ms.unwrap().to_bits(), bits);
        let encoded = codec::encode(&event, 65536).unwrap();
        let (value, _) = codec::JsonlReader::new(encoded.as_slice(), 65536)
            .next()
            .await
            .unwrap()
            .unwrap();
        let Record::Event(Event::Known(KnownEvent::ToolExecutionEnd { duration_ms, .. })) =
            decode_record(value).unwrap()
        else {
            panic!("expected reencoded tool event");
        };
        assert_eq!(duration_ms.unwrap().to_bits(), bits);
    }
}
