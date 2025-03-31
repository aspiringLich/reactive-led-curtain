#![warn(clippy::all, rust_2018_idioms)]
#![feature(anonymous_lifetime_in_impl_trait)]

pub mod cfg;
pub mod color;
pub mod easing;
// pub mod prof;
pub mod state;
pub mod unit;
pub mod util;

pub use rustfft::{Fft, FftDirection, FftPlanner, num_complex::Complex};
pub use emath::Vec2;
