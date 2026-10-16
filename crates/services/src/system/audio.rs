use anyhow::Result;
use std::process::Stdio;
use tokio::io::{AsyncBufReadExt, AsyncReadExt, BufReader};
use tokio::process::Command;

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
pub struct AppStream {
    pub index: u32,
    pub name: String,
    pub icon_name: Option<String>,
    pub volume: u32,
    pub muted: bool,
    pub sink: String,
}

pub fn parse_app_streams(inputs: &str, sinks: &str) -> Vec<AppStream> {
    let sink_names: std::collections::HashMap<u64, String> =
        serde_json::from_str::<Vec<serde_json::Value>>(sinks)
            .unwrap_or_default()
            .iter()
            .filter_map(|sink| Some((sink["index"].as_u64()?, sink["name"].as_str()?.to_string())))
            .collect();
    serde_json::from_str::<Vec<serde_json::Value>>(inputs)
        .unwrap_or_default()
        .iter()
        .filter_map(|input| {
            let properties = &input["properties"];
            let name = properties["application.name"]
                .as_str()
                .or_else(|| properties["application.process.binary"].as_str())?
                .to_string();
            let volume = input["volume"]
                .as_object()
                .and_then(|channels| channels.values().next())
                .and_then(|channel| channel["value_percent"].as_str())
                .and_then(|percent| percent.trim_end_matches('%').parse().ok())
                .unwrap_or(100);
            Some(AppStream {
                index: input["index"].as_u64()? as u32,
                name,
                icon_name: properties["application.icon_name"]
                    .as_str()
                    .map(str::to_string),
                volume,
                muted: input["mute"].as_bool().unwrap_or(false),
                sink: input["sink"]
                    .as_u64()
                    .and_then(|sink| sink_names.get(&sink).cloned())
                    .unwrap_or_default(),
            })
        })
        .collect()
}

async fn pactl_json(args: &[&str]) -> String {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Command::new("pactl")
            .args(["-f", "json"])
            .args(args)
            .output(),
    )
    .await;
    match output {
        Ok(Ok(output)) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).into_owned()
        }
        _ => String::new(),
    }
}

pub async fn fetch_app_streams() -> Vec<AppStream> {
    let (inputs, sinks) = tokio::join!(
        pactl_json(&["list", "sink-inputs"]),
        pactl_json(&["list", "sinks", "short"]),
    );
    parse_app_streams(&inputs, &sinks)
}

pub async fn set_stream_volume(index: u32, percent: u32) -> Result<()> {
    run_pactl(&[
        "set-sink-input-volume",
        &index.to_string(),
        &format!("{percent}%"),
    ])
    .await
}

pub async fn toggle_stream_mute(index: u32) -> Result<()> {
    run_pactl(&["set-sink-input-mute", &index.to_string(), "toggle"]).await
}

pub async fn move_stream(index: u32, sink_name: &str) -> Result<()> {
    run_pactl(&["move-sink-input", &index.to_string(), sink_name]).await
}

async fn run_pactl(args: &[&str]) -> Result<()> {
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        Command::new("pactl").args(args).output(),
    )
    .await??;
    anyhow::ensure!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
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
                            if sender.send(audio_levels(&frame)).is_err() { return; }
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

