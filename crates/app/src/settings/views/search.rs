use gpui::{
    AnyElement, Context, ElementId, FontWeight, IntoElement, ParentElement, Styled, div,
    prelude::*, svg,
};
use services::AppState;
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize, size, units};

use super::common::{card, control_row, hero_header, row_divider, toggle_row};
use crate::settings::Settings;
use crate::settings::sidebar::SettingsTab;
use crate::settings::style;

fn tab_name(tab: SettingsTab) -> String {
    match tab {
        SettingsTab::Appearance => String::from("Appearance"),
        SettingsTab::Apps => String::from("Applications"),
        SettingsTab::Media => String::from("Media"),
        SettingsTab::LockScreen => String::from("Lock Screen"),
        SettingsTab::System => String::from("System"),
        SettingsTab::Record => String::from("Recording"),
    }
}

#[derive(Clone, Copy)]
pub enum SearchControl {
    ToggleDynamicColors,
    ToggleDarkMode,
    ToggleReduceMotion,
    ToggleBarAutoHide,
    ToggleClock24,
    ToggleLockscreenClock,
    ToggleLockscreenMedia,
    ToggleRecordCursor,
    Navigate(SettingsTab),
}

struct SearchDefinition {
    id: &'static str,
    title: &'static str,
    description: &'static str,
    tab: SettingsTab,
    keywords: &'static [&'static str],
    control: SearchControl,
}

