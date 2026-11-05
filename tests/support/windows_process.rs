use std::ffi::c_void;

const SYNCHRONIZE: u32 = 0x0010_0000;
const WAIT_OBJECT_0: u32 = 0;
const WAIT_TIMEOUT: u32 = 258;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
    fn WaitForSingleObject(handle: *mut c_void, milliseconds: u32) -> u32;
    fn CloseHandle(handle: *mut c_void) -> i32;
}

pub struct Process(*mut c_void);
impl Process {
    pub fn open(pid: u32) -> Self {
        // SAFETY: requests only observation rights for the ready child.
        let handle = unsafe { OpenProcess(SYNCHRONIZE, 0, pid) };
        assert!(
            !handle.is_null(),
            "OpenProcess: {}",
            std::io::Error::last_os_error()
        );
        Self(handle)
    }
    pub fn assert_alive(&self) {
        // SAFETY: this object retains a valid handle until Drop.
        assert_eq!(unsafe { WaitForSingleObject(self.0, 0) }, WAIT_TIMEOUT);
    }
    pub async fn wait_terminated(&self) {
        tokio::time::timeout(super::Duration::from_secs(3), async {
            loop {
                // A retained handle observes the original child, not a
                // later process that happens to reuse its PID.
                match unsafe { WaitForSingleObject(self.0, 0) } {
                    WAIT_OBJECT_0 => break,
                    WAIT_TIMEOUT => tokio::time::sleep(super::Duration::from_millis(10)).await,
                    result => panic!("WaitForSingleObject failed: {result:#x}"),
                }
            }
        })
        .await
        .unwrap();
    }
}
impl Drop for Process {
    fn drop(&mut self) {
        // SAFETY: uniquely owned handle, closed exactly once.
        assert_ne!(unsafe { CloseHandle(self.0) }, 0);
    }
}
