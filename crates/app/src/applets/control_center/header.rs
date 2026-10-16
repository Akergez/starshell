use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, svg};
use services::AppState;
use ui::components::{Button, ButtonVariant, IconButton};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize};

use super::ControlCenter;
use crate::applets::common::run_command;
use crate::bar::battery_icon;
use crate::shell::Shell;
use crate::shell::state::Snapshot;

pub fn render(
    snapshot: &Snapshot,
    theme: &Theme,
    power_open: bool,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    let (icon, color, title, subtitle) = match snapshot.battery {
        Some(battery) => {
            let color = if battery.is_critical() {
                theme.error()
            } else if battery.is_charging {
                theme.success()
            } else if battery.is_low() {
                theme.warning()
            } else {
                theme.text()
            };
            let subtitle = battery.remaining_label().unwrap_or_else(|| {
                if battery.is_full {
                    "Fully charged".to_string()
                } else if battery.is_charging {
                    "Charging".to_string()
                } else {
                    "On battery".to_string()
                }
            });
            (
                battery_icon(&battery),
                color,
                format!("{}%", battery.percentage),
                subtitle,
            )
        }
        None => (
            "battery.svg",
            theme.text_muted(),
            "AC".to_string(),
            "No battery".to_string(),
        ),
    };

    div()
        .flex()
        .items_center()
        .w_full()
        .gap(Spacing::Small.rems())
        .child(
            svg()
                .path(icon)
                .flex_shrink_0()
                .size(IconSize::Medium.rems())
                .text_color(color),
        )
        .child(
            div()
                .flex_shrink_0()
                .font_weight(FontWeight::MEDIUM)
                .child(title),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(TextSize::Small.pixels(theme.font_size))
                .text_color(theme.text_muted())
                .child(subtitle),
        )
        .child(
            IconButton::new("power-button", "power.svg")
                .selected(power_open)
                .tooltip("Power")
                .on_click(cx.listener(|this, _, _, cx| this.toggle_power(cx))),
        )
        .into_any_element()
}

pub fn power_actions(theme: &Theme, _cx: &mut Context<ControlCenter>) -> AnyElement {
    let action = |id: &'static str,
                  icon: &'static str,
                  label: &'static str,
                  pick: fn(&services::PowerActions) -> &String| {
        div().flex_1().min_w_0().child(
            Button::new(id)
                .icon(icon)
                .label(label)
                .variant(ButtonVariant::Outline)
                .w_full(true)
                .on_click(move |_, _, cx| {
                    let config = cx.global::<AppState>().config.get();
                    let command = pick(&config.control_center.power_actions).clone();
                    Shell::close(cx);
                    run_command(&command);
                }),
        )
    };
    div()
        .flex()
        .w_full()
        .gap(Spacing::Small.rems())
        .text_color(theme.text())
        .child(action("power-sleep", "moon.svg", "Sleep", |actions| {
            &actions.sleep
        }))
        .child(action(
            "power-reboot",
            "rotate-ccw.svg",
            "Reboot",
            |actions| &actions.reboot,
        ))
        .child(action("power-off", "power.svg", "Power off", |actions| {
            &actions.poweroff
        }))
        .into_any_element()
}
