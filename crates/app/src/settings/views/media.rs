use super::SettingsView;
use super::common::{hero_header, toggle_row};
use crate::settings::Settings;
use crate::settings::style;
use gpui::{
    AnyElement, App, ColorExt, Context, ElementId, FontWeight, IntoElement, ParentElement, Styled,
    Window, div, prelude::*, px, svg,
};
use services::AppState;
use ui::components::{IconButton, Slider, SliderRow};
use ui::theme::Theme;

pub struct MediaView {
    show_output_devices: bool,
    show_input_devices: bool,
    new_player_input: String,
    is_adding_player: bool,
}

impl MediaView {
    pub fn new() -> Self {
        Self {
            show_output_devices: false,
            show_input_devices: false,
            new_player_input: String::new(),
            is_adding_player: false,
        }
    }

    fn volume_listener(
        input: bool,
        cx: &mut Context<Settings>,
    ) -> impl Fn(f32, &mut Window, &mut App) + 'static {
        let listener = cx.listener(move |_, fraction: &f32, _, cx| {
            let value = (fraction * 100.0).round() as u32;
            if cx.has_global::<AppState>() {
                let system = &cx.global::<AppState>().system;
                if input {
                    system.set_input_volume_fast(value);
                } else {
                    system.set_volume_fast(value);
                }
            }
            cx.notify();
        });
        move |fraction, window, cx| listener(&fraction, window, cx)
    }
}