const SEARCH_DEFINITIONS: &[SearchDefinition] = &[
    SearchDefinition {
        id: "dynamic-colors",
        title: "Dynamic colors",
        description: "Derive the accent from the wallpaper",
        tab: SettingsTab::Appearance,
        keywords: &[
            "wallpaper",
            "dynamic",
            "colors",
            "palette",
            "accent",
            "theme",
        ],
        control: SearchControl::ToggleDynamicColors,
    },
    SearchDefinition {
        id: "dark-mode",
        title: "Dark mode",
        description: "Independent of the selected theme",
        tab: SettingsTab::Appearance,
        keywords: &["dark", "light", "mode", "theme", "night"],
        control: SearchControl::ToggleDarkMode,
    },
    SearchDefinition {
        id: "font-size",
        title: "Font size",
        description: "Scales the whole interface",
        tab: SettingsTab::Appearance,
        keywords: &["font", "size", "scale", "text", "zoom", "units"],
        control: SearchControl::Navigate(SettingsTab::Appearance),
    },
    SearchDefinition {
        id: "reduce-motion",
        title: "Reduce motion",
        description: "Show every change instantly",
        tab: SettingsTab::Appearance,
        keywords: &["motion", "animation", "reduce", "accessibility"],
        control: SearchControl::ToggleReduceMotion,
    },
    SearchDefinition {
        id: "clock-24",
        title: "24-hour clock",
        description: "Clock format in the bar",
        tab: SettingsTab::Appearance,
        keywords: &["clock", "time", "24", "12", "format", "hour"],
        control: SearchControl::ToggleClock24,
    },
    SearchDefinition {
        id: "bar-layout",
        title: "Bar",
        description: "Position and height of the bar",
        tab: SettingsTab::Appearance,
        keywords: &[
            "bar", "panel", "position", "top", "bottom", "height", "size",
        ],
        control: SearchControl::Navigate(SettingsTab::Appearance),
    },
    SearchDefinition {
        id: "bar-auto-hide",
        title: "Auto-hide",
        description: "Leave a thin strip that reveals the bar on hover",
        tab: SettingsTab::Appearance,
        keywords: &["bar", "hide", "auto", "autohide"],
        control: SearchControl::ToggleBarAutoHide,
    },
    SearchDefinition {
        id: "apps-defaults",
        title: "Default Applications",
        description: "Tools and binaries Starshell runs for your primary tasks.",
        tab: SettingsTab::Apps,
        keywords: &[
            "apps",
            "terminal",
            "console",
            "browser",
            "web",
            "editor",
            "code",
            "files",
            "file manager",
        ],
        control: SearchControl::Navigate(SettingsTab::Apps),
    },
    SearchDefinition {
        id: "audio-output",
        title: "Audio Output",
        description: "Volume control and playback device selection.",
        tab: SettingsTab::Media,
        keywords: &[
            "audio",
            "volume",
            "output",
            "speakers",
            "headphones",
            "sound",
        ],
        control: SearchControl::Navigate(SettingsTab::Media),
    },
    SearchDefinition {
        id: "audio-input",
        title: "Audio Input (Microphone)",
        description: "Volume control and capture device selection.",
        tab: SettingsTab::Media,
        keywords: &[
            "microphone",
            "mic",
            "input",
            "audio",
            "recording",
            "voice",
            "sound",
        ],
        control: SearchControl::Navigate(SettingsTab::Media),
    },
    SearchDefinition {
        id: "music-players",
        title: "Music Players (MPRIS)",
        description: "MPRIS services tracked for media playback (e.g. Spotify).",
        tab: SettingsTab::Media,
        keywords: &["players", "music", "spotify", "mpris", "whitelist", "media"],
        control: SearchControl::Navigate(SettingsTab::Media),
    },
    SearchDefinition {
        id: "lock-clock",
        title: "Show Clock",
        description: "Shows time and date in screen center.",
        tab: SettingsTab::LockScreen,
        keywords: &["clock", "lockscreen", "time", "date"],
        control: SearchControl::ToggleLockscreenClock,
    },
    SearchDefinition {
        id: "lock-media",
        title: "Media Player",
        description: "Shows MPRIS controls and lyrics on lockscreen.",
        tab: SettingsTab::LockScreen,
        keywords: &[
            "player",
            "lockscreen",
            "mpris",
            "music",
            "spotify",
            "song",
            "media",
            "lyrics",
        ],
        control: SearchControl::ToggleLockscreenMedia,
    },
    SearchDefinition {
        id: "lock-idle-timeout",
        title: "Idle Timeout",
        description: "Lock the screen after a period of inactivity.",
        tab: SettingsTab::LockScreen,
        keywords: &[
            "idle",
            "timeout",
            "lock",
            "lockscreen",
            "inactivity",
            "time",
        ],
        control: SearchControl::Navigate(SettingsTab::LockScreen),
    },
    SearchDefinition {
        id: "lock-date-time-format",
        title: "Time Format",
        description: "Date Format",
        tab: SettingsTab::LockScreen,
        keywords: &["format", "time", "date", "clock"],
        control: SearchControl::Navigate(SettingsTab::LockScreen),
    },
    SearchDefinition {
        id: "system-power",
        title: "Power Plan",
        description: "Adjusts processor power and consumption via power-profiles-daemon.",
        tab: SettingsTab::System,
        keywords: &[
            "power",
            "battery",
            "performance",
            "profile",
            "plan",
            "balanced",
            "saver",
        ],
        control: SearchControl::Navigate(SettingsTab::System),
    },
    SearchDefinition {
        id: "record-output",
        title: "Video Output",
        description: "Monitor or screen to capture during recording.",
        tab: SettingsTab::Record,
        keywords: &["record", "screen", "monitor", "video", "output"],
        control: SearchControl::Navigate(SettingsTab::Record),
    },
    SearchDefinition {
        id: "record-quality",
        title: "Video Quality",
        description: "Resolution, frame rate and container.",
        tab: SettingsTab::Record,
        keywords: &[
            "fps",
            "resolution",
            "quality",
            "frame rate",
            "video",
            "1080p",
            "720p",
            "mkv",
            "mp4",
            "container",
        ],
        control: SearchControl::Navigate(SettingsTab::Record),
    },
    SearchDefinition {
        id: "record-cursor",
        title: "Capture Cursor",
        description: "Include mouse pointer in the recording.",
        tab: SettingsTab::Record,
        keywords: &["cursor", "mouse", "pointer", "record"],
        control: SearchControl::ToggleRecordCursor,
    },
];

