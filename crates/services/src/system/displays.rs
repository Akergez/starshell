use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use tokio::process::Command;
use tokio::sync::mpsc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayBackend {
    Backlight,
    Ddc,
}

impl DisplayBackend {
    pub fn label(self) -> &'static str {
        match self {
            Self::Backlight => "Backlight",
            Self::Ddc => "DDC/CI",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Display {
    pub id: String,
    pub name: String,
    pub connector: String,
    pub backend: DisplayBackend,
    pub brightness: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyboardBacklight {
    pub id: String,
    pub device: String,
    pub brightness: u32,
    max: u64,
}

impl KeyboardBacklight {
    fn level_for(&self, percent: u32) -> u64 {
        (percent.min(100) as f64 / 100.0 * self.max as f64).round() as u64
    }

    fn percent_of(&self, level: u64) -> u32 {
        ((level as f64 / self.max as f64) * 100.0).round() as u32
    }
}

#[derive(Clone)]
pub struct DisplayService {
    displays: Arc<ArcSwap<Vec<Display>>>,
    keyboard_backlights: Arc<ArcSwap<Vec<KeyboardBacklight>>>,
    requests: mpsc::UnboundedSender<(String, String)>,
}

impl Default for DisplayService {
    fn default() -> Self {
        Self::new()
    }
}

impl DisplayService {
    pub fn new() -> Self {
        let _guard = crate::tokio_handle().enter();
        let (requests, receiver) = mpsc::unbounded_channel();
        let service = Self {
            displays: Arc::new(ArcSwap::from_pointee(read_backlights())),
            keyboard_backlights: Arc::new(ArcSwap::from_pointee(read_keyboard_backlights())),
            requests,
        };
        crate::spawn_tokio(service.clone().apply_requests(receiver));
        crate::spawn_tokio(service.clone().poll());
        service
    }

    pub fn get_displays(&self) -> Arc<Vec<Display>> {
        self.displays.load_full()
    }

    pub fn set_brightness(&self, id: &str, percent: u32) {
        let percent = percent.clamp(1, 100);
        let mut displays = (**self.displays.load()).clone();
        for display in displays.iter_mut().filter(|display| display.id == id) {
            display.brightness = percent;
        }
        self.displays.store(Arc::new(displays));
        let _ = self.requests.send((id.to_string(), format!("{percent}%")));
    }

    pub fn get_keyboard_backlights(&self) -> Arc<Vec<KeyboardBacklight>> {
        self.keyboard_backlights.load_full()
    }

    pub fn set_keyboard_brightness(&self, id: &str, percent: u32) {
        let mut keyboards = (**self.keyboard_backlights.load()).clone();
        let Some(keyboard) = keyboards.iter_mut().find(|keyboard| keyboard.id == id) else {
            return;
        };
        let level = keyboard.level_for(percent);
        keyboard.brightness = keyboard.percent_of(level);
        self.keyboard_backlights.store(Arc::new(keyboards));
        let _ = self.requests.send((id.to_string(), level.to_string()));
    }

    pub fn set_all(&self, percent: u32) {
        for display in self.get_displays().iter() {
            self.set_brightness(&display.id, percent);
        }
    }

    async fn poll(self) {
        const DDC_EVERY: u32 = 12;
        let ddc_available = program_exists("ddcutil");
        let mut ddc: Vec<(String, Display)> = Vec::new();
        let mut ticks = 0u32;
        loop {
            if ddc_available && ticks.is_multiple_of(DDC_EVERY) {
                ddc = detect_ddc().await;
                for (display_number, display) in &mut ddc {
                    if let Some(value) = ddc_brightness(display_number).await {
                        display.brightness = value;
                    }
                }
            }
            let mut displays = read_backlights();
            let current = self.displays.load_full();
            displays.extend(ddc.iter().map(|(_, display)| {
                current
                    .iter()
                    .find(|known| known.id == display.id)
                    .cloned()
                    .unwrap_or_else(|| display.clone())
            }));
            if *current != displays {
                self.displays.store(Arc::new(displays));
            }
            let keyboards = read_keyboard_backlights();
            if *self.keyboard_backlights.load_full() != keyboards {
                self.keyboard_backlights.store(Arc::new(keyboards));
            }
            ticks = ticks.wrapping_add(1);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn apply_requests(self, mut receiver: mpsc::UnboundedReceiver<(String, String)>) {
        while let Some(first) = receiver.recv().await {
            let mut pending = HashMap::from([first]);
            while let Ok((id, value)) = receiver.try_recv() {
                pending.insert(id, value);
            }
            for (id, value) in pending {
                let result = if let Some(device) = id
                    .strip_prefix("backlight:")
                    .or_else(|| id.strip_prefix("keyboard:"))
                {
                    run("brightnessctl", &["--device", device, "set", &value])
                        .await
                        .map(drop)
                } else if let Some(display_number) = id.strip_prefix("ddc:") {
                    run(
                        "ddcutil",
                        &[
                            "--display",
                            display_number,
                            "setvcp",
                            "10",
                            value.trim_end_matches('%'),
                        ],
                    )
                    .await
                    .map(drop)
                } else {
                    Ok(())
                };
                if let Err(error) = result {
                    crate::log_warn!("Displays", "Could not set brightness of {id}: {error}");
                }
            }
        }
    }
}

async fn run(program: &str, args: &[&str]) -> anyhow::Result<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(5),
        Command::new(program).args(args).env("LC_ALL", "C").output(),
    )
    .await??;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn program_exists(program: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(program).is_file())
    })
}

fn read_number(path: &Path) -> Option<u64> {
    std::fs::read_to_string(path).ok()?.trim().parse().ok()
}

fn read_backlights() -> Vec<Display> {
    let Ok(entries) = std::fs::read_dir("/sys/class/backlight") else {
        return Vec::new();
    };
    let mut displays: Vec<Display> = entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let device = entry.file_name().to_string_lossy().into_owned();
            let max = read_number(&path.join("max_brightness")).filter(|max| *max > 0)?;
            let current = read_number(&path.join("brightness"))?;
            Some(Display {
                id: format!("backlight:{device}"),
                name: "Built-in display".to_string(),
                connector: device,
                backend: DisplayBackend::Backlight,
                brightness: ((current as f64 / max as f64) * 100.0).round() as u32,
            })
        })
        .collect();
    displays.sort_by(|left, right| left.id.cmp(&right.id));
    displays
}

