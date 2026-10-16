use gpui::{AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, svg};
use services::SystemService;
use ui::components::{Button, ButtonVariant, IconButton};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize};

use super::ControlCenter;
use crate::applets::AppletKind;
use crate::bar::battery_icon;
use crate::shell::state::Snapshot;
use crate::shell::{Shell, Surface};

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
            IconButton::new("wallpapers-button", "wallpaper.svg")
                .tooltip("Wallpapers")
                .on_click(|_, _, cx| Shell::replace(Surface::Applet(AppletKind::Wallpapers), cx)),
        )
        .child(
            IconButton::new("settings-button", "settings.svg")
                .tooltip("Settings")
                .on_click(|_, _, cx| Shell::open_settings(cx)),
        )
        .child(
            IconButton::new("power-button", "power.svg")
                .selected(power_open)
                .tooltip("Power")
                .on_click(cx.listener(|this, _, _, cx| this.toggle_power(cx))),
        )
        .into_any_element()
}

#[derive(Clone, Copy)]
enum PowerAction {
    Sleep,
    Reboot,
    PowerOff,
}

impl PowerAction {
    fn run(self) {
        services::spawn_tokio(async move {
            let result = match self {
                Self::Sleep => SystemService::suspend().await,
                Self::Reboot => SystemService::reboot().await,
                Self::PowerOff => SystemService::poweroff().await,
            };
            if let Err(error) = result {
                services::log_error!("Power", "{error}");
            }
        });
    }
}

pub fn power_actions(theme: &Theme, _cx: &mut Context<ControlCenter>) -> AnyElement {
    let action =
        |id: &'static str, icon: &'static str, label: &'static str, action: PowerAction| {
            div().flex_1().min_w_0().child(
                Button::new(id)
                    .icon(icon)
                    .label(label)
                    .variant(ButtonVariant::Outline)
                    .w_full(true)
                    .on_click(move |_, _, cx| {
                        Shell::close(cx);
                        action.run();
                    }),
            )
        };
    div()
        .flex()
        .w_full()
        .gap(Spacing::Small.rems())
        .text_color(theme.text())
        .child(action(
            "power-sleep",
            "moon.svg",
            "Sleep",
            PowerAction::Sleep,
        ))
        .child(action(
            "power-reboot",
            "rotate-ccw.svg",
            "Reboot",
            PowerAction::Reboot,
        ))
        .child(action(
            "power-off",
            "power.svg",
            "Power off",
            PowerAction::PowerOff,
        ))
        .into_any_element()
}
