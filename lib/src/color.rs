use ecolor::Color32;

#[derive(Debug, Clone)]
pub struct Oklch {
    l: f32,
    c: f32,
    /// Hue in degrees
    h: f32,
}

impl Into<Color32> for Oklch {
    fn into(self) -> Color32 {
        // Convert Oklch to OkLab
        let a = self.c * self.h.to_radians().cos();
        let b = self.c * self.h.to_radians().sin();

        // Convert OkLab to LMS
        let l_ = self.l as f64 + 0.3963377774 * a as f64 + 0.2158037573 * b as f64;
        let m_ = self.l as f64 - 0.1055613458 * a as f64 - 0.0638541728 * b as f64;
        let s_ = self.l as f64 - 0.0894841775 * a as f64 - 1.2914855480 * b as f64;

        // Convert LMS to linear RGB
        let l = l_ * l_ * l_;
        let m = m_ * m_ * m_;
        let s = s_ * s_ * s_;

        let r_linear = 4.0767416621 * l - 3.3077115913 * m + 0.2309699292 * s;
        let g_linear = -1.2684380046 * l + 2.6097574011 * m - 0.3413193965 * s;
        let b_linear = -0.0041960863 * l - 0.7034186147 * m + 1.7076147010 * s;

        // Convert linear RGB to sRGB
        let r = linear_to_srgb_clamped(r_linear) as u8;
        let g = linear_to_srgb_clamped(g_linear) as u8;
        let b = linear_to_srgb_clamped(b_linear) as u8;

        Color32::from_rgb(r, g, b)
    }
}

// Helper function to convert linear RGB to sRGB
fn linear_to_srgb_clamped(value: f64) -> f64 {
    let out = if value <= 0.0031308 {
        12.92 * value
    } else {
        1.055 * value.powf(1.0 / 2.4) - 0.055
    };
    out.clamp(0.0, 255.0)
}

macro_rules! hue_fn {
    ($name:ident, $hue:expr) => {
        #[doc = concat!("Sets the hue to ", stringify!($name), " (", $hue, "°) in OKLCH color space.")]
        pub const fn $name(self) -> Self {
            Oklch { h: $hue, ..self }
        }
    };
}

impl Oklch {
    pub const LIGHT: Self = Oklch {
        l: 55.0,
        c: 15.0,
        h: 0.0,
    };
    pub const MED: Self = Oklch {
        l: 40.0,
        c: 18.1,
        h: 0.0,
    };
    pub const DIM: Self = Oklch {
        l: 30.0,
        c: 10.5,
        h: 0.0,
    };

    hue_fn!(fuschia, 0.0);
    hue_fn!(red, 28.5);
    hue_fn!(orange, 75.0);
    hue_fn!(yellow, 100.0);
    hue_fn!(lime, 120.0);
    hue_fn!(green, 135.0);
    hue_fn!(jade, 150.0);
    hue_fn!(cyan, 175.0);
    hue_fn!(sky_blue, 200.0);
    hue_fn!(blue, 230.0);
    hue_fn!(indigo, 275.0);
    hue_fn!(purple, 290.0);
    hue_fn!(grape, 315.0);
    hue_fn!(magenta, 345.0);
}