fn read_keyboard_backlights() -> Vec<KeyboardBacklight> {
    let Ok(entries) = std::fs::read_dir("/sys/class/leds") else {
        return Vec::new();
    };
    let mut keyboards: Vec<KeyboardBacklight> = entries
        .flatten()
        .filter_map(|entry| {
            let device = entry.file_name().to_string_lossy().into_owned();
            if !device.ends_with("::kbd_backlight") {
                return None;
            }
            let path = entry.path();
            let max = read_number(&path.join("max_brightness")).filter(|max| *max > 0)?;
            let current = read_number(&path.join("brightness"))?;
            let mut keyboard = KeyboardBacklight {
                id: format!("keyboard:{device}"),
                device,
                brightness: 0,
                max,
            };
            keyboard.brightness = keyboard.percent_of(current.min(max));
            Some(keyboard)
        })
        .collect();
    keyboards.sort_by(|left, right| left.id.cmp(&right.id));
    keyboards
}

async fn detect_ddc() -> Vec<(String, Display)> {
    match run("ddcutil", &["detect", "--terse"]).await {
        Ok(output) => parse_ddc_detect(&output),
        Err(_) => Vec::new(),
    }
}

fn parse_ddc_detect(output: &str) -> Vec<(String, Display)> {
    let mut displays = Vec::new();
    let mut current: Option<(String, String, String)> = None;
    let mut flush = |current: &mut Option<(String, String, String)>| {
        if let Some((number, connector, model)) = current.take() {
            displays.push((
                number.clone(),
                Display {
                    id: format!("ddc:{number}"),
                    name: if model.is_empty() {
                        format!("Display {number}")
                    } else {
                        model
                    },
                    connector,
                    backend: DisplayBackend::Ddc,
                    brightness: 100,
                },
            ));
        }
    };
    for line in output.lines() {
        let line = line.trim();
        if let Some(number) = line.strip_prefix("Display ") {
            flush(&mut current);
            current = Some((number.trim().to_string(), String::new(), String::new()));
        } else if let Some((_, connector, model)) = current.as_mut() {
            if let Some(value) = line.strip_prefix("DRM connector:") {
                let value = value.trim();
                *connector = value
                    .split_once('-')
                    .map(|(_, rest)| rest.to_string())
                    .unwrap_or_else(|| value.to_string());
            } else if let Some(value) = line.strip_prefix("Monitor:") {
                *model = value
                    .trim()
                    .split(':')
                    .nth(1)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
            }
        }
    }
    flush(&mut current);
    displays
}

async fn ddc_brightness(display_number: &str) -> Option<u32> {
    let output = run(
        "ddcutil",
        &["--display", display_number, "getvcp", "10", "--terse"],
    )
    .await
    .ok()?;
    parse_vcp(&output)
}

fn parse_vcp(output: &str) -> Option<u32> {
    let fields: Vec<&str> = output.split_whitespace().collect();
    let current: f64 = fields.get(3)?.parse().ok()?;
    let max: f64 = fields.get(4)?.parse().ok()?;
    (max > 0.0).then(|| (current / max * 100.0).round() as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ddc_detection_reads_connector_and_model() {
        let output = "Display 1\n   I2C bus:  /dev/i2c-4\n   DRM connector:  card1-DP-2\n   Monitor:  DEL:DELL U2720Q:ABC\n\nInvalid display\n   I2C bus: /dev/i2c-5\n";
        let displays = parse_ddc_detect(output);
        assert_eq!(displays.len(), 1);
        let (number, display) = &displays[0];
        assert_eq!(number, "1");
        assert_eq!(display.connector, "DP-2");
        assert_eq!(display.name, "DELL U2720Q");
        assert_eq!(display.backend.label(), "DDC/CI");
    }

    #[test]
    fn keyboard_brightness_snaps_to_device_levels() {
        let keyboard = KeyboardBacklight {
            id: "keyboard:tpacpi::kbd_backlight".to_string(),
            device: "tpacpi::kbd_backlight".to_string(),
            brightness: 0,
            max: 2,
        };
        assert_eq!(keyboard.level_for(0), 0);
        assert_eq!(keyboard.level_for(30), 1);
        assert_eq!(keyboard.level_for(80), 2);
        assert_eq!(keyboard.percent_of(1), 50);
    }

    #[test]
    fn vcp_values_become_percentages() {
        assert_eq!(parse_vcp("VCP 10 C 30 60"), Some(50));
        assert_eq!(parse_vcp("VCP 10 ERR"), None);
    }
}
