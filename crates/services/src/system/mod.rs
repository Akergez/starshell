use anyhow::{Context, Result};
use arc_swap::ArcSwap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

mod capture;
mod usage;
pub use capture::CaptureStatus;
pub use usage::UsageStatus;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioSink {
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct AudioSource {
    pub name: String,
    pub description: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SystemStatus {
    pub volume: u32,
    pub is_muted: bool,
    pub input_volume: u32,
    pub is_input_muted: bool,
    pub brightness: u32,
    pub audio_sinks: Vec<AudioSink>,
    pub audio_sources: Vec<AudioSource>,
}

#[derive(Clone)]
pub struct SystemService {
    status: Arc<ArcSwap<SystemStatus>>,
    capture_status: Arc<ArcSwap<CaptureStatus>>,
    target_volume: Arc<AtomicU32>,
    volume_pending: Arc<AtomicBool>,
    volume_notify: Arc<tokio::sync::Notify>,
    target_input_volume: Arc<AtomicU32>,
    input_volume_pending: Arc<AtomicBool>,
    input_volume_notify: Arc<tokio::sync::Notify>,
    audio_changed: Arc<tokio::sync::Notify>,
    usage_sample: Arc<tokio::sync::Mutex<Option<(u64, u64)>>>,
}

impl SystemService {
    pub fn new() -> Self {
        let _guard = crate::tokio_handle().enter();
        let service = Self {
            status: Arc::new(ArcSwap::from_pointee(SystemStatus::default())),
            capture_status: Arc::new(ArcSwap::from_pointee(CaptureStatus::default())),
            target_volume: Arc::new(AtomicU32::new(0)),
            volume_pending: Arc::new(AtomicBool::new(false)),
            volume_notify: Arc::new(tokio::sync::Notify::new()),
            target_input_volume: Arc::new(AtomicU32::new(0)),
            input_volume_pending: Arc::new(AtomicBool::new(false)),
            input_volume_notify: Arc::new(tokio::sync::Notify::new()),
            audio_changed: Arc::new(tokio::sync::Notify::new()),
            usage_sample: Arc::new(tokio::sync::Mutex::new(None)),
        };

        crate::spawn_tokio(capture::monitor(service.capture_status.clone()));

        let service_clone = service.clone();
        crate::spawn_tokio(async move {
            let _ = service_clone.refresh().await;
            let mut interval = tokio::time::interval(std::time::Duration::from_secs(2));
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                let _ = service_clone.refresh_audio().await;
            }
        });

        let service_events = service.clone();
        crate::spawn_tokio(async move {
            service_events.listen_pactl_events().await;
        });

        let service_volume = service.clone();
        crate::spawn_tokio(async move {
            service_volume.run_volume_worker().await;
        });

        let service_input = service.clone();
        crate::spawn_tokio(async move {
            service_input.run_input_volume_worker().await;
        });

        service
    }

    pub async fn reset_usage_sample(&self) {
        *self.usage_sample.lock().await = None;
    }

    pub async fn sample_usage(&self) -> UsageStatus {
        let mut previous = self.usage_sample.lock().await;
        usage::sample(&mut previous).await
    }

    pub fn get_status(&self) -> SystemStatus {
        (**self.status.load()).clone()
    }

    pub fn get_capture_status(&self) -> CaptureStatus {
        **self.capture_status.load()
    }

    pub fn audio_changed(&self) -> Arc<tokio::sync::Notify> {
        self.audio_changed.clone()
    }

    pub async fn monitor_audio_levels(sender: tokio::sync::watch::Sender<[f32; 12]>) {
        loop {
            if sender.is_closed() {
                return;
            }
            if let Ok(mut child) = Command::new("parec")
                .args([
                    "--device=@DEFAULT_MONITOR@",
                    "--format=s16le",
                    "--rate=16000",
                    "--channels=1",
                    "--latency-msec=50",
                    "--raw",
                ])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
            {
                if let Some(mut stdout) = child.stdout.take() {
                    let mut frame = [0_u8; 1536];
                    loop {
                        tokio::select! {
                            _ = sender.closed() => return,
                            result = stdout.read_exact(&mut frame) => {
                                if result.is_err() { break; }
                                if sender.send(Self::audio_levels(&frame)).is_err() { return; }
                            }
                        }
                    }
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            let _ = sender.send([0.0; 12]);
            tokio::select! {
                _ = sender.closed() => return,
                _ = tokio::time::sleep(std::time::Duration::from_secs(2)) => {}
            }
        }
    }

    fn audio_levels(frame: &[u8; 1536]) -> [f32; 12] {
        std::array::from_fn(|index| {
            let samples = &frame[index * 128..(index + 1) * 128];
            let energy = samples
                .chunks_exact(2)
                .map(|sample| {
                    let value = f32::from(i16::from_le_bytes([sample[0], sample[1]])) / 32768.0;
                    value * value
                })
                .sum::<f32>()
                / 64.0;
            let level = (energy.sqrt() * 4.0).clamp(0.0, 1.0);
            (level * 24.0).round() / 24.0
        })
    }

    async fn run_volume_worker(&self) {
        loop {
            self.volume_notify.notified().await;
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;

            if self.volume_pending.swap(false, Ordering::SeqCst) {
                let target = self.target_volume.load(Ordering::SeqCst);
                let percent_str = format!("{target}%");

                if Command::new("wpctl")
                    .args(["set-volume", "@DEFAULT_AUDIO_SINK@", &percent_str])
                    .status()
                    .await
                    .is_err()
                {
                    let _ = Command::new("pactl")
                        .args(["set-sink-volume", "@DEFAULT_SINK@", &percent_str])
                        .status()
                        .await;
                }
            }
        }
    }

    async fn run_input_volume_worker(&self) {
        loop {
            self.input_volume_notify.notified().await;
            tokio::time::sleep(std::time::Duration::from_millis(30)).await;

            if self.input_volume_pending.swap(false, Ordering::SeqCst) {
                let target = self.target_input_volume.load(Ordering::SeqCst);
                let percent_str = format!("{target}%");

                if Command::new("wpctl")
                    .args(["set-volume", "@DEFAULT_AUDIO_SOURCE@", &percent_str])
                    .status()
                    .await
                    .is_err()
                {
                    let _ = Command::new("pactl")
                        .args(["set-source-volume", "@DEFAULT_SOURCE@", &percent_str])
                        .status()
                        .await;
                }
            }
        }
    }

    pub fn set_volume_fast(&self, percent: u32) {
        let percent = percent.min(100);
        let mut current = (**self.status.load()).clone();
        current.volume = percent;
        if percent > 0 {
            current.is_muted = false;
        }
        self.status.store(Arc::new(current));

        self.target_volume.store(percent, Ordering::SeqCst);
        self.volume_pending.store(true, Ordering::SeqCst);
        self.volume_notify.notify_one();
        self.audio_changed.notify_waiters();
    }

    pub fn set_input_volume_fast(&self, percent: u32) {
        let percent = percent.min(100);
        let mut current = (**self.status.load()).clone();
        current.input_volume = percent;
        if percent > 0 {
            current.is_input_muted = false;
        }
        self.status.store(Arc::new(current));

        self.target_input_volume.store(percent, Ordering::SeqCst);
        self.input_volume_pending.store(true, Ordering::SeqCst);
        self.input_volume_notify.notify_one();
        self.audio_changed.notify_waiters();
    }

    pub async fn refresh_audio(&self) -> Result<()> {
        let output = Self::fetch_audio_status().await;
        let input = Self::fetch_input_audio_status().await;
        let mut current = (**self.status.load()).clone();
        let (volume, is_muted) = output.unwrap_or((current.volume, current.is_muted));
        let (input_volume, is_input_muted) =
            input.unwrap_or((current.input_volume, current.is_input_muted));
        if current.volume != volume
            || current.is_muted != is_muted
            || current.input_volume != input_volume
            || current.is_input_muted != is_input_muted
        {
            current.volume = volume;
            current.is_muted = is_muted;
            current.input_volume = input_volume;
            current.is_input_muted = is_input_muted;
            self.status.store(Arc::new(current));
            self.audio_changed.notify_waiters();
        }
        Ok(())
    }

    pub async fn refresh(&self) -> Result<()> {
        let output = Self::fetch_audio_status().await;
        let input = Self::fetch_input_audio_status().await;
        let brightness = Self::fetch_brightness().await.unwrap_or(100);
        let audio_sinks = Self::fetch_audio_sinks().await;
        let audio_sources = Self::fetch_audio_sources().await;

        let current = self.status.load();
        let vol_muted = output.unwrap_or((current.volume, current.is_muted));
        let input_vol_muted = input.unwrap_or((current.input_volume, current.is_input_muted));
        let new_status = SystemStatus {
            volume: vol_muted.0,
            is_muted: vol_muted.1,
            input_volume: input_vol_muted.0,
            is_input_muted: input_vol_muted.1,
            brightness,
            audio_sinks,
            audio_sources,
        };

        self.status.store(Arc::new(new_status));
        self.audio_changed.notify_waiters();

        Ok(())
    }

    async fn listen_pactl_events(&self) {
        loop {
            if let Ok(mut child) = Command::new("pactl")
                .arg("subscribe")
                .env("LC_ALL", "C")
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .kill_on_drop(true)
                .spawn()
            {
                if let Some(stdout) = child.stdout.take() {
                    let _ = self.refresh().await;
                    let mut reader = BufReader::new(stdout).lines();
                    while let Ok(Some(line)) = reader.next_line().await {
                        if line.contains("sink")
                            || line.contains("source")
                            || line.contains("server")
                        {
                            let _ = self.refresh().await;
                        }
                    }
                }
                let _ = child.kill().await;
                let _ = child.wait().await;
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }
    }

    pub async fn set_volume(&self, percent: u32) -> Result<()> {
        self.set_volume_fast(percent);
        Ok(())
    }

    pub async fn set_input_volume(&self, percent: u32) -> Result<()> {
        self.set_input_volume_fast(percent);
        Ok(())
    }

    pub async fn set_default_sink(&self, sink_name: &str) -> Result<()> {
        if Command::new("pactl")
            .args(["set-default-sink", sink_name])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("wpctl")
                .args(["set-default", sink_name])
                .status()
                .await;
        }
        self.refresh().await
    }

    pub async fn set_default_source(&self, source_name: &str) -> Result<()> {
        if Command::new("pactl")
            .args(["set-default-source", source_name])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("wpctl")
                .args(["set-default", source_name])
                .status()
                .await;
        }
        self.refresh().await
    }

    pub async fn toggle_mute(&self) -> Result<()> {
        let is_muted = {
            let mut current = (**self.status.load()).clone();
            current.is_muted = !current.is_muted;
            let res = current.is_muted;
            self.status.store(Arc::new(current));
            res
        };

        if Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SINK@", "toggle"])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("pactl")
                .args(["set-sink-mute", "@DEFAULT_SINK@", "toggle"])
                .status()
                .await;
        }

        let _ = is_muted;
        self.refresh().await
    }

    pub async fn toggle_input_mute(&self) -> Result<()> {
        let is_muted = {
            let mut current = (**self.status.load()).clone();
            current.is_input_muted = !current.is_input_muted;
            let res = current.is_input_muted;
            self.status.store(Arc::new(current));
            res
        };

        if Command::new("wpctl")
            .args(["set-mute", "@DEFAULT_AUDIO_SOURCE@", "toggle"])
            .status()
            .await
            .is_err()
        {
            let _ = Command::new("pactl")
                .args(["set-source-mute", "@DEFAULT_SOURCE@", "toggle"])
                .status()
                .await;
        }

        let _ = is_muted;
        self.refresh().await
    }

    pub async fn set_brightness(&self, percent: u32) -> Result<()> {
        let percent = percent.min(100).max(5);
        let percent_str = format!("{percent}%");

        let output = Command::new("brightnessctl")
            .args(["set", &percent_str])
            .output()
            .await
            .context("Could not run brightnessctl")?;
        anyhow::ensure!(
            output.status.success(),
            "Could not set brightness: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );

        self.refresh().await
    }

    pub async fn lock() -> Result<()> {
        if Command::new("hyprlock").status().await.is_err()
            && Command::new("swaylock").status().await.is_err()
        {
            Command::new("loginctl")
                .arg("lock-session")
                .status()
                .await
                .context("Failed to lock session")?;
        }
        Ok(())
    }

    pub async fn suspend() -> Result<()> {
        Command::new("systemctl")
            .arg("suspend")
            .status()
            .await
            .context("Failed to suspend system")?;
        Ok(())
    }

    pub async fn reboot() -> Result<()> {
        let output = Command::new("systemctl")
            .arg("reboot")
            .output()
            .await
            .context("Failed to reboot system")?;
        anyhow::ensure!(
            output.status.success(),
            "Failed to reboot system: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    pub async fn logout() -> Result<()> {
        let session = std::env::var("XDG_SESSION_ID").context("No login session is available")?;
        anyhow::ensure!(!session.trim().is_empty(), "No login session is available");
        let output = Command::new("loginctl")
            .args(["terminate-session", &session])
            .output()
            .await
            .context("Failed to log out")?;
        anyhow::ensure!(
            output.status.success(),
            "Failed to log out: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    pub async fn poweroff() -> Result<()> {
        let output = Command::new("systemctl")
            .arg("poweroff")
            .output()
            .await
            .context("Failed to poweroff system")?;
        anyhow::ensure!(
            output.status.success(),
            "Failed to poweroff system: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        );
        Ok(())
    }

    async fn fetch_audio_status() -> Result<(u32, bool)> {
        if let Ok(output) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
            .env("LC_ALL", "C")
            .output()
            .await
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let is_muted = text.contains("[MUTED]");
            if let Some(vol_str) = text.split_whitespace().nth(1) {
                if let Ok(val) = vol_str.parse::<f32>() {
                    return Ok(((val * 100.0).round() as u32, is_muted));
                }
            }
        }

        if let Ok(output) = Command::new("pactl")
            .args(["get-sink-volume", "@DEFAULT_SINK@"])
            .env("LC_ALL", "C")
            .output()
            .await
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(pos) = text.find('/') {
                let rest = &text[pos + 1..];
                if let Some(pos2) = rest.find('%') {
                    if let Ok(val) = rest[..pos2].trim().parse::<u32>() {
                        return Ok((val, false));
                    }
                }
            }
        }

        anyhow::bail!("Could not read output audio volume")
    }

    async fn fetch_brightness() -> Result<u32> {
        if let Ok(output) = Command::new("brightnessctl").arg("info").output().await {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(pos) = text.find('(') {
                let rest = &text[pos + 1..];
                if let Some(pos2) = rest.find('%') {
                    if let Ok(val) = rest[..pos2].parse::<u32>() {
                        return Ok(val);
                    }
                }
            }
        }
        Ok(100)
    }

    async fn fetch_audio_sinks() -> Vec<AudioSink> {
        let default_sink = Command::new("pactl")
            .arg("get-default-sink")
            .output()
            .await
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .unwrap_or_default();

        let mut sinks = Vec::new();
        if let Ok(output) = Command::new("pactl")
            .arg("list")
            .arg("sinks")
            .output()
            .await
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_desc = String::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    if !current_name.is_empty() {
                        let is_def = current_name == default_sink;
                        sinks.push(AudioSink {
                            name: current_name.clone(),
                            description: if current_desc.is_empty() {
                                current_name.clone()
                            } else {
                                current_desc.clone()
                            },
                            is_default: is_def,
                        });
                        current_desc.clear();
                    }
                    current_name = trimmed.trim_start_matches("Name: ").trim().to_string();
                } else if trimmed.starts_with("Description: ") {
                    current_desc = trimmed
                        .trim_start_matches("Description: ")
                        .trim()
                        .to_string();
                }
            }

            if !current_name.is_empty() {
                let is_def = current_name == default_sink;
                sinks.push(AudioSink {
                    name: current_name,
                    description: if current_desc.is_empty() {
                        "Audio Output".to_string()
                    } else {
                        current_desc
                    },
                    is_default: is_def,
                });
            }
        }
        sinks
    }

    async fn fetch_input_audio_status() -> Result<(u32, bool)> {
        if let Ok(output) = Command::new("wpctl")
            .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
            .env("LC_ALL", "C")
            .output()
            .await
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let is_muted = text.contains("[MUTED]");
            if let Some(vol_str) = text.split_whitespace().nth(1) {
                if let Ok(val) = vol_str.parse::<f32>() {
                    return Ok(((val * 100.0).round() as u32, is_muted));
                }
            }
        }

        if let Ok(output) = Command::new("pactl")
            .args(["get-source-volume", "@DEFAULT_SOURCE@"])
            .env("LC_ALL", "C")
            .output()
            .await
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            if let Some(pos) = text.find('/') {
                let rest = &text[pos + 1..];
                if let Some(pos2) = rest.find('%') {
                    if let Ok(val) = rest[..pos2].trim().parse::<u32>() {
                        return Ok((val, false));
                    }
                }
            }
        }

        anyhow::bail!("Could not read input audio volume")
    }

    async fn fetch_audio_sources() -> Vec<AudioSource> {
        let default_source = Command::new("pactl")
            .arg("get-default-source")
            .output()
            .await
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
            .unwrap_or_default();

        let mut sources = Vec::new();
        if let Ok(output) = Command::new("pactl")
            .arg("list")
            .arg("sources")
            .output()
            .await
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let mut current_name = String::new();
            let mut current_desc = String::new();

            for line in text.lines() {
                let trimmed = line.trim();
                if trimmed.starts_with("Name: ") {
                    if !current_name.is_empty() && !current_name.ends_with(".monitor") {
                        let is_def = current_name == default_source;
                        sources.push(AudioSource {
                            name: current_name.clone(),
                            description: if current_desc.is_empty() {
                                current_name.clone()
                            } else {
                                current_desc.clone()
                            },
                            is_default: is_def,
                        });
                        current_desc.clear();
                    }
                    current_name = trimmed.trim_start_matches("Name: ").trim().to_string();
                } else if trimmed.starts_with("Description: ") {
                    current_desc = trimmed
                        .trim_start_matches("Description: ")
                        .trim()
                        .to_string();
                }
            }

            if !current_name.is_empty() && !current_name.ends_with(".monitor") {
                let is_def = current_name == default_source;
                sources.push(AudioSource {
                    name: current_name,
                    description: if current_desc.is_empty() {
                        "Audio Input".to_string()
                    } else {
                        current_desc
                    },
                    is_default: is_def,
                });
            }
        }
        sources
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn audio_visualizer_tracks_pcm_energy() {
        assert_eq!(SystemService::audio_levels(&[0; 1536]), [0.0; 12]);
        let mut frame = [0_u8; 1536];
        for sample in frame[..128].chunks_exact_mut(2) {
            sample.copy_from_slice(&8192_i16.to_le_bytes());
        }
        let levels = SystemService::audio_levels(&frame);
        assert_eq!(levels[0], 1.0);
        assert_eq!(&levels[1..], &[0.0; 11]);
        for sample in frame.chunks_exact_mut(2) {
            sample.copy_from_slice(&i16::MIN.to_le_bytes());
        }
        assert_eq!(SystemService::audio_levels(&frame), [1.0; 12]);
    }

    #[test]
    fn audio_refresh_recovers_and_preserves_last_volume() -> Result<()> {
        const FIXTURE_ENV: &str = "CAPSULE_AUDIO_TEST_DIR";
        if let Some(directory) = std::env::var_os(FIXTURE_ENV) {
            let directory = std::path::PathBuf::from(directory);
            return tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()?
                .block_on(async {
                    let service = SystemService {
                        status: Arc::new(ArcSwap::from_pointee(SystemStatus::default())),
                        capture_status: Arc::new(ArcSwap::from_pointee(CaptureStatus::default())),
                        target_volume: Arc::new(AtomicU32::new(0)),
                        volume_pending: Arc::new(AtomicBool::new(false)),
                        volume_notify: Arc::new(tokio::sync::Notify::new()),
                        target_input_volume: Arc::new(AtomicU32::new(0)),
                        input_volume_pending: Arc::new(AtomicBool::new(false)),
                        input_volume_notify: Arc::new(tokio::sync::Notify::new()),
                        audio_changed: Arc::new(tokio::sync::Notify::new()),
                        usage_sample: Arc::new(tokio::sync::Mutex::new(None)),
                    };
                    assert!(SystemService::fetch_audio_status().await.is_err());
                    assert!(SystemService::fetch_input_audio_status().await.is_err());
                    service.refresh().await?;
                    assert_eq!(service.get_status().volume, 0);

                    let sink = directory.join("@DEFAULT_AUDIO_SINK@");
                    let source = directory.join("@DEFAULT_AUDIO_SOURCE@");
                    std::fs::write(&sink, "Volume: 0.83 [MUTED]\n")?;
                    service.refresh_audio().await?;
                    assert_eq!(service.get_status().volume, 83);
                    assert!(service.get_status().is_muted);

                    std::fs::write(&source, "Volume: 0.27 [MUTED]\n")?;
                    service.refresh().await?;
                    assert_eq!(service.get_status().input_volume, 27);
                    assert!(service.get_status().is_input_muted);

                    std::fs::write(directory.join("fail"), "")?;
                    std::fs::write(&sink, "Volume: 0.50\n")?;
                    std::fs::write(&source, "Volume: 0.50\n")?;
                    service.refresh_audio().await?;
                    service.refresh().await?;
                    assert_eq!(service.get_status().volume, 83);
                    assert!(service.get_status().is_muted);
                    assert_eq!(service.get_status().input_volume, 27);
                    assert!(service.get_status().is_input_muted);

                    std::fs::remove_file(directory.join("fail"))?;
                    std::fs::write(&sink, "Volume: 0.64\n")?;
                    service.refresh_audio().await?;
                    assert_eq!(service.get_status().volume, 64);
                    assert!(!service.get_status().is_muted);
                    Ok(())
                });
        }

        let directory = std::env::temp_dir().join(format!(
            "capsule-audio-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        ));
        std::fs::create_dir(&directory)?;
        for (name, script) in [
            (
                "wpctl",
                "#!/bin/sh\n/bin/cat \"$CAPSULE_AUDIO_TEST_DIR/$2\" || exit 1\n[ ! -f \"$CAPSULE_AUDIO_TEST_DIR/fail\" ]\n",
            ),
            ("pactl", "#!/bin/sh\nexit 1\n"),
            ("brightnessctl", "#!/bin/sh\nexit 1\n"),
        ] {
            let path = directory.join(name);
            std::fs::write(&path, script)?;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700))?;
        }
        let result = std::process::Command::new(std::env::current_exe()?)
            .args([
                "--exact",
                "system::tests::audio_refresh_recovers_and_preserves_last_volume",
                "--nocapture",
            ])
            .env(FIXTURE_ENV, &directory)
            .env("PATH", &directory)
            .output();
        std::fs::remove_dir_all(&directory)?;
        let output = result?;
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(())
    }
}
