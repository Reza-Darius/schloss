/* a simple spin lock */
#![allow(dead_code)]

use std::{
    cell::UnsafeCell,
    hint::spin_loop,
    sync::atomic::{AtomicBool, Ordering::Relaxed},
};

pub struct Lock<T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
}

unsafe impl<T> Send for Lock<T> {}
unsafe impl<T> Sync for Lock<T> {}

pub struct Guard<'a, T> {
    lock: &'a Lock<T>,
}

impl<T> Guard<'_, T> {
    pub fn get(&mut self) -> &mut T {
        // SAFETY: the guard is only ever held by one thread
        unsafe { &mut *self.lock.data.get() }
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
                            let n = *guard.get();
                            *guard.get() = n + 1;
                        }
                    });
                }
            });
            assert_eq!(N_COUNT, *counter.lock().get());
        }
    }
}
