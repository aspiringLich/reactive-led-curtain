use ecolor::Color32;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq)]
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
    ([$_base:ident $(,$base:ident)* $(,)?] $(,($name:ident, $hue:expr))* $(,)?) => {
        ::paste::paste! {
            color_const!($_base, [<$_base _COLORS>], $(($name, $hue),)*);
        }
        hue_fn!([$($base,)*], $(($name, $hue),)*);
    };
    ([] $(,($name:ident, $hue:expr))* $(,)?) => {
        $(
            #[doc = concat!("Sets the hue to ", stringify!($name), " (", $hue, "°) in OKLCH color space.")]
            pub const fn $name(self) -> Self {
                Oklch { h: $hue, ..self }
            }
        )*
    };
}

macro_rules! color_const {
    ($base:ident, $c:ident $(,($name:ident, $hue:expr))+ $(,)?) => {
        pub const $c: &[(f32, &str, Oklch)] = &[
            $(color_const!($base, $name, $hue),)+
        ];
    };

    ($base:ident, $name:ident, $hue:expr $(,)?) => {
        ($hue, stringify!($name), Oklch::$base.$name())
    }
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

    hue_fn!(
        [LIGHT, MED, DIM],
        (fuschia, 0.0),
        (red, 28.5),
        (orange, 75.0),
        (yellow, 100.0),
        (lime, 120.0),
        (green, 135.0),
        (jade, 150.0),
        (cyan, 175.0),
        (sky_blue, 200.0),
        (blue, 230.0),
        (indigo, 275.0),
        (purple, 290.0),
        (grape, 315.0),
        (magenta, 345.0)
    );

    pub fn light_from_str(s: &str) -> Option<Self> {
        Oklch::LIGHT_COLORS.iter().find(|i| i.1 == s).map(|i| i.2.clone())
    }

    pub fn into_hue_str(&self) -> &'static str {
        // doesnt panic because Oklch::LIGHT_COLORS should have stuff in it
        Oklch::LIGHT_COLORS.iter().min_by_key(|i| (i.0 - self.h).abs() as u32).unwrap().1
    }
}
