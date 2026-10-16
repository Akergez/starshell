use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div};
use services::{AppState, PowerProfile};
use ui::components::SegmentedControl;
use ui::theme::Theme;
use ui::tokens::{Spacing, size, units};

use super::SettingsView;
use super::common::{card, control_row, hero_header};
use crate::settings::Settings;

const PROFILES: [PowerProfile; 3] = [
    PowerProfile::PowerSaver,
    PowerProfile::Balanced,
    PowerProfile::Performance,
];

pub struct SystemView;

impl SystemView {
    pub fn new() -> Self {
        Self
    }
}

impl SettingsView for SystemView {
    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn render(&mut self, _window: &mut Window, cx: &mut Context<Settings>) -> AnyElement {
        let theme = cx.global::<Theme>().clone();
        let power = &cx.global::<AppState>().power;
        let available = power.profiles_available();
        let current = power.get_active_profile();
        let selector = PROFILES
            .iter()
            .fold(
                SegmentedControl::new("system-power-plan"),
                |control, profile| control.option(profile.display_name()),
            )
            .selected(
                available
                    .then(|| PROFILES.iter().position(|profile| *profile == current))
                    .flatten(),
            )
            .disabled(!available)
            .on_select(|index, _, cx| {
                if let Some(profile) = PROFILES.get(index) {
                    cx.global::<AppState>()
                        .power
                        .set_active_profile(profile.clone());
                }
            });

        div()
            .flex()
            .flex_col()
            .gap(Spacing::XLarge.rems())
            .w_full()
            .min_w_0()
            .child(hero_header(
                "settings.svg",
                "System",
                "Processor power profile.",
                &theme,
            ))
            .child(card(&theme).child(control_row(
                "Power Plan",
                Some(if available {
                    "Adjusts processor power and consumption via power-profiles-daemon."
                } else {
                    "power-profiles-daemon is not running."
                }),
                div().w(units(size::PANEL_MIN_WIDTH)).child(selector),
                &theme,
            )))
            .into_any_element()
    }
}
