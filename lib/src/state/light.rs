use serde::{Deserialize, Serialize};

pub struct LightData {}

#[derive(Deserialize, Serialize)]
#[serde(default)]
pub struct LightConfig {
    pub width: u32,
    pub height: u32,
}

impl Default for LightConfig {
    fn default() -> Self {
        Self {
            width: 20,
            height: 26,
        }
    }
}