pub fn render_search_results(query: &str, theme: &Theme, cx: &mut Context<Settings>) -> AnyElement {
    let query_clean = query.trim().to_lowercase();
    let query_words: Vec<&str> = query_clean.split_whitespace().collect();

    let config = if cx.has_global::<AppState>() {
        cx.global::<AppState>().config.get()
    } else {
        Default::default()
    };

    let mut matched_rows = Vec::new();

    for def in SEARCH_DEFINITIONS {
        let title = def.title.to_string();
        let desc = def.description.to_string();
        let tab_str = tab_name(def.tab);

        let matches = query_words.iter().all(|qw| {
            title.to_lowercase().contains(qw)
                || desc.to_lowercase().contains(qw)
                || tab_str.to_lowercase().contains(qw)
                || def
                    .keywords
                    .iter()
                    .any(|k| k.to_lowercase().contains(qw) || qw.contains(&k.to_lowercase()))
        });

        if !matches {
            continue;
        }

        let subtitle = format!("{} • {}", tab_str, desc);

        let row: AnyElement = match def.control {
            SearchControl::ToggleDynamicColors => {
                let is_on = config.ui.dynamic_colors;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.ui.dynamic_colors = !cfg.ui.dynamic_colors;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleDarkMode => {
                let is_on = config.ui.dark_mode;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.ui.dark_mode = !cfg.ui.dark_mode;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleReduceMotion => {
                let is_on = config.ui.reduce_motion;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        let _ = cx.global::<AppState>().config.update(|cfg| {
                            cfg.ui.reduce_motion = !cfg.ui.reduce_motion;
                        });
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleClock24 => {
                let is_on = config.ui.clock_24_hour;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.ui.clock_24_hour = !cfg.ui.clock_24_hour;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleBarAutoHide => {
                let is_on = config.bar.autohide;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        let _ = cx.global::<AppState>().config.update(|cfg| {
                            cfg.bar.autohide = !cfg.bar.autohide;
                        });
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleLockscreenClock => {
                let is_on = config.lockscreen.show_clock;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.lockscreen.show_clock = !cfg.lockscreen.show_clock;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleLockscreenMedia => {
                let is_on = config.lockscreen.show_media_player;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.lockscreen.show_media_player =
                                    !cfg.lockscreen.show_media_player;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::ToggleRecordCursor => {
                let is_on = config.record.include_cursor;
                toggle_row(
                    ElementId::Name(format!("search-toggle-{}", def.id).into()),
                    &title,
                    Some(&subtitle),
                    is_on,
                    theme,
                    cx.listener(|_this, _, _, cx| {
                        if cx.has_global::<AppState>() {
                            let _ = cx.global::<AppState>().config.update(|cfg| {
                                cfg.record.include_cursor = !cfg.record.include_cursor;
                            });
                        }
                        cx.notify();
                    }),
                )
            }
            SearchControl::Navigate(tab) => {
                let tab_label = tab_str.clone();
                let nav_btn = div()
                    .id(format!("search-nav-{}", def.id))
                    .flex()
                    .items_center()
                    .gap(Spacing::Small.rems())
                    .h(units(size::APPLICATION_ICON))
                    .px(Spacing::Large.rems())
                    .rounded_full()
                    .bg(style::surface(theme))
                    .border_1()
                    .border_color(style::border(theme))
                    .hover(|s| s.bg(style::hover(theme)).border_color(theme.accent()))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.set_tab(tab, cx);
                    }))
                    .child(
                        div()
                            .text_size(TextSize::XSmall.pixels(theme.font_size))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(theme.text())
                            .child(tab_label),
                    )
                    .child(
                        svg()
                            .path("chevron-right.svg")
                            .size(IconSize::Small.rems())
                            .text_color(theme.text_muted()),
                    );

                control_row(&title, Some(&subtitle), nav_btn, theme)
            }
        };

        matched_rows.push(row);
    }

    let display_query = if query.chars().count() > 30 {
        format!("{}…", query.chars().take(30).collect::<String>())
    } else {
        query.to_string()
    };

    if matched_rows.is_empty() {
        let no_sub = String::from("No settings found matching \"{query}\"")
            .replace("{query}", &display_query);

        div()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .w_full()
            .min_w_0()
            .py(units(size::VIEWPORT_INSET))
            .gap(Spacing::Large.rems())
            .child(
                div()
                    .size(IconSize::XXLarge.rems())
                    .rounded_full()
                    .bg(style::surface(theme))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        svg()
                            .path("search.svg")
                            .size(IconSize::XLarge.rems())
                            .text_color(theme.text_muted()),
                    ),
            )
            .child(
                div()
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_size(TextSize::Medium.pixels(theme.font_size))
                    .text_color(theme.text())
                    .child(String::from("No results")),
            )
            .child(
                div()
                    .text_size(TextSize::Small.pixels(theme.font_size))
                    .text_color(theme.text_muted())
                    .text_center()
                    .max_w(units(size::DEVICE_PANEL))
                    .overflow_hidden()
                    .text_ellipsis()
                    .child(no_sub),
            )
            .into_any_element()
    } else {
        let count_str = matched_rows.len().to_string();
        let sub_text = String::from("{count} options found for \"{query}\"")
            .replace("{count}", &count_str)
            .replace("{query}", &display_query);

        let mut card_el = card(theme);
        for (idx, row) in matched_rows.into_iter().enumerate() {
            if idx > 0 {
                card_el = card_el.child(row_divider(theme));
            }
            card_el = card_el.child(row);
        }

        div()
            .flex()
            .flex_col()
            .gap(Spacing::XLarge.rems())
            .w_full()
            .min_w_0()
            .child(hero_header(
                "search.svg",
                &String::from("Search Results"),
                &sub_text,
                theme,
            ))
            .child(card_el)
            .into_any_element()
    }
}
