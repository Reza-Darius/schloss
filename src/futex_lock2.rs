/*
* a simple futex lock based on: https://www.akkadia.org/drepper/futex.pdf
*/

use crate::futex::*;
use std::{
    cell::UnsafeCell,
    ops::{Deref, DerefMut},
    sync::atomic::{
        AtomicU32,
        Ordering::{Acquire, Relaxed, Release},
    },
};

const UNLOCKED: u32 = 0;
const LOCKED: u32 = 1;
const CONTENDED: u32 = 2;

pub struct FutexLock<T> {
    data: UnsafeCell<T>,
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
            data: UnsafeCell::new(value),
            fword: 0.into(),
        }
    }

    pub fn lock(&self) -> FutexGuard<'_, T> {
        // The Linux implementation of std::sync::Mutex in the Rust standard library, at least the one in Rust 1.66.0, uses a spin count of 100.
        const MAX_SPIN: u8 = 100;

        let fw = &self.fword;
        let mut spin_count = 0;

        while self.fword.load(Relaxed) == 1 && spin_count < MAX_SPIN {
            spin_count += 1;
            std::hint::spin_loop();
        }

        if fw
            .compare_exchange(UNLOCKED, LOCKED, Acquire, Relaxed)
            .is_err()
        {
            while fw.swap(CONTENDED, Acquire) != UNLOCKED {
                futex_wait(fw, CONTENDED);
            }
        }
        FutexGuard { lock: self }
    }

    fn unlock(&self) {
        // we only wake in the contended case
        if self.fword.swap(UNLOCKED, Release) == CONTENDED {
            futex_wake(&self.fword, 1);
        };
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use super::*;

    #[test]
    fn futex_lock2() {
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
                            *guard += 1;
                        }
                    });
                }
            });
            assert_eq!(N_COUNT, *counter.lock());
        }
    }
}
