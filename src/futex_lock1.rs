/*
* a simple futex lock
* capable to sleeping idle threads and waking them up
*/

#![allow(dead_code)]

use crate::futex::{futex_wait, futex_wake};
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{
        AtomicU32,
        Ordering::{AcqRel, Relaxed},
    },
};

const LOCK_MASK: u32 = 1 << 31;

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
        let l = fw.fetch_or(LOCK_MASK, AcqRel);
        // fast path: if the lock bit is 0 we got the lock
        if l & LOCK_MASK == 0 {
            return FutexGuard { lock: self };
        }

        // increment the thread wait count
        fw.fetch_add(1, Relaxed);
        let mut exp = l + 1;
        while exp & LOCK_MASK != 0 {
            futex_wait(fw, exp);
            exp = fw.fetch_or(LOCK_MASK, AcqRel);
        }

        // decrement the thread wait count
        fw.fetch_sub(1, Relaxed);

        FutexGuard { lock: self }
    }

    fn unlock(&self) {
        // clear the lock bit and check for waiting threads
        // we can avoid the system call in the uncontended case
        if self.inner.fword.fetch_and(!LOCK_MASK, AcqRel) & !LOCK_MASK != 0 {
            println!("waking");

            futex_wake(&self.inner.fword, 1);
        };
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
