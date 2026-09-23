/*
* a simple thread safe queue using conditional variables
*/

#![allow(dead_code)]
use std::{collections::VecDeque, sync::Arc};

use parking_lot::{Condvar, Mutex};

#[derive(Debug, Default, Clone)]
pub struct Channel<T> {
    inner: Arc<ChanInner<T>>,
}

#[derive(Debug, Default)]
pub struct ChanInner<T> {
    q: Mutex<VecDeque<T>>,
    prod_cv: Condvar,
    cons_cv: Condvar,
}

impl<T> Channel<T>
where
    T: std::fmt::Debug,
{
    pub fn new(cap: usize) -> Self {
        Channel {
            inner: ChanInner {
                q: Mutex::new(VecDeque::with_capacity(cap)),
                prod_cv: Condvar::new(),
                cons_cv: Condvar::new(),
            }
            .into(),
        }
    }

    /// blocks the thread until room is available for a value
    pub fn push(&self, value: T) {
        let mut guard = self.inner.q.lock();
        loop {
            if guard.capacity() > guard.len() {
                eprintln!("pushing element {:?}", value);

                guard.push_back(value);
                self.inner.cons_cv.notify_one();
                return;
            } else {
                eprintln!("waiting for capacity...");
                // wait on full queue
                self.inner.prod_cv.wait(&mut guard);
            }
        }
    }

    /// blocks the thread until a value becomes available
    pub fn pop(&self) -> T {
        let mut guard = self.inner.q.lock();
        loop {
            if let Some(item) = guard.pop_front() {
                eprintln!("popping element {:?}", item);

                self.inner.prod_cv.notify_one();
                return item;
            } else {
                eprintln!("waiting for element...");
                // wait on empty queue
                self.inner.cons_cv.wait(&mut guard);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn channel() {
        const N_ITERATIONS: usize = 10000;
        const N_ARR_SIZE: usize = 100;

        let data = (0..N_ARR_SIZE).collect::<Vec<_>>();

        for _ in 0..N_ITERATIONS {
            let queue = Channel::new(5);

            let res = std::thread::scope(|s| {
                // producer
                s.spawn(|| {
                    for e in data.iter().copied() {
                        queue.push(e);
                    }
                });

                // consumer
                let r = s.spawn(|| {
                    let mut res = vec![];
                    for _ in 0..data.len() {
                        res.push(queue.pop());
                    }
                    res
                });
                r.join().unwrap()
            });

            assert_eq!(res, data, "data should be the same in FIFO order");
        }
    }
}
