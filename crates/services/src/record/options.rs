use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Stdio;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordStatus {
    Recording,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordBackend {
    GpuScreenRecorder,
    WlScreenRec,
}

impl RecordBackend {
    pub fn as_str(&self) -> &'static str {
        match self {
            RecordBackend::GpuScreenRecorder => "gpu-screen-recorder",
            RecordBackend::WlScreenRec => "wl-screenrec",
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            RecordBackend::GpuScreenRecorder => std::process::Command::new("which")
                .arg("gpu-screen-recorder")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
            RecordBackend::WlScreenRec => std::process::Command::new("which")
                .arg("wl-screenrec")
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .map(|s| s.success())
                .unwrap_or(false),
        }
    }

    pub fn detect_preferred() -> Option<Self> {
        if RecordBackend::GpuScreenRecorder.is_available() {
            Some(RecordBackend::GpuScreenRecorder)
        } else if RecordBackend::WlScreenRec.is_available() {
            Some(RecordBackend::WlScreenRec)
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordTarget {
    Screen,
    Monitor(String),
    Region {
        x: i32,
        y: i32,
        width: i32,
        height: i32,
    },
    Portal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecordAudio {
    None,
    Desktop,
    Microphone,
    Both,
    Custom(String),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordOptions {
    pub backend: RecordBackend,
    pub target: RecordTarget,
    pub audio: RecordAudio,
    pub fps: u32,
    pub resolution: String,
    pub quality: String,
    pub container: String,
    pub output_path: Option<PathBuf>,
    pub include_cursor: bool,
}

impl Default for RecordOptions {
    fn default() -> Self {
        Self {
            backend: RecordBackend::GpuScreenRecorder,
            target: RecordTarget::Screen,
            audio: RecordAudio::Desktop,
            fps: 60,
            resolution: "native".to_string(),
            quality: "very_high".to_string(),
            container: "mp4".to_string(),
            output_path: None,
            include_cursor: true,
        }
    }
}

impl RecordOptions {
    pub fn from_config(config: &crate::config::RecordConfig) -> Self {
        let audio = match config.audio.as_str() {
            "mic" | "microphone" => RecordAudio::Microphone,
            "both" => RecordAudio::Both,
            "none" | "off" => RecordAudio::None,
            _ => RecordAudio::Desktop,
        };

        let target = if config.output == "screen" || config.output.is_empty() {
            RecordTarget::Screen
        } else {
            RecordTarget::Monitor(config.output.clone())
        };

        Self {
            backend: RecordBackend::detect_preferred().unwrap_or(RecordBackend::GpuScreenRecorder),
            target,
            audio,
            fps: config.fps,
            resolution: config.resolution.clone(),
            quality: config.quality.clone(),
            container: config.container.clone(),
            output_path: None,
            include_cursor: config.include_cursor,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::RecordConfig;

    #[test]
    fn test_record_options_default() {
        let opts = RecordOptions::default();
        assert_eq!(opts.backend, RecordBackend::GpuScreenRecorder);
        assert_eq!(opts.target, RecordTarget::Screen);
        assert_eq!(opts.fps, 60);
        assert_eq!(opts.container, "mp4");
        assert_eq!(opts.quality, "very_high");
        assert!(opts.include_cursor);
    }

    #[test]
    fn test_record_options_from_config() {
        let config = RecordConfig {
            output: "screen".to_string(),
            fps: 30,
            resolution: "1920x1080".to_string(),
            quality: "high".to_string(),
            container: "mkv".to_string(),
            audio: "mic".to_string(),
            include_cursor: false,
        };

        let opts = RecordOptions::from_config(&config);
        assert_eq!(opts.target, RecordTarget::Screen);
        assert_eq!(opts.fps, 30);
        assert_eq!(opts.resolution, "1920x1080");
        assert_eq!(opts.quality, "high");
        assert_eq!(opts.container, "mkv");
        assert_eq!(opts.audio, RecordAudio::Microphone);
        assert!(!opts.include_cursor);
    }

    #[test]
    fn test_backend_as_str() {
        assert_eq!(
            RecordBackend::GpuScreenRecorder.as_str(),
            "gpu-screen-recorder"
        );
        assert_eq!(RecordBackend::WlScreenRec.as_str(), "wl-screenrec");
    }
}
