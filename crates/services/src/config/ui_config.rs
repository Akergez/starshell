use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct UIConfig {
    pub dynamic_colors: bool,
    pub dark_mode: bool,
    pub clock_24_hour: bool,
    pub reduce_motion: bool,
}

pub type UiConfig = UIConfig;

impl Default for UIConfig {
    fn default() -> Self {
        Self {
            dynamic_colors: false,
            dark_mode: true,
            clock_24_hour: true,
            reduce_motion: false,
        }
    }
}

impl UIConfig {
    pub fn clock_format(&self) -> &'static str {
        if self.clock_24_hour {
            "%H:%M"
        } else {
            "%I:%M %p"
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_island_keys_are_ignored() {
        let config: UIConfig =
            toml::from_str("use_new_capsule = false\ncapsule_round = 12.0\ndark_mode = false")
                .expect("legacy config");
        assert!(!config.dark_mode);
        assert!(!config.reduce_motion);
        assert_eq!(config.clock_format(), "%H:%M");
    }

    #[test]
    fn appearance_preferences_survive_serialization() {
        let config = UIConfig {
            dynamic_colors: true,
            dark_mode: false,
            clock_24_hour: false,
            reduce_motion: true,
        };
        let encoded = toml::to_string(&config).expect("serialize config");
        let decoded: UIConfig = toml::from_str(&encoded).expect("deserialize config");
        assert_eq!(decoded, config);
        assert_eq!(decoded.clock_format(), "%I:%M %p");
    }
}
