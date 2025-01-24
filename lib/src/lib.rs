#![warn(clippy::all, rust_2018_idioms)]

pub mod fft;
pub mod state;
pub mod unit;

pub const SAMPLE_SIZE: usize = 2048 * 3;
pub const SAMPLE_WINDOWS: usize = 8;
