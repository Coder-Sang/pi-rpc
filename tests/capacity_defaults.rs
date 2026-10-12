use pi_rpc::Config;

#[test]
fn image_capacity_defaults_and_explicit_limits() {
    let defaults = Config::default();
    assert_eq!(defaults.max_record_bytes, 32 * 1024 * 1024);
    assert_eq!(defaults.max_writer_bytes, 64 * 1024 * 1024);
    assert_eq!(defaults.max_event_bytes, 128 * 1024 * 1024);
    let explicit = Config {
        max_record_bytes: 4096,
        ..defaults
    };
    assert_eq!(explicit.max_record_bytes, 4096);
    #[cfg(feature = "session_file")]
    {
        let native = pi_rpc::session_file::SessionLimits::default();
        assert_eq!(native.max_record_bytes, 32 * 1024 * 1024);
        assert_eq!(native.max_file_bytes, 64 * 1024 * 1024);
    }
}
