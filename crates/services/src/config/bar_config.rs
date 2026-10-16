use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BarEdge {
    #[default]
    Top,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BarModule {
    Launcher,
    Workspaces,
    Media,
    Clock,
    Tray,
    Notifications,
    Clipboard,
    Recorder,
    Shelf,
    KeyboardLayout,
    System,
    Audio,
    Displays,
    ControlCenter,
}

impl BarModule {
    pub const ALL: [BarModule; 14] = [
        Self::Launcher,
        Self::Workspaces,
        Self::Media,
        Self::Clock,
        Self::Tray,
        Self::Notifications,
        Self::Clipboard,
        Self::Recorder,
        Self::Shelf,
        Self::KeyboardLayout,
        Self::System,
        Self::Audio,
        Self::Displays,
        Self::ControlCenter,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Self::Launcher => "launcher",
            Self::Workspaces => "workspaces",
            Self::Media => "media",
            Self::Clock => "clock",
            Self::Tray => "tray",
            Self::Notifications => "notifications",
            Self::Clipboard => "clipboard",
            Self::Recorder => "recorder",
            Self::Shelf => "shelf",
            Self::KeyboardLayout => "keyboard-layout",
            Self::System => "system",
            Self::Audio => "audio",
            Self::Displays => "displays",
            Self::ControlCenter => "control-center",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|module| module.key() == key.trim())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarLayout {
    pub left: Vec<String>,
    pub middle: Vec<String>,
    pub right: Vec<String>,
}

impl Default for BarLayout {
    fn default() -> Self {
        let keys = |modules: &[BarModule]| {
            modules
                .iter()
                .map(|module| module.key().to_string())
                .collect()
        };
        Self {
            left: keys(&[BarModule::Launcher, BarModule::Workspaces, BarModule::Media]),
            middle: keys(&[BarModule::Clock]),
            right: keys(&[
                BarModule::Recorder,
                BarModule::Shelf,
                BarModule::KeyboardLayout,
                BarModule::Tray,
                BarModule::Notifications,
                BarModule::System,
                BarModule::Audio,
                BarModule::Displays,
                BarModule::ControlCenter,
            ]),
        }
    }
}

impl BarLayout {
    pub fn resolve(keys: &[String]) -> Vec<BarModule> {
        let mut modules = Vec::with_capacity(keys.len());
        for key in keys {
            match BarModule::from_key(key) {
                Some(module) if !modules.contains(&module) => modules.push(module),
                Some(_) => {}
                None => crate::log_warn!("Config", "Unknown bar module '{key}'"),
            }
        }
        modules
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub height: f32,
    pub edge: BarEdge,
    pub autohide: bool,
    pub autohide_delay_ms: u64,
    pub layout: BarLayout,
}

impl Default for BarConfig {
    fn default() -> Self {
        Self {
            height: Self::DEFAULT_HEIGHT,
            edge: BarEdge::Top,
            autohide: false,
            autohide_delay_ms: 500,
            layout: BarLayout::default(),
        }
    }
}

impl BarConfig {
    pub const DEFAULT_HEIGHT: f32 = 28.0;
    pub const HEIGHT_RANGE: (f32, f32) = (22.0, 48.0);

    pub fn clamped_height(&self) -> f32 {
        self.height
            .clamp(Self::HEIGHT_RANGE.0, Self::HEIGHT_RANGE.1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_keeps_known_modules_once_in_order() {
        let keys = ["clock", "control-center", "nope", "clock", "audio"].map(String::from);
        assert_eq!(
            BarLayout::resolve(&keys),
            vec![BarModule::Clock, BarModule::ControlCenter, BarModule::Audio]
        );
    }

    #[test]
    fn every_module_key_round_trips() {
        for module in BarModule::ALL {
            assert_eq!(BarModule::from_key(module.key()), Some(module));
        }
    }

    #[test]
    fn bar_section_parses_with_defaults() {
        let config: BarConfig =
            toml::from_str("height = 32.0\nedge = 'bottom'\n[layout]\nleft = ['clock']")
                .expect("bar config");
        assert_eq!(config.height, 32.0);
        assert_eq!(config.edge, BarEdge::Bottom);
        assert!(!config.autohide);
        assert_eq!(config.layout.left, vec!["clock".to_string()]);
        assert!(!config.layout.right.is_empty());
        assert_eq!(
            BarConfig {
                height: 100.0,
                ..config
            }
            .clamped_height(),
            48.0
        );
    }
}
