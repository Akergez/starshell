use std::time::Duration;

use gpui::{
    AnyElement, Context, IntoElement, ParentElement, Render, Styled, Task, Window, div, relative,
    svg,
};
use services::{AppState, system::UsageStatus};
use ui::components::panel;
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize, size, units};

pub struct SystemApplet {
    usage: UsageStatus,
    _poll: Task<()>,
}

impl SystemApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let system = cx.global::<AppState>().system.clone();
        let poll = cx.spawn(async move |this, cx| {
            let _ = services::spawn_tokio({
                let system = system.clone();
                async move { system.reset_usage_sample().await }
            })
            .await;
            loop {
                let usage = services::spawn_tokio({
                    let system = system.clone();
                    async move { system.sample_usage().await }
                })
                .await
                .unwrap_or_default();
                if this
                    .update(cx, |applet: &mut SystemApplet, cx| {
                        applet.usage = usage;
                        cx.notify();
                    })
                    .is_err()
                {
                    break;
                }
                cx.background_executor().timer(Duration::from_secs(1)).await;
            }
        });
        Self {
            usage: UsageStatus::default(),
            _poll: poll,
        }
    }
}

pub fn memory_label(used_kib: u64, total_kib: u64) -> String {
    let gib = |kib: u64| kib as f64 / 1024.0 / 1024.0;
    format!("{:.1} / {:.1} GiB", gib(used_kib), gib(total_kib))
}

fn meter(
    theme: &Theme,
    icon: &'static str,
    label: &'static str,
    value: String,
    fraction: Option<f32>,
) -> AnyElement {
    let small = TextSize::Small.pixels(theme.font_size);
    div()
        .flex()
        .items_center()
        .gap(Spacing::Medium.rems())
        .child(
            div()
                .size(units(size::CONTROL))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icon)
                        .size(IconSize::Medium.rems())
                        .text_color(theme.text_muted()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .flex_1()
                .min_w_0()
                .gap(Spacing::XSmall.rems())
                .child(
                    div()
                        .flex()
                        .justify_between()
                        .text_size(small)
                        .child(div().text_color(theme.text_muted()).child(label))
                        .child(value),
                )
                .child(
                    div()
                        .w_full()
                        .h(units(size::SLIDER_TRACK))
                        .rounded_full()
                        .bg(theme.border_variant())
                        .child(
                            div()
                                .h_full()
                                .rounded_full()
                                .bg(theme.accent())
                                .w(relative(fraction.unwrap_or(0.0).clamp(0.0, 1.0))),
                        ),
                ),
        )
        .into_any_element()
}

impl Render for SystemApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let usage = self.usage;
        let percent = |value: Option<u8>| {
            value
                .map(|value| format!("{value}%"))
                .unwrap_or_else(|| "—".to_string())
        };
        let memory = (usage.memory_total_kib > 0)
            .then(|| usage.memory_used_kib as f32 / usage.memory_total_kib as f32);
        panel(&theme)
            .w_full()
            .child(meter(
                &theme,
                "cpu.svg",
                "Processor",
                percent(usage.cpu),
                usage.cpu.map(|value| value as f32 / 100.0),
            ))
            .child(meter(
                &theme,
                "memory-stick.svg",
                "Memory",
                memory_label(usage.memory_used_kib, usage.memory_total_kib),
                memory,
            ))
            .children(usage.gpu.map(|gpu| {
                meter(
                    &theme,
                    "gpu.svg",
                    "Graphics",
                    format!("{gpu}%"),
                    Some(gpu as f32 / 100.0),
                )
            }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_is_shown_in_gibibytes() {
        assert_eq!(memory_label(1024 * 1024, 4 * 1024 * 1024), "1.0 / 4.0 GiB");
    }
}
