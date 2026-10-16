use gpui::{
    AnyElement, App, AppContext, Context, IntoElement, ParentElement, Render, Window, div,
    prelude::FluentBuilder,
};
use services::{AppState, Display, SystemStatus};
use ui::components::{Slider, SliderRow, caption, panel};
use ui::theme::Theme;

use crate::bar::volume_icon;
use crate::shell::Surface;
use crate::shell::overlay::{Content, Overlay};
use crate::shell::state::ShellState;

pub fn run_command(command: &str) {
    let command = command.trim().to_string();
    if command.is_empty() {
        return;
    }
    services::spawn_tokio(async move {
        let mut process = tokio::process::Command::new("sh");
        process
            .arg("-c")
            .arg(&command)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        if let Err(error) = process.status().await {
            services::log_error!("Shell", "Failed to run '{command}': {error}");
        }
    });
}

pub fn output_label(audio: &SystemStatus) -> String {
    audio
        .audio_sinks
        .iter()
        .find(|sink| sink.is_default)
        .map(|sink| sink.description.clone())
        .unwrap_or_else(|| "No output".to_string())
}

pub fn input_label(audio: &SystemStatus) -> String {
    audio
        .audio_sources
        .iter()
        .find(|source| source.is_default)
        .map(|source| source.description.clone())
        .unwrap_or_else(|| "No input".to_string())
}

pub fn volume_row(theme: &Theme, audio: &SystemStatus) -> AnyElement {
    let available = !audio.audio_sinks.is_empty();
    SliderRow::new(
        SliderRow::mute_button(
            "volume-mute",
            theme,
            volume_icon(audio.volume, audio.is_muted),
            audio.is_muted,
        )
        .disabled(!available)
        .on_click(|_, _, cx| {
            let system = cx.global::<AppState>().system.clone();
            services::spawn_tokio(async move {
                let _ = system.toggle_mute().await;
            });
        }),
        output_label(audio),
        Slider::new("volume-slider")
            .disabled(!available)
            .when(audio.is_muted, |slider| slider.color(theme.text_muted()))
            .on_change(|fraction, _, cx| {
                let percent = (fraction * 100.0).round() as u32;
                cx.global::<AppState>().system.set_volume_fast(percent);
                ShellState::refresh_now(cx);
            }),
        audio.volume.min(100) as f32 / 100.0,
    )
    .into_any_element()
}

pub fn input_row(theme: &Theme, audio: &SystemStatus) -> AnyElement {
    let available = !audio.audio_sources.is_empty();
    SliderRow::new(
        SliderRow::mute_button(
            "input-mute",
            theme,
            if audio.is_input_muted {
                "mic-off.svg"
            } else {
                "mic.svg"
            },
            audio.is_input_muted,
        )
        .disabled(!available)
        .on_click(|_, _, cx| {
            let system = cx.global::<AppState>().system.clone();
            services::spawn_tokio(async move {
                let _ = system.toggle_input_mute().await;
            });
        }),
        input_label(audio),
        Slider::new("input-slider")
            .disabled(!available)
            .when(audio.is_input_muted, |slider| {
                slider.color(theme.text_muted())
            })
            .on_change(|fraction, _, cx| {
                let percent = (fraction * 100.0).round() as u32;
                cx.global::<AppState>()
                    .system
                    .set_input_volume_fast(percent);
                ShellState::refresh_now(cx);
            }),
        audio.input_volume.min(100) as f32 / 100.0,
    )
    .into_any_element()
}

pub fn display_row(theme: &Theme, display: &Display) -> AnyElement {
    let id = display.id.clone();
    SliderRow::new(
        SliderRow::icon(theme, "sun.svg"),
        display.name.clone(),
        Slider::new(gpui::ElementId::Name(
            format!("display-{}", display.id).into(),
        ))
        .on_change(move |fraction, _, cx| {
            let percent = (fraction * 100.0).round() as u32;
            cx.global::<AppState>()
                .displays
                .set_brightness(&id, percent);
            ShellState::refresh_now(cx);
        }),
        display.brightness.min(100) as f32 / 100.0,
    )
    .into_any_element()
}

struct Placeholder {
    title: &'static str,
}

impl Render for Placeholder {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl gpui::IntoElement {
        let theme = cx.global::<Theme>();
        panel(theme)
            .child(div().child(self.title))
            .child(caption(theme, "Coming soon"))
    }
}

pub fn placeholder(surface: Surface, cx: &mut Context<Overlay>) -> Content {
    let title = match surface {
        Surface::Launcher => "Launcher",
        Surface::Polkit => "Authentication",
        Surface::Applet(_) => "Applet",
    };
    Content {
        view: cx.new(|_| Placeholder { title }).into(),
        focus: None,
    }
}

pub fn refresh(cx: &mut App) {
    ShellState::refresh_now(cx);
}
