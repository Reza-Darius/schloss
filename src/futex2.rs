/*
* a simple futex lock
* capable to sleeping idle threads and waking them up
*/

#![allow(dead_code)]

use std::{
    cell::UnsafeCell,
    ffi::c_uint,
    ops::{Deref, DerefMut},
    sync::atomic::{
        AtomicU32,
        Ordering::{AcqRel, Acquire, Relaxed},
    },
};

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const CONTENDED: u32 = 2;

pub struct FutexLock<T> {
    data: UnsafeCell<T>,
    inner: Box<LockInner>,
}

// futexes need a stable address so we box it
struct LockInner {
    // MSB indicates the locked state, with 1 denoting an acquired lock
    fword: AtomicU32,
}

unsafe impl<T: Send> Send for FutexLock<T> {}
unsafe impl<T: Send> Sync for FutexLock<T> {}

pub struct FutexGuard<'a, T> {
    lock: &'a FutexLock<T>,
}

impl<T> DerefMut for FutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Deref for FutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for FutexGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

impl<T> FutexLock<T> {
    pub fn new(value: T) -> Self {
        FutexLock {
            inner: Box::new(LockInner { fword: 0.into() }),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> FutexGuard<'_, T> {
        let fw = &self.inner.fword;

        if fw
            .compare_exchange(UNLOCKED, LOCKED, Relaxed, Relaxed)
            .is_err()
        {
            while fw.swap(CONTENDED, Relaxed) != UNLOCKED {
                futex_wait(fw, CONTENDED);
            }
        }
        FutexGuard { lock: self }
    }

    fn unlock(&self) {
        // we only wake in the contended case
        if self.inner.fword.swap(UNLOCKED, Relaxed) == CONTENDED {
            futex_wake(&self.inner.fword, 1);
        };
    }
}

/// tests if the futex word == expected, if yes, puts the thread to sleep
fn futex_wait(fword: &AtomicU32, expected: u32) {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_futex,
            fword as *const AtomicU32,
            libc::FUTEX_WAIT | libc::FUTEX_PRIVATE_FLAG,
            expected as c_uint,
            0,
        );
        if rc == -1 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                panic!("futex error {err}");
            }
        }
    }
}

// wakes n waker fow the futex word
fn futex_wake(fword: &AtomicU32, nwaker: u32) {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_futex,
            fword as *const AtomicU32,
            libc::FUTEX_WAKE | libc::FUTEX_PRIVATE_FLAG,
            nwaker as c_uint,
        );
        if rc == -1 {
            let err = std::io::Error::last_os_error();
            if err.raw_os_error() != Some(libc::EWOULDBLOCK) {
                panic!("futex error {err}");
            }
        }
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use super::*;

    #[test]
    fn futex_lock() {
        const N_COUNT: u32 = 10000;
        const N_THREADS: u32 = 20;
        const N_ITERATTIONS: u32 = 1000;

        for _ in 0..N_ITERATTIONS {
            let counter = FutexLock::new(0);

            thread::scope(|s| {
                for _ in 0..N_THREADS {
                    s.spawn(|| {
                        for _ in 0..N_COUNT / N_THREADS {
                            let mut guard = counter.lock();
                            let n = *guard;
                            *guard = n + 1;
                        }
                    });
                }
            });
            assert_eq!(N_COUNT, *counter.lock());
        }
    }
}
