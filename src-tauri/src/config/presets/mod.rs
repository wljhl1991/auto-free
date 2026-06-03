pub mod zero_cost;
pub mod text_only;
pub mod default;
pub mod minimal;

use crate::types::ai_provider::ConfigPreset;

pub fn all_presets() -> Vec<ConfigPreset> {
    vec![
        zero_cost::preset(),
        text_only::preset(),
        default::preset(),
        minimal::preset(),
    ]
}
