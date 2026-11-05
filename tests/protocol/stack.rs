use super::*;

#[test]
fn boundary_parse_convert_clone_serialize_and_drop_on_two_mib_stack() {
    const CHILD: &str = "PI_RPC_VERSIONED_STACK_CHILD";
    if std::env::var_os(CHILD).is_some() {
        std::thread::Builder::new().stack_size(2 * 1024 * 1024).spawn(|| {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let nested = format!("{}null{}", "[".repeat(511), "]".repeat(511));
                let unknown = format!("{{\"type\":\"future_event\",\"payload\":{nested}}}\n");
                let known = format!("{{\"type\":\"tool_execution_start\",\"toolCallId\":\"c\",\"toolName\":\"t\",\"args\":{nested}}}\n");
                for wire in [unknown, known] {
                    let (value, _) = pi_rpc::codec::JsonlReader::new(wire.as_bytes(), 65536).next().await.unwrap().unwrap();
                    let record = decode_record(value).unwrap();
                    let cloned = record.clone();
                    match &cloned {
                        Record::Event(Event::Known(event)) => { pi_rpc::codec::encode(event, 65536).unwrap(); serde_json::to_vec(event).unwrap(); }
                        Record::Event(Event::Unknown(value)) => { pi_rpc::codec::encode(value, 65536).unwrap(); serde_json::to_vec(value).unwrap(); }
                        _ => panic!("wrong boundary event"),
                    }
                    drop((record, cloned));
                }
                let bad = format!("{{\"type\":\"tool_execution_start\",\"args\":{nested}}}\n");
                let (value, _) = pi_rpc::codec::JsonlReader::new(bad.as_bytes(), 65536).next().await.unwrap().unwrap();
                assert!(matches!(decode_record(value), Err(Error::Protocol(_))));
                let excessive = format!("{{\"type\":\"future_event\",\"payload\":[{nested}]}}\n");
                assert!(matches!(pi_rpc::codec::JsonlReader::new(excessive.as_bytes(), 65536).next().await, Err(Error::Protocol(_))));
            });
        }).unwrap().join().unwrap();
        return;
    }
    let output = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["--exact", STACK_TEST, "--nocapture"])
        .env(CHILD, "1")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "stack child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("1 passed"),
        "stack filter executed no test"
    );
}
