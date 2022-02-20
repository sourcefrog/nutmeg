
#![cfg(windows)]
   
use std::sync::atomic::{AtomicBool, Ordering};

    static WINDOWS_TRIED: AtomicBool = AtomicBool::new(false);
    static WINDOWS_SUCCEEDED: AtomicBool = AtomicBool::new(false);

    pub(crate) fn enable_windows_ansi() -> bool {
        if WINDOWS_TRIED.load(Ordering::SeqCst) {
            WINDOWS_SUCCEEDED.load(Ordering::SeqCst)
        } else {
            let succeeded = yansi::Paint::enable_windows_ascii();
            WINDOWS_TRIED.store(true, Ordering::SeqCst);
            WINDOWS_SUCCEEDED.store(succeeded, Ordering::SeqCst);
            succeeded
        }
    }
