pub mod defaults;
pub mod lock_screen_config;
pub mod mpris_config;
pub mod record_config;
pub mod ui_config;

pub use defaults::Defaults;
pub use lock_screen_config::{LockScreenConfig, LockscreenConfig};
pub use mpris_config::MprisConfig;
pub use record_config::RecordConfig;
pub use ui_config::{CapsuleLocation, CapsuleStyle, UIConfig, UiConfig};

use arc_swap::ArcSwap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::broadcast;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    #[serde(alias = "lock_screen")]
    pub lockscreen: LockScreenConfig,

    #[serde(default)]
    pub defaults: Defaults,

    #[serde(default)]
    pub ui: UIConfig,

    #[serde(default)]
    #[serde(alias = "music")]
    pub mpris: MprisConfig,

    #[serde(default)]
    pub record: RecordConfig,
}

impl AppConfig {
    pub fn load() -> Self {
        let config_path = Self::get_config_path();
        match fs::read_to_string(&config_path) {
            Ok(content) => match toml::from_str::<AppConfig>(&content) {
                Ok(mut cfg) => {
                    cfg.ui.capsule_round = cfg.ui.capsule_round.clamp(0.0, 24.0);
                    cfg
                }
                Err(error) => {
                    eprintln!("Invalid config {}: {error}", config_path.display());
                    AppConfig::default()
                }
            },
            Err(_) => {
                let default_cfg = AppConfig::default();
                let _ = Self::save(&default_cfg);
                default_cfg
            }
        }
    }

    pub fn save(cfg: &AppConfig) -> std::io::Result<()> {
        let config_path = Self::get_config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let existing = fs::read_to_string(&config_path)
            .ok()
            .and_then(|content| content.parse::<toml::Table>().ok())
            .unwrap_or_default();
        let content = Self::merged_content(existing, cfg)?;
        fs::write(config_path, content)
    }

    pub fn get_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("~/.config"))
            .join("starshell")
            .join("config.toml")
    }

    fn merged_content(mut existing: toml::Table, cfg: &AppConfig) -> std::io::Result<String> {
        let current = toml::Table::try_from(cfg).map_err(std::io::Error::other)?;
        merge_tables(&mut existing, current);
        toml::to_string_pretty(&existing).map_err(std::io::Error::other)
    }
}

fn merge_tables(target: &mut toml::Table, source: toml::Table) {
    for (key, value) in source {
        match (target.get_mut(&key), value) {
            (Some(toml::Value::Table(target_table)), toml::Value::Table(source_table)) => {
                merge_tables(target_table, source_table);
            }
            (_, value) => {
                target.insert(key, value);
            }
        }
    }
}


#[derive(Clone)]
pub struct ConfigService {
    config: Arc<ArcSwap<AppConfig>>,
    tx: broadcast::Sender<Arc<AppConfig>>,
}

impl Default for ConfigService {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigService {
    pub fn new() -> Self {
        let loaded = AppConfig::load();
        let (tx, _) = broadcast::channel(16);
        Self {
            config: Arc::new(ArcSwap::from_pointee(loaded)),
            tx,
        }
    }

    pub fn get(&self) -> Arc<AppConfig> {
        self.config.load_full()
    }

    pub fn set(&self, mut new_config: AppConfig) -> std::io::Result<()> {
        new_config.ui.capsule_round = new_config.ui.capsule_round.clamp(0.0, 24.0);
        AppConfig::save(&new_config)?;
        let arc_config = Arc::new(new_config);
        self.config.store(arc_config.clone());
        let _ = self.tx.send(arc_config);
        Ok(())
    }

    pub fn update<F>(&self, f: F) -> std::io::Result<()>
    where
        F: FnOnce(&mut AppConfig),
    {
        let mut current = (**self.config.load()).clone();
        f(&mut current);
        current.ui.capsule_round = current.ui.capsule_round.clamp(0.0, 24.0);
        AppConfig::save(&current)?;
        let arc_config = Arc::new(current);
        self.config.store(arc_config.clone());
        let _ = self.tx.send(arc_config);
        Ok(())
    }

    pub fn reload(&self) {
        let reloaded = AppConfig::load();
        let arc_config = Arc::new(reloaded);
        self.config.store(arc_config.clone());
        let _ = self.tx.send(arc_config);
    }

    pub fn subscribe(&self) -> broadcast::Receiver<Arc<AppConfig>> {
        self.tx.subscribe()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saving_preserves_keys_owned_by_other_tools() {
        let existing: toml::Table =
            "[bar]\nsize = 30.0\n[osd]\nposition = 'right'\n[ui]\nforeign = 1\n"
                .parse()
                .expect("existing config");
        let content = AppConfig::merged_content(existing, &AppConfig::default())
            .expect("merged config");
        let merged: toml::Table = content.parse().expect("merged table");
        assert_eq!(merged["osd"]["position"].as_str(), Some("right"));
        assert_eq!(merged["ui"]["foreign"].as_integer(), Some(1));
        assert!(merged["ui"].get("dark_mode").is_some());
    }
}
