/* a simple spin lock */
#![allow(dead_code)]

use std::{
    cell::UnsafeCell,
    hint::spin_loop,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

pub struct Lock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for Lock<T> where T: Send {}
unsafe impl<T> Sync for Lock<T> where T: Sync {}

pub struct Guard<'a, T> {
    lock: &'a Lock<T>,
}

impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &mut *self.lock.data.get() }
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: only one thread can hold the guard
        unsafe { &*self.lock.data.get() }
    }
}

impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.lock.unlock();
    }
}

impl<T> Lock<T> {
    pub fn new(value: T) -> Self {
        Lock {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(value),
        }
    }

    pub fn lock(&self) -> Guard<'_, T> {
        while self
            .locked
            .compare_exchange(false, true, Relaxed, Relaxed)
            .is_err()
        {
            spin_loop();
        }
        Guard { lock: self }
    }

    fn unlock(&self) {
        self.locked.store(false, Relaxed);
    }
}

#[cfg(test)]
mod test {
    use std::thread;

    use super::*;

    #[test]
    fn spin_lock1() {
        const N_COUNT: u32 = 10000;
        const N_THREADS: u32 = 20;
        const N_ITERATTIONS: u32 = 100;

        for _ in 0..N_ITERATTIONS {
            let counter = Lock::new(0);

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
