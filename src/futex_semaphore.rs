/*
* a simple Semaphore with RAII semantics
*/

#![allow(dead_code)]
use std::sync::atomic::{
    AtomicU32,
    Ordering::{Acquire, Relaxed, Release},
};

use crate::futex::{futex_wait, futex_wake_one};

#[derive(Debug)]
pub struct Semaphore {
    inner: AtomicU32,
    max: u32,
}

// a RAII guard which puts back the permit into the semaphore on drop
#[derive(Debug)]
pub struct Permit<'a> {
    sem: &'a Semaphore,
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
    pub fn new(max_permits: u32, initial_permits: u32) -> Self {
        Semaphore {
            inner: initial_permits.into(),
            max: max_permits,
        }
    }

    /// takes a RAII permit from the semaphore, waits if none are available
    ///
    /// use [std::mem::forget()] to prevent the permit to incrmenet the semaphore on drop
    pub fn permit(&self) -> Permit<'_> {
        self.wait();
        Permit { sem: self }
    }

    pub fn free_permits(&self) -> u32 {
        todo!()
    }

    fn wait(&self) {
        let mut v = self.inner.load(Relaxed);
        loop {
            while v == 0 {
                futex_wait(&self.inner, 0);
                v = self.inner.load(Relaxed);
            }
            match self.inner.compare_exchange(v, v - 1, Acquire, Relaxed) {
                // we got the permit
                Ok(_) => return,
                // update value
                Err(old) => v = old,
            }
        }
    }

    pub fn post(&self) {
        if self.inner.fetch_add(1, Release) == 0 {
            futex_wake_one(&self.inner);
        };
    }
}
