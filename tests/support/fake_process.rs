use std::{path::PathBuf, sync::OnceLock};
pub(crate) fn fixture() -> PathBuf {
    static BINARY: OnceLock<(tempfile::TempDir, PathBuf)> = OnceLock::new();
    BINARY
        .get_or_init(|| {
            let dir = tempfile::tempdir().unwrap();
            let binary = dir
                .path()
                .join(format!("fake-pi{}", std::env::consts::EXE_SUFFIX));
            let status = std::process::Command::new("rustc")
                .args(["--edition=2024", "tests/fixtures/fake_pi.rs", "-o"])
                .arg(&binary)
                .status()
                .unwrap();
            assert!(status.success());
            (dir, binary)
        })
        .1
        .clone()
}
pub(crate) fn named_fixture(path: &std::path::Path) -> PathBuf {
    // Reuse immutable executable bytes. Linux uses hard links to avoid
    // inherited writable-copy fds. macOS uses symlinks to one stable source
    // vnode: hard-link aliases can trigger APFS/Gatekeeper path rejection.
    // The macOS case label lives in the owned directory, not the executable.
    #[cfg(target_os = "macos")]
    let path = {
        let mut label = path.file_name().unwrap().to_os_string();
        label.push(".case");
        let directory = path.with_file_name(label);
        std::fs::create_dir(&directory).unwrap();
        directory.join(fixture().file_name().unwrap())
    };
    #[cfg(not(target_os = "macos"))]
    let path = path.to_path_buf();
    #[cfg(all(unix, not(target_os = "macos")))]
    std::fs::hard_link(fixture(), &path).unwrap();
    #[cfg(target_os = "macos")]
    std::os::unix::fs::symlink(fixture(), &path).unwrap();
    #[cfg(not(unix))]
    std::fs::copy(fixture(), &path).unwrap();
    path
}
