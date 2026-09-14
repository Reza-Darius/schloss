/*
* a simple thread safe queue
*/

#![allow(dead_code)]
use std::{collections::VecDeque, sync::Arc};

use parking_lot::{Condvar, Mutex};

#[derive(Debug, Default, Clone)]
pub struct Queue<T> {
    inner: Arc<QueueInner<T>>,
}

#[derive(Debug, Default)]
pub struct QueueInner<T> {
    q: Mutex<VecDeque<T>>,
    condvar: Condvar,
}

impl<T> Queue<T>
where
    T: std::fmt::Debug,
{
    pub fn new(cap: usize) -> Self {
        Queue {
            inner: QueueInner {
                q: Mutex::new(VecDeque::with_capacity(cap)),
                condvar: Condvar::new(),
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
                self.inner.condvar.notify_one();
                return;
            } else {
                eprintln!("waiting for capacity...");
                // wait on full queue
                self.inner.condvar.wait(&mut guard);
            }
        }
    }

    /// blocks the thread until a value becomes available
    pub fn pop(&self) -> T {
        let mut guard = self.inner.q.lock();
        loop {
            if let Some(item) = guard.pop_front() {
                eprintln!("popping element {:?}", item);

                self.inner.condvar.notify_one();
                return item;
            } else {
                eprintln!("waiting for element...");
                // wait on empty queue
                self.inner.condvar.wait(&mut guard);
            }
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    #[test]
    fn spsc() {
        const N_ITERATIONS: usize = 10000;
        const N_ARR_SIZE: usize = 100;

        let data = (0..N_ARR_SIZE).collect::<Vec<_>>();

        for _ in 0..N_ITERATIONS {
            let queue = Queue::new(5);

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
