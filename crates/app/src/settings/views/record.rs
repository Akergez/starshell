use gpui::{
    AnyElement, ColorExt, Context, ElementId, FontWeight, IntoElement, ParentElement, Styled,
    Window, div,
};
use services::{AppState, RecordService};
use ui::theme::Theme;
use ui::tokens::{Spacing, TextSize};

use super::SettingsView;
use super::common::{card, control_row, hero_header, pill_button, row_divider, toggle_row};
use crate::settings::Settings;

pub struct RecordView {
    available_monitors: Vec<String>,
}

impl RecordView {
    pub fn new() -> Self {
        Self {
            available_monitors: RecordService::list_monitors(),
        }
    }
}

impl Default for RecordView {
    fn default() -> Self {
        Self::new()
    }
}

impl SettingsView for RecordView {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Settings>) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let config = if cx.has_global::<AppState>() {
            cx.global::<AppState>().config.get()
        } else {
            Default::default()
        };

        // Output monitors options
        let screen_label = String::from("Full Screen");
        let mut monitor_options = vec![("screen".to_string(), screen_label)];
        for m in &self.available_monitors {
            if m != "screen" {
                monitor_options.push((m.clone(), m.clone()));
            }
        }

        let fps_options = [30u32, 60, 120];

        let native_label = String::from("Native");
        let res_options = [
            ("native", native_label),
            ("1920x1080", "1080p".to_string()),
            ("1280x720", "720p".to_string()),
        ];

        let ultra_label = String::from("Ultra");
        let very_high_label = String::from("Very High");
        let high_label = String::from("High");
        let medium_label = String::from("Medium");
        let quality_options = [
            ("ultra", ultra_label),
            ("very_high", very_high_label),
            ("high", high_label),
            ("medium", medium_label),
        ];

        let container_options = [("mp4", "MP4"), ("mkv", "MKV")];

        let desktop_label = String::from("Desktop");
        let mic_label = String::from("Microphone");
        let both_label = String::from("Both");
        let none_label = String::from("None");
        let audio_options = [
            ("desktop", desktop_label),
            ("mic", mic_label),
            ("both", both_label),
            ("none", none_label),
        ];

        div()
            .flex()
            .flex_col()
            .gap(Spacing::XLarge.rems())
            .w_full()
            .min_w_0()
            // Hero Header
            .child(hero_header(
                "play.svg",
                &String::from("Screen Recording"),
                &String::from("Configure frame rate (FPS), resolution, video quality, and audio source for gpu-screen-recorder."),
                &theme,
            ))
            // Video Output Card
            .child(
                card(&theme).child(control_row(
                    &String::from("Video Output"),
                    Some(&String::from("Monitor or screen to capture during recording.")),
                    div()
                        .flex()
                        .gap(Spacing::Small.rems())
                        .children(monitor_options.into_iter().enumerate().map(|(idx, (val, label))| {
                            let is_selected = config.record.output == val;
                            let val_str = val.clone();
                            pill_button(
                                ElementId::NamedInteger("output-pill".into(), idx as u64),
                                &label,
                                is_selected,
                                &theme,
                                cx.listener(move |_this, _, _, cx| {
                                    if cx.has_global::<AppState>() {
                                        let target = val_str.clone();
                                        let _ = cx.global::<AppState>().config.update(|cfg| {
                                            cfg.record.output = target;
                                        });
                                    }
                                    cx.notify();
                                }),
                            )
                        })),
                    &theme,
                )),
            )
            // Video Capture Settings Card (FPS, Resolution, Quality, Container)
            .child(
                card(&theme)
                    .child(control_row(
                        &String::from("Frames per Second (FPS)"),
                        Some(&String::from("Smoothness of the screen recording capture.")),
                        div()
                            .flex()
                            .gap(Spacing::Small.rems())
                            .children(fps_options.into_iter().enumerate().map(|(idx, fps)| {
                                let is_selected = config.record.fps == fps;
                                pill_button(
                                    ElementId::NamedInteger("fps-pill".into(), idx as u64),
                                    &format!("{fps} FPS"),
                                    is_selected,
                                    &theme,
                                    cx.listener(move |_this, _, _, cx| {
                                        if cx.has_global::<AppState>() {
                                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                                cfg.record.fps = fps;
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                            })),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(control_row(
                        &String::from("Resolution"),
                        Some(&String::from("Capture scale for the selected monitor.")),
                        div()
                            .flex()
                            .gap(Spacing::Small.rems())
                            .children(res_options.into_iter().enumerate().map(|(idx, (val, label))| {
                                let is_selected = config.record.resolution == val;
                                let val_str = val.to_string();
                                pill_button(
                                    ElementId::NamedInteger("res-pill".into(), idx as u64),
                                    &label,
                                    is_selected,
                                    &theme,
                                    cx.listener(move |_this, _, _, cx| {
                                        if cx.has_global::<AppState>() {
                                            let target = val_str.clone();
                                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                                cfg.record.resolution = target;
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                            })),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(control_row(
                        &String::from("Video Quality"),
                        Some(&String::from("Bitrate and compression of the video encoder.")),
                        div()
                            .flex()
                            .gap(Spacing::Small.rems())
                            .children(quality_options.into_iter().enumerate().map(|(idx, (val, label))| {
                                let is_selected = config.record.quality == val;
                                let val_str = val.to_string();
                                pill_button(
                                    ElementId::NamedInteger("quality-pill".into(), idx as u64),
                                    &label,
                                    is_selected,
                                    &theme,
                                    cx.listener(move |_this, _, _, cx| {
                                        if cx.has_global::<AppState>() {
                                            let target = val_str.clone();
                                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                                cfg.record.quality = target;
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                            })),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(control_row(
                        &String::from("Container Format"),
                        Some(&String::from("Final file format (MP4 or MKV).")),
                        div()
                            .flex()
                            .gap(Spacing::Small.rems())
                            .children(container_options.into_iter().enumerate().map(|(idx, (val, label))| {
                                let is_selected = config.record.container == val;
                                let val_str = val.to_string();
                                pill_button(
                                    ElementId::NamedInteger("container-pill".into(), idx as u64),
                                    label,
                                    is_selected,
                                    &theme,
                                    cx.listener(move |_this, _, _, cx| {
                                        if cx.has_global::<AppState>() {
                                            let target = val_str.clone();
                                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                                cfg.record.container = target;
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                            })),
                        &theme,
                    )),
            )
            // Audio & Cursor Card
            .child(
                card(&theme)
                    .child(control_row(
                        &String::from("Audio Source"),
                        Some(&String::from("Default audio captured during recording.")),
                        div()
                            .flex()
                            .gap(Spacing::Small.rems())
                            .children(audio_options.into_iter().enumerate().map(|(idx, (val, label))| {
                                let is_selected = config.record.audio == val;
                                let val_str = val.to_string();
                                pill_button(
                                    ElementId::NamedInteger("audio-pill".into(), idx as u64),
                                    &label,
                                    is_selected,
                                    &theme,
                                    cx.listener(move |_this, _, _, cx| {
                                        if cx.has_global::<AppState>() {
                                            let target = val_str.clone();
                                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                                cfg.record.audio = target;
                                            });
                                        }
                                        cx.notify();
                                    }),
                                )
                            })),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("record-cursor-toggle".into()),
                        &String::from("Capture Cursor"),
                        Some(&String::from("Include mouse pointer in the recording.")),
                        config.record.include_cursor,
                        &theme,
                        cx.listener(|_this, _, _, cx| {
                            if cx.has_global::<AppState>() {
                                let _ = cx.global::<AppState>().config.update(|cfg| {
                                    cfg.record.include_cursor = !cfg.record.include_cursor;
                                });
                            }
                            cx.notify();
                        }),
                    )),
            )
            // Engine Backend info badge
            .child(
                card(&theme).child(control_row(
                    &String::from("Recording Engine"),
                    Some(&String::from("Hardware-accelerated GPU backend (gpu-screen-recorder).")),
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(Spacing::Small.rems())
                        .px(Spacing::Large.rems())
                        .py(Spacing::Small.rems())
                        .rounded_full()
                        .bg(theme.accent().opacity(0.12))
                        .border_1()
                        .border_color(theme.accent().opacity(0.3))
                        .child(
                            div()
                                .size(Spacing::Small.rems())
                                .rounded_full()
                                .bg(theme.accent()),
                        )
                        .child(
                            div()
                                .text_size(TextSize::XSmall.pixels(theme.font_size))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(theme.accent())
                                .child("gpu-screen-recorder (GPU)"),
                        ),
                    &theme,
                )),
            )
            .into_any_element()
    }
}
