use emath::Vec2;
use fields_iter::FieldsInspect;
use serde::{Deserialize, Serialize};

use crate::color::Oklch;

#[derive(Default, Debug, Clone, Serialize, Deserialize, FieldsInspect)]
#[serde(default)]
pub struct EasingFunctions {
    pub percussive: EasingFunction,
    pub bass: EasingFunction,
    pub note: EasingFunction,
    pub octave: EasingFunction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct EasingFunction {
    pub min: f32,
    pub max: f32,
    #[serde(flatten)]
    pub variant: EasingFunctionVariant,
    /// Last value of x used in the easing function (0 to 1)
    #[serde(skip)]
    pub last_x: Vec<f32>,
    #[serde(with="oklch")]
    #[serde(default)]
    pub colors: Option<Vec<Oklch>>
}

mod oklch {
    use serde::Deserialize;
    use crate::color::Oklch;

    pub fn serialize<S>(colors: &Option<Vec<Oklch>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match colors {
            Some(colors) => {
                let colors_vec: Vec<&str> = colors.iter().map(|c| c.into_hue_str()).collect();
                serializer.collect_seq(colors_vec)
            }
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<Vec<Oklch>>, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let opt: Option<Vec<String>> = Option::deserialize(deserializer)?;
        Ok(opt.map(|v| v.into_iter().filter_map(|s| Oklch::light_from_str(&s)).collect::<Vec<_>>()))
    }
}

impl Default for EasingFunction {
    fn default() -> Self {
        EasingFunction {
            min: 0.0,
            max: 1.0,
            variant: EasingFunctionVariant::CubicBezier(CubicBezier {
                p1: Vec2::new(0.5, 0.0),
                p2: Vec2::new(0.5, 1.0),
            }),
            last_x: vec![],
            colors: None,
        }
    }
}

impl EasingFunction {
    /// Ease x and output y in the domain [0, 1]
    pub fn ease_normalize(&mut self, x: f32) -> f32 {
        let x = ((x - self.min) / self.range()).clamp(0.0, 1.0);
        self.last_x.push(x);
        let y = self.variant.solve(x).clamp(0.0, 1.0);
        y
    }

    /// Ease x and output y in x's domain
    pub fn ease(&mut self, x: f32) -> f32 {
        self.ease_normalize(x) * self.range() + self.min
    }

    pub fn range(&self) -> f32 {
        self.max - self.min
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum EasingFunctionVariant {
    CubicBezier(CubicBezier),
}

impl EasingFunctionVariant {
    pub fn solve(&self, x: f32) -> f32 {
        match self {
            EasingFunctionVariant::CubicBezier(bezier) => bezier.solve(x),
        }
    }

    pub fn parametric(&self, t: f32) -> Vec2 {
        match self {
            EasingFunctionVariant::CubicBezier(bezier) => bezier.parametric(t),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CubicBezier {
    #[serde(with = "vec2_as_tuple")]
    pub p1: Vec2,
    #[serde(with = "vec2_as_tuple")]
    pub p2: Vec2,
}

mod vec2_as_tuple {
    use serde::{Deserialize, Serialize};

    pub fn serialize<S>(vec2: &emath::Vec2, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        (vec2.x, vec2.y).serialize(serializer)
    }
    pub fn deserialize<'de, D>(deserializer: D) -> Result<emath::Vec2, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (x, y) = <(f32, f32)>::deserialize(deserializer)?;
        Ok(emath::Vec2::new(x, y))
    }
}

impl CubicBezier {
    pub fn new(p1: Vec2, p2: Vec2) -> Self {
        CubicBezier { p1, p2 }
    }

    pub fn parametric(&self, t: f32) -> Vec2 {
        let x1 = self.p1.x;
        let x2 = self.p2.x;
        let y1 = self.p1.y;
        let y2 = self.p2.y;
        let x = 3.0 * (1.0 - t).powi(2) * t * x1 + 3.0 * (1.0 - t) * t.powi(2) * x2 + t.powi(3);
        let y = 3.0 * (1.0 - t).powi(2) * t * y1 + 3.0 * (1.0 - t) * t.powi(2) * y2 + t.powi(3);
        Vec2::new(x, y)
    }

    /// Returns the y value of the cubic bezier at x
    pub fn solve(&self, x: f32) -> f32 {
        let mut t = 0.5; // Initial guess
        let epsilon = 1e-6;
        let max_iterations = 10;
        for _ in 0..max_iterations {
            let x_t = self.parametric(t).x;
            let error = x_t - x;
            if error.abs() < epsilon {
                break;
            }
            let d_x_t = derivative(|t| self.parametric(t).x, t, epsilon);
            t -= error / d_x_t;
        }
        self.parametric(t).y
    }
}

fn derivative(f: impl Fn(f32) -> f32, t: f32, h: f32) -> f32 {
    (f(t + h) - f(t)) / h
}

#[test]
fn test_cubic() {
    let bez = CubicBezier::new([0.5, 0.0].into(), [0.5, 1.0].into());
    fn assert_float_eq(a: f32, b: f32) {
        assert!(
            (a - b).abs() < f32::EPSILON,
            "assertion `a ≈ b` failed\
           \n  left: {}\
           \n right: {}",
            a,
            b
        );
    }
    assert_float_eq(bez.solve(0.5), 0.5);
    assert_float_eq(bez.solve(0.0), 0.0);
    assert_float_eq(bez.solve(1.0), 1.0);
}
