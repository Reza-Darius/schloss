use std::{ffi::c_uint, sync::atomic::AtomicU32};

/// tests if the futex word == expected, if yes, puts the thread to sleep
pub fn futex_wait(fword: &AtomicU32, expected: u32) {
    unsafe {
        loop {
            let rc = libc::syscall(
                libc::SYS_futex,
                fword as *const AtomicU32,
                libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG,
                expected as c_uint,
                0,
            );
            if rc == -1 {
                let err = std::io::Error::last_os_error();
                match err.raw_os_error().unwrap() {
                    libc::EINTR => continue,
                    libc::EWOULDBLOCK => return,
                    _ => panic!("futex error {err}"),
                }
            } else {
                return;
            }
        }
    }
}

// wakes n waker fow the futex word
pub fn futex_wake(fword: &AtomicU32, nwaker: u32) {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_futex,
            fword as *const AtomicU32,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            nwaker as libc::c_uint,
        );

        if rc == -1 {
            panic!("futex wake: {}", std::io::Error::last_os_error());
        }
    }
}

// wakes n waker fow the futex word
pub fn futex_wake_one(fword: &AtomicU32) {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_futex,
            fword as *const AtomicU32,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            1,
        );

        if rc == -1 {
            panic!("futex wake: {}", std::io::Error::last_os_error());
        }
    }
}
