use std::collections::VecDeque;

use derive_more::derive::Deref;

pub fn vec_default<T: Default>(len: usize) -> Vec<T> {
    Vec::from_iter((0..len).into_iter().map(|_| Default::default()))
}

#[derive(Deref)]
pub struct RingBuffer<T: Default> {
    deq: VecDeque<T>,
}

impl<T: Default> RingBuffer<T> {
    pub fn from_default(len: usize) -> Self {
        assert!(len > 0);
        Self {
            deq: VecDeque::from_iter((0..len).into_iter().map(|_| Default::default())),
        }
    }

    /// Add an element to the end of the ring and pop the value its replacing.
    pub fn replace(&mut self, elem: T) -> T {
        let out = self.deq.pop_front().unwrap();
        self.deq.push_back(elem);
        out
    }
}
