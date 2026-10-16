use gpui::{AnyElement, Context, ElementId, IntoElement, ParentElement, Styled, Window, div, px};
use services::AppState;
use ui::theme::Theme;

use super::SettingsView;
use super::common::{
    card, control_row, heading, hero_header, pill_button, row_divider, slider_row, toggle_row,
};
use crate::settings::Settings;

pub struct LockScreenView;

impl LockScreenView {
    pub fn new() -> Self {
        Self
    }

    fn update_timeout(fraction: f32, cx: &mut Context<Settings>) {
        let minutes = (fraction * 60.0).round().clamp(0.0, 60.0);
        let seconds = (minutes * 60.0) as u64;
        if cx.has_global::<AppState>() {
            let _ = cx.global::<AppState>().config.update(|cfg| {
                cfg.lockscreen.idle_timeout = seconds;
            });
        }
        cx.notify();
    }
}

impl SettingsView for LockScreenView {
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

        let idle_secs = config.lockscreen.idle_timeout;
        let idle_mins = (idle_secs / 60) as f32;
        let display_timeout = if idle_secs == 0 {
            String::from("Disabled")
        } else {
            format!("{idle_mins:.0} min")
        };
        let timeout_pct = (idle_mins / 60.0).clamp(0.0, 1.0);

        let time_presets = [("%H:%M", "24h (14:30)"), ("%I:%M %p", "12h (02:30 PM)")];

        let date_presets = [
            ("%A, %B %e, %Y", "%A, %B %e, %Y"),
            ("%d/%m/%Y", "DD/MM/YYYY"),
            ("%Y-%m-%d", "YYYY-MM-DD"),
        ];

        div()
            .flex()
            .flex_col()
            .gap(px(16.0))
            .w_full()
            .min_w_0()
            // Hero Header
            .child(hero_header(
                "lock.svg",
                &String::from("Lock Screen"),
                &String::from(
                    "Configure auto-suspend, clock format, and authentication screen controls.",
                ),
                &theme,
            ))
            // Behavior Card
            .child(
                card(&theme)
                    .child(heading(&String::from("General"), &theme))
                    .child(slider_row(
                        ElementId::Name("slider-idle-timeout".into()),
                        &String::from("Idle Timeout"),
                        &display_timeout,
                        timeout_pct,
                        &theme,
                        {
                            let listener = cx.listener(|_, fraction: &f32, _, cx| {
                                Self::update_timeout(*fraction, cx)
                            });
                            move |fraction, window, cx| listener(&fraction, window, cx)
                        },
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("lock-show-clock-toggle".into()),
                        &String::from("Show Clock"),
                        Some(&String::from("Shows time and date in screen center.")),
                        config.lockscreen.show_clock,
                        &theme,
                        cx.listener(|_this, _, _, cx| {
                            if cx.has_global::<AppState>() {
                                let _ = cx.global::<AppState>().config.update(|cfg| {
                                    cfg.lockscreen.show_clock = !cfg.lockscreen.show_clock;
                                });
                            }
                            cx.notify();
                        }),
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("lock-show-media-toggle".into()),
                        &String::from("Media Player"),
                        Some(&String::from(
                            "Shows MPRIS controls and lyrics on lockscreen.",
                        )),
                        config.lockscreen.show_media_player,
                        &theme,
                        cx.listener(|_this, _, _, cx| {
                            if cx.has_global::<AppState>() {
                                let _ = cx.global::<AppState>().config.update(|cfg| {
                                    cfg.lockscreen.show_media_player =
                                        !cfg.lockscreen.show_media_player;
                                });
                            }
                            cx.notify();
                        }),
                    )),
            )
            // Time & Date Formats Card
            .child(
                card(&theme)
                    .child(heading(&String::from("Time & Date Formats"), &theme))
                    .child(control_row(
                        &String::from("Time Format"),
                        Some(&String::from(
                            "Chrono syntax (e.g. %H:%M for 24h, %I:%M %p for 12h).",
                        )),
                        div().flex().gap(px(6.0)).children(
                            time_presets
                                .into_iter()
                                .enumerate()
                                .map(|(idx, (fmt, label))| {
                                    let is_selected = config.lockscreen.time_format == fmt;
                                    let fmt_str = fmt.to_string();
                                    pill_button(
                                        ElementId::NamedInteger("time-fmt-pill".into(), idx as u64),
                                        label,
                                        is_selected,
                                        &theme,
                                        cx.listener(move |_this, _, _, cx| {
                                            if cx.has_global::<AppState>() {
                                                let target = fmt_str.clone();
                                                let _ =
                                                    cx.global::<AppState>().config.update(|cfg| {
                                                        cfg.lockscreen.time_format = target;
                                                    });
                                            }
                                            cx.notify();
                                        }),
                                    )
                                }),
                        ),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(control_row(
                        &String::from("Date Format"),
                        Some(&String::from(
                            "Chrono syntax for date below the main clock.",
                        )),
                        div().flex().gap(px(6.0)).children(
                            date_presets
                                .into_iter()
                                .enumerate()
                                .map(|(idx, (fmt, label))| {
                                    let is_selected = config.lockscreen.date_format == fmt;
                                    let fmt_str = fmt.to_string();
                                    pill_button(
                                        ElementId::NamedInteger("date-fmt-pill".into(), idx as u64),
                                        label,
                                        is_selected,
                                        &theme,
                                        cx.listener(move |_this, _, _, cx| {
                                            if cx.has_global::<AppState>() {
                                                let target = fmt_str.clone();
                                                let _ =
                                                    cx.global::<AppState>().config.update(|cfg| {
                                                        cfg.lockscreen.date_format = target;
                                                    });
                                            }
                                            cx.notify();
                                        }),
                                    )
                                }),
                        ),
                        &theme,
                    )),
            )
            .into_any_element()
    }
}
