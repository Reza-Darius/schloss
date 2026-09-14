/* a simple futex lock */
#![allow(dead_code)]

use std::{
    cell::UnsafeCell, ffi::c_uint, marker::PhantomPinned, ops::{Deref, DerefMut}, pin::Pin, sync::atomic::{
        AtomicU32, Ordering::{AcqRel, Relaxed},
    },
};

const LOCK_MASK: u32 = 1 << 31;

pub struct FutexLock<T> {
    inner: Pin<Box<LockInner<T>>>,
}

// futexes need a stable address for the futex word so this type is !Unpin
struct LockInner<T> {
    // 1 = locked
    fword: AtomicU32,
    data: UnsafeCell<T>,
    _boo: PhantomPinned,
}

unsafe impl<T: Send> Send for FutexLock<T> {}
unsafe impl<T: Send> Sync for FutexLock<T> {}

pub struct FutexGuard<'a, T> {
    lock: &'a FutexLock<T>,
}

impl<T> DerefMut for FutexGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &mut *self.lock.inner.data.get() }
    }
}

impl<T> Deref for FutexGuard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &*self.lock.inner.data.get() }
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
            inner: Box::pin(LockInner {
                fword: 0.into(),
                data: UnsafeCell::new(value),
                _boo: PhantomPinned,
            }),
        }
    }

    pub fn lock(&self) -> FutexGuard<'_, T> {
        // if the lock bit is 0 we got the lock
        if self.inner.fword.fetch_or(LOCK_MASK, AcqRel) & LOCK_MASK == 0 {
            return FutexGuard { lock: self };
        }
        self.inner.fword.fetch_add(1, Relaxed);

        loop {
            if self.inner.fword.fetch_or(LOCK_MASK, AcqRel) & LOCK_MASK == 0 {
                self.inner.fword.fetch_sub(1, Relaxed);
                return FutexGuard { lock: self };
            }
            let exp = self.inner.fword.load(Relaxed);
            if exp & LOCK_MASK == 0 {
                continue;
            }
            futex_wait(&self.inner.fword, exp);
        }
    }

    fn unlock(&self) {
        // if no thread is waiting we return
        if self.inner.fword.fetch_and(!LOCK_MASK, AcqRel) & !LOCK_MASK == 0 {
            return;
        };
        futex_wake(&self.inner.fword, 1);
    }
}

/// tests if the futex word == expected, if yes, puts the thread to sleep
fn futex_wait(fword: &AtomicU32, expected: u32) {
    unsafe {
        let rc = libc::syscall(
            libc::SYS_futex,
            fword.as_ptr() as *const c_uint,
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
            fword.as_ptr() as *const c_uint,
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
