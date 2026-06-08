pub mod zero_cost;
pub mod default;

use crate::types::ai_provider::ConfigPreset;

pub fn all_presets() -> Vec<ConfigPreset> {
    vec![
        zero_cost::preset(),
        default::preset(),
    ]
}
