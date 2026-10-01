use std::{
    ops::Deref,
    ptr::NonNull,
    sync::atomic::{
        AtomicU32,
        Ordering::{AcqRel, Relaxed},
    },
};

pub struct Arc<T> {
    inner: NonNull<InnerArc<T>>,
}

unsafe impl<T: Send + Sync> Send for Arc<T> {}
unsafe impl<T: Send + Sync> Sync for Arc<T> {}

impl<T> Deref for Arc<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.data().data
    }
}

impl<T> Clone for Arc<T> {
    fn clone(&self) -> Self {
        self.data().counter.fetch_add(1, Relaxed);
        Self { inner: self.inner }
    }
}

impl<T> Drop for Arc<T> {
    fn drop(&mut self) {
        if self.data().counter.fetch_sub(1, AcqRel) == 1 {
            unsafe { drop(Box::from_non_null(self.inner)) };
        }
    }
}

impl<T> Arc<T> {
    pub fn new(value: T) -> Self {
        let inner = Box::new(InnerArc {
            counter: 1.into(),
            data: value,
        });

        Arc {
            inner: Box::into_non_null(inner),
        }
    }

    fn data(&self) -> &InnerArc<T> {
        unsafe { self.inner.as_ref() }
    }
}

struct InnerArc<T> {
    counter: AtomicU32,
    data: T,
}
