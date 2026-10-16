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

#[derive(Clone)]
pub struct DisplayService {
    displays: Arc<ArcSwap<Vec<Display>>>,
    requests: mpsc::UnboundedSender<(String, u32)>,
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
        let _ = self.requests.send((id.to_string(), percent));
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
            ticks = ticks.wrapping_add(1);
            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn apply_requests(self, mut receiver: mpsc::UnboundedReceiver<(String, u32)>) {
        while let Some(first) = receiver.recv().await {
            let mut pending = HashMap::from([first]);
            while let Ok((id, percent)) = receiver.try_recv() {
                pending.insert(id, percent);
            }
            for (id, percent) in pending {
                let result = if let Some(device) = id.strip_prefix("backlight:") {
                    run(
                        "brightnessctl",
                        &["--device", device, "set", &format!("{percent}%")],
                    )
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
                            &percent.to_string(),
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
    fn vcp_values_become_percentages() {
        assert_eq!(parse_vcp("VCP 10 C 30 60"), Some(50));
        assert_eq!(parse_vcp("VCP 10 ERR"), None);
    }
}
