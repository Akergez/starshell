use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BarPosition {
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
    Record,
    Shelf,
    System,
    Audio,
    Displays,
    ControlCenter,
}

impl BarModule {
    pub fn parse(name: &str) -> Option<Self> {
        let normalized = name
            .trim()
            .to_ascii_lowercase()
            .replace(['-', '_', ' '], "");
        Some(match normalized.as_str() {
            "launcher" | "launcherbtn" | "apps" => Self::Launcher,
            "workspaces" => Self::Workspaces,
            "media" | "mpris" | "player" => Self::Media,
            "clock" | "calendar" => Self::Clock,
            "tray" | "systray" => Self::Tray,
            "notifications" | "notification" => Self::Notifications,
            "clipboard" => Self::Clipboard,
            "record" | "recorder" => Self::Record,
            "shelf" => Self::Shelf,
            "system" | "sysinfo" => Self::System,
            "audio" | "volume" => Self::Audio,
            "displays" | "brightness" => Self::Displays,
            "controlcenter" | "controls" | "settings" | "quicksettings" => Self::ControlCenter,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Launcher => "Launcher",
            Self::Workspaces => "Workspaces",
            Self::Media => "Media",
            Self::Clock => "Clock",
            Self::Tray => "Tray",
            Self::Notifications => "Notifications",
            Self::Clipboard => "Clipboard",
            Self::Record => "Record",
            Self::Shelf => "Shelf",
            Self::System => "System",
            Self::Audio => "Audio",
            Self::Displays => "Displays",
            Self::ControlCenter => "ControlCenter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BarConfig {
    pub size: f32,
    pub position: BarPosition,
    pub auto_hide: bool,
    pub hide_delay_ms: u64,
    pub start: Vec<String>,
    pub center: Vec<String>,
    pub end: Vec<String>,
}

impl Default for BarConfig {
    fn default() -> Self {
        let names = |modules: &[BarModule]| {
            modules
                .iter()
                .map(|module| module.name().to_string())
                .collect()
        };
        Self {
            size: 28.0,
            position: BarPosition::Top,
            auto_hide: false,
            hide_delay_ms: 500,
            start: names(&[BarModule::Launcher, BarModule::Workspaces, BarModule::Media]),
            center: names(&[BarModule::Clock]),
            end: names(&[
                BarModule::Record,
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

impl BarConfig {
    pub fn modules(names: &[String]) -> Vec<BarModule> {
        let mut modules = Vec::with_capacity(names.len());
        for module in names.iter().filter_map(|name| BarModule::parse(name)) {
            if !modules.contains(&module) {
                modules.push(module);
            }
        }
        modules
    }

    pub fn clamped_size(&self) -> f32 {
        self.size.clamp(22.0, 48.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct PowerActions {
    pub sleep: String,
    pub reboot: String,
    pub poweroff: String,
}

impl Default for PowerActions {
    fn default() -> Self {
        Self {
            sleep: "systemctl suspend".to_string(),
            reboot: "systemctl reboot".to_string(),
            poweroff: "systemctl poweroff".to_string(),
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ControlCenterConfig {
    pub power_actions: PowerActions,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn module_names_accept_aliases_and_skip_unknown_entries() {
        let names = [
            "LauncherBtn",
            "Mpris",
            "SysInfo",
            "Systray",
            "KeyboardLayout",
            "Settings",
            "mpris",
        ]
        .map(String::from);
        assert_eq!(
            BarConfig::modules(&names),
            vec![
                BarModule::Launcher,
                BarModule::Media,
                BarModule::System,
                BarModule::Tray,
                BarModule::ControlCenter,
            ]
        );
    }

    #[test]
    fn existing_bar_sections_parse_with_defaults() {
        let config: BarConfig =
            toml::from_str("size = 30.0\nposition = 'bottom'\npadding = 12.0\nshow_border = true")
                .expect("bar config");
        assert_eq!(config.size, 30.0);
        assert_eq!(config.position, BarPosition::Bottom);
        assert!(!config.auto_hide);
        assert!(!config.end.is_empty());
    }

    #[test]
    fn every_module_name_round_trips() {
        for module in [
            BarModule::Launcher,
            BarModule::Workspaces,
            BarModule::Media,
            BarModule::Clock,
            BarModule::Tray,
            BarModule::Notifications,
            BarModule::Clipboard,
            BarModule::Record,
            BarModule::Shelf,
            BarModule::System,
            BarModule::Audio,
            BarModule::Displays,
            BarModule::ControlCenter,
        ] {
            assert_eq!(BarModule::parse(module.name()), Some(module));
        }
    }
}
