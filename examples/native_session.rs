use pi_rpc::{
    protocol::MessageContent,
    session_file::{SessionFile, SessionLimits, user_message},
};
fn main() -> pi_rpc::Result<()> {
    let mut file = SessionFile::new("example-native", "2026-10-02T12:00:00.000Z", "/project");
    file.append_message(
        "entry-1",
        None,
        "2026-10-02T12:00:00.000Z",
        user_message(
            MessageContent::Text("Native stored user message".into()),
            1790942400000,
        )?,
    )?;
    let bytes = file.encode(SessionLimits::default())?;
    assert_eq!(SessionFile::parse(&bytes, SessionLimits::default())?, file);
    // Remote/local delivery and --session selection are the host's responsibility.
    Ok(())
}
