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
    lock: Mutex<i32>,
}

impl InnerSem {
    fn wait(&self) {
        let mut guard = self.lock.lock().unwrap();
        while *guard <= 0 {
            guard = self.cv.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    fn post(&self) {
        let mut guard = self.lock.lock().unwrap();
        *guard += 1;
        self.cv.notify_one();
    }
}

// a RAII guard which puts back the permit into the semaphore on drop
#[derive(Debug)]
pub struct Permit<'a> {
    sem: &'a InnerSem,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.sem.post();
    }
}

impl Semaphore {
    pub fn new(permits: i32) -> Self {
        Semaphore {
            inner: InnerSem {
                cv: Condvar::new(),
                lock: Mutex::new(permits),
            }
            .into(),
        }
    }

    /// takes a RAII permit from the semaphore, waits if none are available
    pub fn permit(&self) -> Permit<'_> {
        self.wait();
        Permit { sem: &self.inner }
    }

    /// decrements the semaphore and waits if permits <= 0
    pub fn wait(&self) {
        self.inner.wait();
    }

    /// increments the semaphore and wakes a potential waiter
    pub fn post(&self) {
        self.inner.post();
    }

    pub fn permits(&self) -> i32 {
        *self.inner.lock.lock().unwrap()
    }
}
