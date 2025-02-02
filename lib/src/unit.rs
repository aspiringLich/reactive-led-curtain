use derive_more::derive::Deref;
use rustfft::num_complex::Complex;

#[derive(Clone, Copy, Debug, Deref, Default, PartialEq, PartialOrd)]
pub struct Db(pub f32);

impl Into<Db> for f32 {
    fn into(self) -> Db {
        let amin: f32 = 1e-10_f32;
        let power = self * self;
        Db(10.0 * f32::log10(f32::max(amin, power)))
    }
}

impl Into<Db> for Complex<f32> {
    fn into(self) -> Db {
        let amin: f32 = 1e-10_f32;
        let power = self.norm_sqr();
        Db(10.0 * f32::log10(f32::max(amin, power)))
    }
}

#[derive(Clone, Copy, Debug, Deref, Default, PartialEq, PartialOrd)]
pub struct Power(pub f32);

impl Into<Power> for f32 {
    fn into(self) -> Power {
        Power(self * self)
    }
}

impl Into<Db> for Power {
    fn into(self) -> Db {
        let amin: f32 = 1e-10_f32;
        Db(10.0 * f32::log10(f32::max(*self, amin)))
    }
}
