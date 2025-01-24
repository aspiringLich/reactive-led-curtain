use derive_more::derive::Deref;

#[derive(Clone, Copy, Debug, Deref, Default)]
pub struct Db(pub f32);

impl Db {
    pub fn from_amplitude(a: f32) -> Db {
        let amin: f32 = 1e-10_f32;
        let power = a.powi(2);
        Db(10.0 * f32::log10(f32::max(amin, power)))
    }
}
