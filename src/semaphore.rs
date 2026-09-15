/*
* a simple Semaphore
*/

#![allow(dead_code)]
use std::sync::{Condvar, Mutex};

pub struct Semaphore {
    cv: Condvar,
    lock: Mutex<i32>,
}

impl Semaphore {
    pub fn new(permits: i32) -> Self {
        Semaphore {
            cv: Condvar::new(),
            lock: Mutex::new(permits),
        }
    }

    pub fn wait(&self) {
        let mut guard = self.lock.lock().unwrap();
        while *guard <= 0 {
            guard = self.cv.wait(guard).unwrap();
        }
        *guard -= 1;
    }

    pub fn post(&self) {
        let mut guard = self.lock.lock().unwrap();
        *guard += 1;
        self.cv.notify_one();

    }

    pub fn permits(&self) -> i32 {
        *self.lock.lock().unwrap()
    }
}
