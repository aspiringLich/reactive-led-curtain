use std::collections::VecDeque;

use derive_more::derive::Deref;


macro_rules! profile_function {
    ($($args:tt)*) => {
        #[cfg(feature = "profiling")]
        ::puffin_egui::puffin::profile_function!($($args)*);
    };
}
pub(crate) use profile_function;

macro_rules! profile_scope {
    ($($args:tt)*) => {
        #[cfg(feature = "profiling")]
        ::puffin_egui::puffin::profile_scope!($($args)*);
    };
}
pub(crate) use profile_scope;


pub fn vec_default<T: Default>(len: usize) -> Vec<T> {
    Vec::from_iter((0..len).into_iter().map(|_| Default::default()))
}

pub fn vec_clone<T: Clone>(elem: &T, len: usize) -> Vec<T> {
    Vec::from_iter((0..len).into_iter().map(|_| elem.clone()))
}

#[derive(Deref, Clone, Debug)]
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

#[derive(Clone)]
pub struct RollingAverage {
    sum: f32,
    buf: RingBuffer<f32>,
}

impl RollingAverage {
    pub fn new(len: usize) -> Self {
        assert!(len > 0);
        Self {
            sum: 0.0,
            buf: RingBuffer::from_default(len),
        }
    }

    pub fn consume(&mut self, elem: f32) -> f32 {
        let out = self.buf.replace(elem);
        self.sum += elem - out;
        self.average()
    }

    pub fn average(&self) -> f32 {
        self.sum / self.buf.len() as f32
    }
}
