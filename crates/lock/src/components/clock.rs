use chrono::Local;
use gpui::{ColorExt, Element, ParentElement, Styled, div};
use services::LockScreenConfig;
use ui::theme::Theme;
use ui::tokens::{Spacing, TextSize, units};

const CLOCK_SIZE: f32 = 84.0;

pub fn render_clock(theme: &Theme, config: &LockScreenConfig) -> impl Element {
    let now = Local::now();
    let time_str = now.format(&config.time_format).to_string();
    let date_str = now.format("%A, %B %-d").to_string();

    div()
        .flex()
        .flex_col()
        .items_center()
        .gap(Spacing::XSmall.rems())
        .child(
            div()
                .font_family(theme.font_family())
                .text_size(TextSize::Large.pixels(theme.font_size))
                .font_weight(gpui::FontWeight::MEDIUM)
                .text_color(theme.text().opacity(0.92))
                .child(date_str),
        )
        .child(
            div()
                .font_family(theme.font_family())
                .text_size(units(CLOCK_SIZE))
                .line_height(units(CLOCK_SIZE))
                .font_weight(gpui::FontWeight::BOLD)
                .text_color(theme.text())
                .child(time_str),
        )
}