impl SettingsView for MediaView {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn handle_key_down(
        &mut self,
        event: &gpui::KeyDownEvent,
        _window: &mut Window,
        cx: &mut Context<Settings>,
    ) -> bool {
        if !self.is_adding_player {
            return false;
        }
        let key = event.keystroke.key.as_str();
        match key {
            "enter" => {
                let clean = self.new_player_input.trim().to_string();
                if !clean.is_empty() && cx.has_global::<AppState>() {
                    let state = cx.global::<AppState>();
                    let _ = state.config.update(|cfg| {
                        if !cfg
                            .mpris
                            .players
                            .iter()
                            .any(|p| p.eq_ignore_ascii_case(&clean))
                        {
                            cfg.mpris.players.push(clean);
                        }
                    });
                    self.new_player_input.clear();
                    self.is_adding_player = false;
                    cx.notify();
                }
                true
            }
            "escape" => {
                self.is_adding_player = false;
                self.new_player_input.clear();
                cx.notify();
                true
            }
            "backspace" => {
                self.new_player_input.pop();
                cx.notify();
                true
            }
            _ => {
                if let Some(c) = event.keystroke.key_char.as_deref() {
                    if c.chars().count() == 1 {
                        self.new_player_input.push_str(c);
                        cx.notify();
                        return true;
                    }
                }
                false
            }
        }
    }

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Settings>) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let app_state = if cx.has_global::<AppState>() {
            Some(cx.global::<AppState>().clone())
        } else {
            None
        };

        let status = app_state
            .as_ref()
            .map(|s| s.system.get_status())
            .unwrap_or_default();
        let config = app_state
            .as_ref()
            .map(|s| s.config.get())
            .unwrap_or_default();

        let out_vol = status.volume.min(100);
        let out_fraction = out_vol as f32 / 100.0;
        let out_muted = status.is_muted;

        let in_vol = status.input_volume.min(100);
        let in_fraction = in_vol as f32 / 100.0;
        let in_muted = status.is_input_muted;

        let active_color = if out_muted {
            theme.text_muted()
        } else {
            theme.accent()
        };
        let in_active_color = if in_muted {
            theme.text_muted()
        } else {
            theme.accent()
        };

        // 1. Output Volume Drag Bar (Capsule Dashboard Style)
        let output_drag_bar = SliderRow::new(
            SliderRow::mute_button(
                "settings-output-mute",
                &theme,
                if out_muted {
                    "volume-x.svg"
                } else {
                    "volume-2.svg"
                },
                out_muted,
            )
            .on_click(|_, _, cx| {
                let system = cx.global::<AppState>().system.clone();
                services::spawn_tokio(async move {
                    let _ = system.toggle_mute().await;
                });
            }),
            "Output",
            Slider::new("settings-output-slider")
                .color(active_color)
                .on_change(Self::volume_listener(false, cx)),
            out_fraction,
        );

        let input_drag_bar = SliderRow::new(
            SliderRow::mute_button(
                "settings-input-mute",
                &theme,
                if in_muted { "mic-off.svg" } else { "mic.svg" },
                in_muted,
            )
            .on_click(|_, _, cx| {
                let system = cx.global::<AppState>().system.clone();
                services::spawn_tokio(async move {
                    let _ = system.toggle_input_mute().await;
                });
            }),
            "Input",
            Slider::new("settings-input-slider")
                .color(in_active_color)
                .on_change(Self::volume_listener(true, cx)),
            in_fraction,
        );

        // Output devices list (Capsule native style)
        let show_out_devices = self.show_output_devices;
        let mut out_devices_list = div().flex().flex_col().gap(px(6.0)).w_full();
        if status.audio_sinks.is_empty() {
            out_devices_list = out_devices_list.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_muted())
                    .p(px(8.0))
                    .child(String::from("No audio output devices found")),
            );
        } else {
            for sink in &status.audio_sinks {
                let is_selected = sink.is_default;
                let sink_name = sink.name.clone();
                let desc_lower = sink.description.to_lowercase();
                let icon = if desc_lower.contains("headphone")
                    || desc_lower.contains("auricular")
                    || desc_lower.contains("headset")
                {
                    "headphones.svg"
                } else if desc_lower.contains("hdmi")
                    || desc_lower.contains("displayport")
                    || desc_lower.contains("monitor")
                    || desc_lower.contains("tv")
                {
                    "monitor.svg"
                } else {
                    "volume-2.svg"
                };

                out_devices_list = out_devices_list.child(
                    div()
                        .id(ElementId::Name(format!("sink-{}", sink.name).into()))
                        .w_full()
                        .p(px(10.0))
                        .rounded(ui::tokens::Radius::Medium.rems())
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .bg(if is_selected {
                            style::accent_card(&theme)
                        } else {
                            style::surface(&theme)
                        })
                        .border_1()
                        .border_color(if is_selected {
                            theme.accent().opacity(0.4)
                        } else {
                            style::border(&theme)
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(style::hover(&theme)))
                        .on_click(cx.listener(move |_this, _, _, cx| {
                            let target = sink_name.clone();
                            if cx.has_global::<AppState>() {
                                let sys = cx.global::<AppState>().system.clone();
                                services::spawn_tokio(async move {
                                    let _ = sys.set_default_sink(&target).await;
                                });
                            }
                        }))
                        .child(
                            div()
                                .size(px(28.0))
                                .rounded_full()
                                .bg(style::background(&theme))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(svg().path(icon).size(px(15.0)).text_color(
                                    if is_selected {
                                        theme.accent()
                                    } else {
                                        theme.text_muted()
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_ellipsis()
                                        .child(sink.description.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.text_muted())
                                        .text_ellipsis()
                                        .child(sink.name.clone()),
                                ),
                        ),
                );
            }
        }

        // Input devices list
        let show_in_devices = self.show_input_devices;
        let mut in_devices_list = div().flex().flex_col().gap(px(6.0)).w_full();
        if status.audio_sources.is_empty() {
            in_devices_list = in_devices_list.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_muted())
                    .p(px(8.0))
                    .child(String::from("No audio input devices found")),
            );
        } else {
            for source in &status.audio_sources {
                let is_selected = source.is_default;
                let source_name = source.name.clone();
                in_devices_list = in_devices_list.child(
                    div()
                        .id(ElementId::Name(format!("source-{}", source.name).into()))
                        .w_full()
                        .p(px(10.0))
                        .rounded(ui::tokens::Radius::Medium.rems())
                        .flex()
                        .items_center()
                        .gap(px(12.0))
                        .bg(if is_selected {
                            style::accent_card(&theme)
                        } else {
                            style::surface(&theme)
                        })
                        .border_1()
                        .border_color(if is_selected {
                            theme.accent().opacity(0.4)
                        } else {
                            style::border(&theme)
                        })
                        .cursor_pointer()
                        .hover(|s| s.bg(style::hover(&theme)))
                        .on_click(cx.listener(move |_this, _, _, cx| {
                            let target = source_name.clone();
                            if cx.has_global::<AppState>() {
                                let sys = cx.global::<AppState>().system.clone();
                                services::spawn_tokio(async move {
                                    let _ = sys.set_default_source(&target).await;
                                });
                            }
                        }))
                        .child(
                            div()
                                .size(px(28.0))
                                .rounded_full()
                                .bg(style::background(&theme))
                                .flex()
                                .items_center()
                                .justify_center()
                                .child(svg().path("mic.svg").size(px(15.0)).text_color(
                                    if is_selected {
                                        theme.accent()
                                    } else {
                                        theme.text_muted()
                                    },
                                )),
                        )
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .flex()
                                .flex_col()
                                .gap(px(2.0))
                                .child(
                                    div()
                                        .text_size(px(13.0))
                                        .font_weight(FontWeight::MEDIUM)
                                        .text_ellipsis()
                                        .child(source.description.clone()),
                                )
                                .child(
                                    div()
                                        .text_size(px(11.0))
                                        .text_color(theme.text_muted())
                                        .text_ellipsis()
                                        .child(source.name.clone()),
                                ),
                        ),
                );
            }
        }

        // Music players list
        let mut players_list = div().flex().flex_col().gap(px(6.0)).w_full();
        if config.mpris.players.is_empty() {
            players_list = players_list.child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.text_muted())
                    .p(px(8.0))
                    .child(String::from("No music players configured.")),
            );
        } else {
            for (idx, player) in config.mpris.players.iter().enumerate() {
                let p_name = player.clone();
                players_list = players_list.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .w_full()
                        .p(px(10.0))
                        .rounded(ui::tokens::Radius::Medium.rems())
                        .bg(style::surface(&theme))
                        .child(
                            div()
                                .text_size(px(13.0))
                                .font_weight(FontWeight::MEDIUM)
                                .child(p_name),
                        )
                        .child(
                            IconButton::new(format!("remove-player-{}", idx), "close.svg")
                                .size(22.0)
                                .icon_size(10.0)
                                .on_click(cx.listener(move |_this, _, _, cx| {
                                    if cx.has_global::<AppState>() {
                                        let _ = cx.global::<AppState>().config.update(|cfg| {
                                            if idx < cfg.mpris.players.len() {
                                                cfg.mpris.players.remove(idx);
                                            }
                                        });
                                    }
                                })),
                        ),
                );
            }
        }

        // Render whole Media View
        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            // Header
            .child(hero_header(
                "music.svg",
                &String::from("Media"),
                &String::from("Media player control and authorized MPRIS services."),
                &theme,
            ))
            // 1. Output Audio Card
            .child(
                style::card(&theme)
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .p(px(16.0))
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(String::from("Audio Output")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text_muted())
                                            .child(String::from("Volume control and playback device selection.")),
                                    ),
                            )
                            .child(
                                div()
                                    .id("toggle-choose-output-device")
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .rounded(px(6.0))
                                    .hover(|s| s.bg(style::hover(&theme)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(media) = this.view_as_mut::<MediaView>() {
                                            media.show_output_devices = !media.show_output_devices;
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.accent())
                                            .child(if show_out_devices {
                                                String::from("Hide devices")
                                            } else {
                                                String::from("Choose device")
                                            }),
                                    )
                                    .child(
                                        svg()
                                            .path(if show_out_devices { "chevron-down.svg" } else { "chevron-right.svg" })
                                            .size(px(14.0))
                                            .text_color(theme.accent()),
                                    ),
                            ),
                    )
                    .child(output_drag_bar)
                    .when(show_out_devices, |s| s.child(out_devices_list)),
            )
            // 2. Input Audio Card
            .child(
                style::card(&theme)
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .p(px(16.0))
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(String::from("Audio Input (Microphone)")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text_muted())
                                            .child(String::from("Volume control and capture device selection.")),
                                    ),
                            )
                            .child(
                                div()
                                    .id("toggle-choose-input-device")
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .rounded(px(6.0))
                                    .hover(|s| s.bg(style::hover(&theme)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(media) = this.view_as_mut::<MediaView>() {
                                            media.show_input_devices = !media.show_input_devices;
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.accent())
                                            .child(if show_in_devices {
                                                String::from("Hide devices")
                                            } else {
                                                String::from("Choose device")
                                            }),
                                    )
                                    .child(
                                        svg()
                                            .path(if show_in_devices { "chevron-down.svg" } else { "chevron-right.svg" })
                                            .size(px(14.0))
                                            .text_color(theme.accent()),
                                    ),
                            ),
                    )
                    .child(input_drag_bar)
                    .when(show_in_devices, |s| s.child(in_devices_list)),
            )
            // 3. Media Player & MPRIS Options Card
            .child(
                style::card(&theme)
                    .flex()
                    .flex_col()
                    .gap(px(14.0))
                    .p(px(16.0))
                    .w_full()
                    .child(toggle_row(
                        ElementId::Name("toggle-media-lockscreen".into()),
                        &String::from("Media Player"),
                        Some(&String::from("Shows MPRIS controls and lyrics on lockscreen.")),
                        config.lockscreen.show_media_player,
                        &theme,
                        cx.listener(|_this, _, _, cx| {
                            if cx.has_global::<AppState>() {
                                let _ = cx.global::<AppState>().config.update(|cfg| {
                                    cfg.lockscreen.show_media_player = !cfg.lockscreen.show_media_player;
                                });
                            }
                            cx.notify();
                        }),
                    ))
                    .child(div().w_full().h(px(1.0)).bg(style::border(&theme)))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_between()
                            .w_full()
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .child(
                                        div()
                                            .text_size(px(14.0))
                                            .font_weight(FontWeight::MEDIUM)
                                            .child(String::from("Music Players (MPRIS)")),
                                    )
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.text_muted())
                                            .child(String::from("MPRIS services tracked for media playback (e.g. Spotify, Fastpotify).")),
                                    ),
                            )
                            .child(
                                div()
                                    .id("btn-add-music-player")
                                    .flex()
                                    .items_center()
                                    .gap(px(6.0))
                                    .cursor_pointer()
                                    .p(px(6.0))
                                    .rounded(px(6.0))
                                    .hover(|s| s.bg(style::hover(&theme)))
                                    .on_click(cx.listener(|this, _, _, cx| {
                                        if let Some(media) = this.view_as_mut::<MediaView>() {
                                            media.is_adding_player = !media.is_adding_player;
                                            cx.notify();
                                        }
                                    }))
                                    .child(
                                        div()
                                            .text_size(px(12.0))
                                            .text_color(theme.accent())
                                            .child(String::from("Add")),
                                    )
                                    .child(svg().path("plus.svg").size(px(14.0)).text_color(theme.accent())),
                            ),
                    )
                    .when(self.is_adding_player, |s| {
                        let placeholder = String::from("App name...");
                        let text_val = if self.new_player_input.is_empty() {
                            &placeholder
                        } else {
                            &self.new_player_input
                        };
                        s.child(
                            div()
                                .w_full()
                                .p(px(10.0))
                                .rounded(ui::tokens::Radius::Medium.rems())
                                .bg(style::surface(&theme))
                                .border_1()
                                .border_color(theme.accent())
                                .text_size(px(13.0))
                                .text_color(if self.new_player_input.is_empty() {
                                    theme.text_muted()
                                } else {
                                    theme.text()
                                })
                                .child(text_val.to_string()),
                        )
                    })
                    .child(players_list),
            )
            .into_any_element()
    }
}
