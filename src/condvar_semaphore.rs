/*
* a simple Semaphore with RAII semantics
*/

#![allow(dead_code)]
use std::sync::{Arc, Condvar, Mutex};

#[derive(Debug, Clone)]
pub struct Semaphore {
    inner: Arc<InnerSem>,
}

#[derive(Debug)]
struct InnerSem {
    cv: Condvar,
    max: i32,
    lock: Mutex<i32>,
}

impl InnerSem {
    /// decrements the semaphore and waits if permits <= 0
    fn wait(&self) {
        let mut guard = self.lock.lock().unwrap();
        while *guard <= 0 {
            guard = self.cv.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    /// increments the semaphore and wakes a potential waiter
    fn post(&self) {
        let mut guard = self.lock.lock().unwrap();
        if *guard < self.max {
            *guard += 1;
        }
        self.cv.notify_one();
    }
}

// a RAII guard which puts back the permit into the semaphore on drop
#[derive(Debug)]
pub struct Permit<'a> {
    sem: &'a InnerSem,
}

impl Permit<'_> {
    /// drops the guard without putting the permit back into the semaphore
    pub fn forget(self) {
        std::mem::forget(self);
    }
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.sem.post();
    }
}

impl Semaphore {
    pub fn new(max_permits: i32, initial_permits: i32) -> Self {
        Semaphore {
            inner: InnerSem {
                cv: Condvar::new(),
                max: max_permits,
                lock: Mutex::new(initial_permits),
            }
            .into(),
        }
    }

    /// takes a RAII permit from the semaphore, waits if none are available
    ///
    /// use [std::mem::forget()] to prevent the permit to incrmenet the semaphore on drop
    pub fn permit(&self) -> Permit<'_> {
        self.inner.wait();
        Permit { sem: &self.inner }
    }

    pub fn permits(&self) -> i32 {
        *self.inner.lock.lock().unwrap()
    }
}
