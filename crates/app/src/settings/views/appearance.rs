use gpui::{
    AnyElement, App, Context, ElementId, IntoElement, ParentElement, Styled, Window, div,
    prelude::*,
};
use services::{AppConfig, AppState, BarPosition};
use ui::components::SegmentedControl;
use ui::theme::Theme;
use ui::tokens::{Spacing, TextSize, size, units};

use super::SettingsView;
use super::common::{card, control_row, heading, hero_header, row_divider, slider_row, toggle_row};
use crate::settings::Settings;
use crate::settings::appearance::{AppearanceStatus, set_font_size};

const FONT_SIZES: (f32, f32) = (12.0, 22.0);
const BAR_SIZES: (f32, f32) = (22.0, 48.0);

pub struct AppearanceView;

impl AppearanceView {
    pub fn new() -> Self {
        Self
    }
}

impl Default for AppearanceView {
    fn default() -> Self {
        Self::new()
    }
}

fn fraction(value: f32, (min, max): (f32, f32)) -> f32 {
    ((value - min) / (max - min)).clamp(0.0, 1.0)
}

fn value(fraction: f32, (min, max): (f32, f32)) -> f32 {
    (min + fraction * (max - min)).round()
}

fn update_config(cx: &mut App, change: impl FnOnce(&mut AppConfig)) {
    if let Err(error) = cx.global::<AppState>().config.update(change) {
        eprintln!("Failed to save settings: {error}");
    }
}

fn toggle(
    cx: &mut Context<Settings>,
    change: fn(&mut AppConfig),
) -> impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |_, _, _, cx| {
        update_config(cx, change);
        cx.notify();
    })
}

impl SettingsView for AppearanceView {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Settings>) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let config = cx.global::<AppState>().config.get();
        let error = cx.global::<AppearanceStatus>().error.clone();
        let font_size = theme.font_size;

        let font_listener = cx.listener(|_, fraction: &f32, _, cx| {
            set_font_size(value(*fraction, FONT_SIZES), cx);
            cx.notify();
        });
        let bar_listener = cx.listener(|_, fraction: &f32, _, cx| {
            let size = value(*fraction, BAR_SIZES);
            update_config(cx, |config| config.bar.size = size);
            cx.notify();
        });
        let position = SegmentedControl::new("setting-bar-position")
            .option("Top")
            .option("Bottom")
            .selected(Some(match config.bar.position {
                BarPosition::Top => 0,
                BarPosition::Bottom => 1,
            }))
            .on_select(|index, _, cx| {
                let position = if index == 0 {
                    BarPosition::Top
                } else {
                    BarPosition::Bottom
                };
                update_config(cx, |config| config.bar.position = position);
            });

        div()
            .flex()
            .flex_col()
            .gap(Spacing::XLarge.rems())
            .w_full()
            .min_w_0()
            .child(hero_header(
                "palette.svg",
                "Appearance",
                "Colors, type size, motion and the bar.",
                &theme,
            ))
            .child(
                card(&theme)
                    .child(heading("Colors", &theme))
                    .child(toggle_row(
                        ElementId::Name("setting-dynamic-colors".into()),
                        "Dynamic colors",
                        Some("Derive the accent from the wallpaper"),
                        config.ui.dynamic_colors,
                        &theme,
                        toggle(cx, |config| {
                            config.ui.dynamic_colors = !config.ui.dynamic_colors
                        }),
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("setting-dark-mode".into()),
                        "Dark mode",
                        Some("Independent of the selected theme"),
                        config.ui.dark_mode,
                        &theme,
                        toggle(cx, |config| config.ui.dark_mode = !config.ui.dark_mode),
                    ))
                    .when_some(error, |card, error| {
                        card.child(
                            div()
                                .px(Spacing::Large.rems())
                                .pb(Spacing::Medium.rems())
                                .text_size(TextSize::Small.pixels(theme.font_size))
                                .text_color(theme.warning())
                                .child(error),
                        )
                    }),
            )
            .child(
                card(&theme)
                    .child(heading("Interface", &theme))
                    .child(slider_row(
                        ElementId::Name("setting-font-size".into()),
                        "Font size",
                        &format!("{font_size:.0} px"),
                        fraction(font_size, FONT_SIZES),
                        &theme,
                        move |fraction, window, cx| font_listener(&fraction, window, cx),
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("setting-reduce-motion".into()),
                        "Reduce motion",
                        Some("Show every change instantly"),
                        config.ui.reduce_motion,
                        &theme,
                        toggle(cx, |config| {
                            config.ui.reduce_motion = !config.ui.reduce_motion
                        }),
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("setting-clock-24h".into()),
                        "24-hour clock",
                        None,
                        config.ui.clock_24_hour,
                        &theme,
                        toggle(cx, |config| {
                            config.ui.clock_24_hour = !config.ui.clock_24_hour
                        }),
                    )),
            )
            .child(
                card(&theme)
                    .child(heading("Bar", &theme))
                    .child(control_row(
                        "Position",
                        None,
                        div().w(units(size::PANEL_MIN_WIDTH)).child(position),
                        &theme,
                    ))
                    .child(row_divider(&theme))
                    .child(slider_row(
                        ElementId::Name("setting-bar-size".into()),
                        "Height",
                        &format!("{:.0} units", config.bar.clamped_size()),
                        fraction(config.bar.clamped_size(), BAR_SIZES),
                        &theme,
                        move |fraction, window, cx| bar_listener(&fraction, window, cx),
                    ))
                    .child(row_divider(&theme))
                    .child(toggle_row(
                        ElementId::Name("setting-bar-auto-hide".into()),
                        "Auto-hide",
                        Some("Leave a thin strip that reveals the bar on hover"),
                        config.bar.auto_hide,
                        &theme,
                        toggle(cx, |config| config.bar.auto_hide = !config.bar.auto_hide),
                    )),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slider_fraction_round_trips_whole_values() {
        assert_eq!(fraction(15.0, FONT_SIZES), 0.3);
        assert_eq!(value(0.3, FONT_SIZES), 15.0);
        assert_eq!(value(fraction(28.0, BAR_SIZES), BAR_SIZES), 28.0);
        assert_eq!(fraction(100.0, BAR_SIZES), 1.0);
    }
}
