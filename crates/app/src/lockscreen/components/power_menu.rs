use gpui::{
    ColorExt, Element, InteractiveElement, ParentElement, StatefulInteractiveElement, Styled, div,
    svg,
};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, size, units};

pub fn render_power_menu(theme: &Theme) -> impl Element {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_center()
        .gap(Spacing::Large.rems())
        .child(
            div()
                .id("power-suspend")
                .flex()
                .items_center()
                .justify_center()
                .w(units(size::CONTROL))
                .h(units(size::CONTROL))
                .rounded_full()
                .bg(theme.elevated().opacity(0.45))
                .border_1()
                .border_color(theme.elevated().opacity(0.25))
                .hover(|s| s.bg(theme.elevated().opacity(0.8)))
                .active(|s| s.opacity(0.6))
                .cursor_pointer()
                .on_click(|_, _, _| {
                    let _ = std::process::Command::new("systemctl")
                        .arg("suspend")
                        .spawn();
                })
                .child(
                    svg()
                        .path("moon.svg")
                        .size(IconSize::Small.rems())
                        .text_color(theme.text_muted()),
                ),
        )
        .child(
            div()
                .id("power-reboot")
                .flex()
                .items_center()
                .justify_center()
                .w(units(size::CONTROL))
                .h(units(size::CONTROL))
                .rounded_full()
                .bg(theme.elevated().opacity(0.45))
                .border_1()
                .border_color(theme.elevated().opacity(0.25))
                .hover(|s| s.bg(theme.elevated().opacity(0.8)))
                .active(|s| s.opacity(0.6))
                .cursor_pointer()
                .on_click(|_, _, _| {
                    let _ = std::process::Command::new("systemctl")
                        .arg("reboot")
                        .spawn();
                })
                .child(
                    svg()
                        .path("rotate-ccw.svg")
                        .size(IconSize::Small.rems())
                        .text_color(theme.text_muted()),
                ),
        )
        .child(
            div()
                .id("power-shutdown")
                .flex()
                .items_center()
                .justify_center()
                .w(units(size::CONTROL))
                .h(units(size::CONTROL))
                .rounded_full()
                .bg(theme.elevated().opacity(0.45))
                .border_1()
                .border_color(theme.elevated().opacity(0.25))
                .hover(|s| {
                    s.bg(theme.error().opacity(0.25))
                        .border_color(theme.error().opacity(0.5))
                })
                .active(|s| s.opacity(0.6))
                .cursor_pointer()
                .on_click(|_, _, _| {
                    let _ = std::process::Command::new("systemctl")
                        .arg("poweroff")
                        .spawn();
                })
                .child(
                    svg()
                        .path("power.svg")
                        .size(IconSize::Small.rems())
                        .text_color(theme.text_muted()),
                ),
        )
}
