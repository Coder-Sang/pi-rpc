use std::path::{Path, PathBuf};

pub(super) const VERSIONS: [&str; 7] = [
    "1.1.0", "1.1.10", "1.2.0", "1.3.0", "1.3.1", "1.3.10", "1.4.0",
];

pub(super) struct Fixture {
    _root: tempfile::TempDir,
    executable: PathBuf,
}

impl Fixture {
    pub(super) fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let executable = root
            .path()
            .join(format!("acceptance-probe{}", std::env::consts::EXE_SUFFIX));
        let source =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/acceptance_probe.rs");
        let output = std::process::Command::new("rustc")
            .arg("--edition=2024")
            .arg(source)
            .arg("-o")
            .arg(&executable)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        Self {
            _root: root,
            executable,
        }
    }

    pub(super) fn probe(&self, version: &str) -> Probe {
        let root = tempfile::tempdir().unwrap();
        let executable = root
            .path()
            .join(format!("acceptance-probe{}", std::env::consts::EXE_SUFFIX));
        // Keep Linux aliases immutable, as in the existing subprocess fixtures:
        // fork can inherit a concurrent copy's writable fd and cause ETXTBSY.
        #[cfg(target_os = "linux")]
        std::fs::hard_link(&self.executable, &executable).unwrap();
        #[cfg(not(target_os = "linux"))]
        std::fs::copy(&self.executable, &executable).unwrap();
        std::fs::write(executable.with_extension("version"), version).unwrap();
        Probe {
            _root: root,
            executable,
        }
    }
}

pub(super) struct Probe {
    _root: tempfile::TempDir,
    executable: PathBuf,
}

impl Probe {
    pub(super) fn executable(&self) -> &Path {
        &self.executable
    }

    #[track_caller]
    pub(super) fn assert_outcome(&self, result: pi_rpc::Result<()>, expected: &str) {
        let actual = std::fs::read_to_string(self.executable.with_extension("version")).unwrap();
        assert_eq!(
            std::fs::read(self.executable.with_extension("probe-started")).unwrap(),
            b"probe"
        );
        if actual == expected {
            assert_eq!(
                result,
                Err(pi_rpc::Error::ProcessExit(Some(37))),
                "expected={expected} actual={actual}"
            );
            assert_eq!(
                std::fs::read(self.executable.with_extension("rpc-started")).unwrap(),
                b"rpc"
            );
        } else {
            assert_eq!(
                result,
                Err(pi_rpc::Error::Version),
                "expected={expected} actual={actual}"
            );
            assert!(
                !self.executable.with_extension("rpc-started").exists(),
                "mismatched version started RPC"
            );
        }
    }
}
