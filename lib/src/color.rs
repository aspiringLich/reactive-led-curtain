use ecolor::Color32;
use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Oklch {
    l: f32,
    c: f32,
    /// Hue in degrees
    h: f32,
    a: f32,
}

impl Into<Color32> for Oklch {
    fn into(self) -> Color32 {
        // Convert Oklch to OkLab
        let a = (self.c * self.h.to_radians().cos()) as f64;
        let b = (self.c * self.h.to_radians().sin()) as f64;

        // Convert OkLab to LMS
        let l_ = self.l as f64 + 0.3963377774 * a + 0.2158037573 * b;
        let m_ = self.l as f64 - 0.1055613458 * a - 0.0638541728 * b;
        let s_ = self.l as f64 - 0.0894841775 * a - 1.2914855480 * b;

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

        Color32::from_rgba_premultiplied(r, g, b, (self.a * 256.0) as u8)
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

impl Into<Oklch> for Color32 {
    fn into(self) -> Oklch {
        // Convert sRGB to linear RGB
        let r = srgb_to_linear_clamped(self.r() as f64 / 255.0);
        let g = srgb_to_linear_clamped(self.g() as f64 / 255.0);
        let b = srgb_to_linear_clamped(self.b() as f64 / 255.0);

        // Convert linear RGB to LMS
        let l = 0.4122214708 * r + 0.5363325363 * g + 0.0514459929 * b;
        let m = 0.2119034982 * r + 0.6806995451 * g + 0.1073969566 * b;
        let s = 0.0883024619 * r + 0.2817188376 * g + 0.6299787005 * b;

        // Convert LMS to OkLab
        let l_ = l.powf(1.0 / 3.0);
        let m_ = m.powf(1.0 / 3.0);
        let s_ = s.powf(1.0 / 3.0);

        let l = 0.2104542553 * l_ + 0.7936177850 * m_ - 0.0040720468 * s_;
        let a = 1.9779984951 * l_ - 2.4285922050 * m_ + 0.4505937099 * s_;
        let b = 0.0259040371 * l_ + 0.7827717662 * m_ - 0.8086757660 * s_;

        // Convert OkLab to Oklch
        let c = (a * a + b * b).sqrt();
        let h = b.atan2(a).to_degrees();
        let h = if h < 0.0 { h + 360.0 } else { h };

        Oklch {
            l: l as f32 * 100.0,
            c: c as f32 * 100.0,
            h: h as f32,
            a: self.a() as f32 / 256.0,
        }
    }
}

fn srgb_to_linear_clamped(value: f64) -> f64 {
    let out = if value <= 0.04045 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    };
    out.clamp(0.0, 1.0)
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
    /// Color when other is laid on top of self (taking transparency into account)
    pub fn overlay(&self, other: &Self) -> Self {
        let ap = 1.0 - other.a; //a'
        Self {
            l: self.l * ap + other.l * other.a,
            c: self.c * ap + other.c * other.a,
            h: self.h * ap + other.h * other.a,
            a: self.a * ap + other.a,
        }
    }

    pub fn lerp(&self, other: &Self, ratio: f32) -> Self {
        let ratiop = 1.0 - ratio;

        let delta_h = (other.h - self.h + 360.0) % 360.0;
        let shortest_delta_h = if delta_h > 180.0 { delta_h - 360.0 } else { delta_h };
        let interpolated_h = (self.h + shortest_delta_h * ratio + 360.0) % 360.0;

        Self {
            l: self.l * ratiop + other.l * ratio,
            c: self.c * ratiop + other.c * ratio,
            h: interpolated_h,
            a: self.a * ratiop + other.a * ratio,
        }
    }

    pub const LIGHT: Self = Oklch {
        l: 55.0,
        c: 15.0,
        h: 0.0,
        a: 1.0,
    };
    pub const MED: Self = Oklch {
        l: 40.0,
        c: 18.1,
        h: 0.0,
        a: 1.0,
    };
    pub const DIM: Self = Oklch {
        l: 30.0,
        c: 10.5,
        h: 0.0,
        a: 1.0,
    };
    pub const TRANSPARENT: Self = Oklch {
        l: 0.0,
        c: 0.0,
        h: 0.0,
        a: 0.0,
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
        Oklch::LIGHT_COLORS
            .iter()
            .find(|i| i.1 == s)
            .map(|i| i.2.clone())
    }

    pub fn into_hue_str(&self) -> &'static str {
        // doesnt panic because Oklch::LIGHT_COLORS should have stuff in it
        Oklch::LIGHT_COLORS
            .iter()
            .min_by_key(|i| (i.0 - self.h).abs() as u32)
            .unwrap()
            .1
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct OklchGradient {
    stops: Vec<OklchGradientStop>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OklchGradientStop {
    #[serde(deserialize_with = "as_hex")]
    pub color: Oklch,
    pub position: f32,
}

fn as_hex<'de, D>(deserializer: D) -> Result<Oklch, D::Error>
where
    D: Deserializer<'de>,
{
    let hex = String::deserialize(deserializer)?;
    if hex.starts_with('#') && hex.len() == 7 {
        let r = u8::from_str_radix(&hex[1..3], 16).unwrap();
        let g = u8::from_str_radix(&hex[3..5], 16).unwrap();
        let b = u8::from_str_radix(&hex[5..7], 16).unwrap();
        Ok(Color32::from_rgb(r, g, b).into())
    } else {
        Err(serde::de::Error::custom("Expected a hex string"))
    }
}

impl OklchGradient {
    pub fn new(stops: Vec<OklchGradientStop>) -> Option<Self> {
        Some(Self { stops })
    }

    pub fn new_simple(colors: impl ExactSizeIterator<Item = Color32>) -> Self {
        let len = colors.len();
        let stops = colors
            .into_iter()
            .enumerate()
            .map(|(i, color)| OklchGradientStop {
                color: color.into(),
                position: i as f32 / (len - 1) as f32,
            })
            .collect();
        Self { stops }
    }

    pub fn new_hex(hex: impl ExactSizeIterator<Item = &str>) -> Self {
        Self::new_simple(hex.map(|hex| Color32::from_hex(hex).unwrap()))
    }

    pub fn color(&self, position: f32) -> Option<Oklch> {
        let mut prev_stop = None;
        let mut next_stop = None;

        for stop in &self.stops {
            if stop.position > position {
                next_stop = Some(stop);
                break;
            }
            prev_stop = Some(stop);
        }

        match (prev_stop, next_stop) {
            (Some(prev), Some(next)) => {
                let t = (position - prev.position) / (next.position - prev.position);
                Some(prev.color.lerp(&next.color, t))
            }
            (Some(prev), None) => Some(prev.color.clone()),
            (None, Some(next)) => Some(next.color.clone()),
            (None, None) => None,
        }
    }
}