pub fn audio_levels(frame: &[u8; 1536]) -> [f32; 12] {
    std::array::from_fn(|index| {
        let samples = &frame[index * 128..(index + 1) * 128];
        let energy = samples
            .as_chunks::<2>()
            .0
            .iter()
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

pub async fn apply_volume(target: u32) {
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

pub async fn apply_input_volume(target: u32) {
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

pub async fn apply_toggle_mute() {
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
}

pub async fn apply_toggle_input_mute() {
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
}

pub async fn set_default_sink(sink_name: &str) -> Result<()> {
    let pactl_ok = if let Ok(status) = Command::new("pactl")
        .args(["set-default-sink", sink_name])
        .status()
        .await
    {
        status.success()
    } else {
        false
    };

    if !pactl_ok {
        let _ = Command::new("wpctl")
            .args(["set-default", sink_name])
            .status()
            .await;
    }
    Ok(())
}

pub async fn set_default_source(source_name: &str) -> Result<()> {
    let pactl_ok = if let Ok(status) = Command::new("pactl")
        .args(["set-default-source", source_name])
        .status()
        .await
    {
        status.success()
    } else {
        false
    };

    if !pactl_ok {
        let _ = Command::new("wpctl")
            .args(["set-default", source_name])
            .status()
            .await;
    }
    Ok(())
}

pub async fn fetch_audio_status() -> Result<(u32, bool)> {
    if let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SINK@"])
        .env("LC_ALL", "C")
        .output()
        .await
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let is_muted = text.contains("[MUTED]");
        if let Some(vol_str) = text.split_whitespace().nth(1)
            && let Ok(val) = vol_str.parse::<f32>()
        {
            return Ok(((val * 100.0).round() as u32, is_muted));
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
            if let Some(pos2) = rest.find('%')
                && let Ok(val) = rest[..pos2].trim().parse::<u32>()
            {
                return Ok((val, false));
            }
        }
    }

    anyhow::bail!("Could not read output audio volume")
}

pub async fn fetch_input_audio_status() -> Result<(u32, bool)> {
    if let Ok(output) = Command::new("wpctl")
        .args(["get-volume", "@DEFAULT_AUDIO_SOURCE@"])
        .env("LC_ALL", "C")
        .output()
        .await
        && output.status.success()
    {
        let text = String::from_utf8_lossy(&output.stdout);
        let is_muted = text.contains("[MUTED]");
        if let Some(vol_str) = text.split_whitespace().nth(1)
            && let Ok(val) = vol_str.parse::<f32>()
        {
            return Ok(((val * 100.0).round() as u32, is_muted));
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
            if let Some(pos2) = rest.find('%')
                && let Ok(val) = rest[..pos2].trim().parse::<u32>()
            {
                return Ok((val, false));
            }
        }
    }

    anyhow::bail!("Could not read input audio volume")
}

pub async fn fetch_audio_sinks() -> Vec<AudioSink> {
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
    if sinks.is_empty() {
        sinks = parse_wpctl_sinks().await;
    }
    sinks
}

pub async fn parse_wpctl_sinks() -> Vec<AudioSink> {
    let mut sinks = Vec::new();
    if let Ok(output) = Command::new("wpctl")
        .arg("status")
        .env("LC_ALL", "C")
        .output()
        .await
    {
        if !output.status.success() {
            return sinks;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut in_sinks = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Sinks:") {
                in_sinks = true;
                continue;
            }
            if in_sinks {
                if trimmed.contains("Sources:")
                    || trimmed.contains("Filters:")
                    || trimmed.contains("Streams:")
                    || trimmed.starts_with("├─")
                    || trimmed.starts_with("└─")
                {
                    break;
                }
                if let Some(dot_pos) = trimmed.find('.') {
                    let prefix = trimmed[..dot_pos]
                        .trim_start_matches(['│', ' ', '*'])
                        .trim();
                    if let Ok(id) = prefix.parse::<u32>() {
                        let is_default = trimmed.contains('*');
                        let rest = trimmed[dot_pos + 1..].trim();
                        let desc = if let Some(bracket_pos) = rest.rfind('[') {
                            rest[..bracket_pos].trim().to_string()
                        } else {
                            rest.to_string()
                        };

                        sinks.push(AudioSink {
                            name: id.to_string(),
                            description: if desc.is_empty() {
                                format!("Output {id}")
                            } else {
                                desc
                            },
                            is_default,
                        });
                    }
                }
            }
        }
    }
    sinks
}

pub async fn fetch_audio_sources() -> Vec<AudioSource> {
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
    if sources.is_empty() {
        sources = parse_wpctl_sources().await;
    }
    sources
}

pub async fn parse_wpctl_sources() -> Vec<AudioSource> {
    let mut sources = Vec::new();
    if let Ok(output) = Command::new("wpctl")
        .arg("status")
        .env("LC_ALL", "C")
        .output()
        .await
    {
        if !output.status.success() {
            return sources;
        }
        let text = String::from_utf8_lossy(&output.stdout);
        let mut in_sources = false;

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.contains("Sources:") {
                in_sources = true;
                continue;
            }
            if in_sources {
                if trimmed.contains("Filters:")
                    || trimmed.contains("Streams:")
                    || trimmed.starts_with("├─")
                    || trimmed.starts_with("└─")
                {
                    break;
                }
                if let Some(dot_pos) = trimmed.find('.') {
                    let prefix = trimmed[..dot_pos]
                        .trim_start_matches(['│', ' ', '*'])
                        .trim();
                    if let Ok(id) = prefix.parse::<u32>() {
                        let is_default = trimmed.contains('*');
                        let rest = trimmed[dot_pos + 1..].trim();
                        let desc = if let Some(bracket_pos) = rest.rfind('[') {
                            rest[..bracket_pos].trim().to_string()
                        } else {
                            rest.to_string()
                        };

                        sources.push(AudioSource {
                            name: id.to_string(),
                            description: if desc.is_empty() {
                                format!("Input {id}")
                            } else {
                                desc
                            },
                            is_default,
                        });
                    }
                }
            }
        }
    }
    sources
}

pub async fn listen_pactl_events(
    on_change: impl Fn() -> tokio::task::JoinHandle<()> + Send + Sync + 'static,
) {
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
                drop(on_change());
                let mut reader = BufReader::new(stdout).lines();
                while let Ok(Some(line)) = reader.next_line().await {
                    if line.contains("sink") || line.contains("source") || line.contains("server") {
                        drop(on_change());
                    }
                }
            }
            let _ = child.kill().await;
            let _ = child.wait().await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
}

#[cfg(test)]
mod stream_tests {
    use super::*;

    #[test]
    fn only_application_streams_are_listed_with_their_output() {
        let inputs = r#"[
            {"index":72,"sink":4294967295,"mute":false,"volume":{"aux0":{"value_percent":"100%"}},"properties":{"media.name":"Speakers DSP Out"}},
            {"index":80,"sink":58,"mute":true,"volume":{"front-left":{"value_percent":"42%"}},"properties":{"application.name":"Firefox","application.icon_name":"firefox"}}
        ]"#;
        let sinks = r#"[{"index":58,"name":"alsa_output.headphones"}]"#;
        let streams = parse_app_streams(inputs, sinks);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "Firefox");
        assert_eq!(streams[0].volume, 42);
        assert!(streams[0].muted);
        assert_eq!(streams[0].sink, "alsa_output.headphones");
        assert_eq!(streams[0].icon_name.as_deref(), Some("firefox"));
    }
}
