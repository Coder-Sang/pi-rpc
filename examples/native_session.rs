use pi_rpc::{
    protocol::{Cost, Message, MessageContent, Usage},
    session_file::{SessionFile, SessionFileBuilder, SessionLimits, user_message},
};
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut builder =
        SessionFileBuilder::new("example-native", "2026-10-02T12:00:00.000Z", "/project")?;
    builder.append_message(
        "entry-1",
        None,
        "2026-10-02T12:00:00.000Z",
        user_message(
            MessageContent::Text("Native stored user message".into()),
            1790942400000,
        )?,
    )?;
    // Illustrative values only: the SDK does not count tokens or calculate fees.
    let usage = Usage {
        input: 100.0,
        output: 20.0,
        cache_read: 0.0,
        cache_write: 0.0,
        total_tokens: 120.0,
        cost: Cost {
            input: 0.0,
            output: 0.0,
            cache_read: 0.0,
            cache_write: 0.0,
            total: 0.0,
            extra: Default::default(),
        },
        extra: Default::default(),
    };
    let assistant: Message = serde_json::from_value(serde_json::json!({
        "role": "assistant", "content": [{"type": "text", "text": "Stored reply"}],
        "api": "example-api", "provider": "example-provider", "model": "example-model",
        "usage": usage, "stopReason": "stop", "timestamp": 1790942401000_u64,
    }))?;
    builder.append_message(
        "entry-2",
        Some("entry-1".into()),
        "2026-10-02T12:00:01.000Z",
        assistant,
    )?;
    let file = builder.finish()?;
    let bytes = file.encode(SessionLimits::default())?;
    assert_eq!(SessionFile::parse(&bytes, SessionLimits::default())?, file);
    // Remote/local delivery and --session selection are the host's responsibility.
    Ok(())
}
