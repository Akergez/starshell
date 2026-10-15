use crate::theme::Theme;
use crate::tokens::{Radius, Spacing, TextSize, motion, units};
use gpui::{
    Animation, AnimationExt, Div, ElementId, FontWeight, IntoElement, ParentElement, SharedString,
    Styled, div, ease_out_quint,
};

pub fn panel(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::Large.rems())
        .p(Spacing::XLarge.rems())
        .bg(theme.surface())
        .border_1()
        .border_color(theme.border_variant())
        .rounded(Radius::XLarge.rems())
        .text_color(theme.text())
        .font_family(theme.font_family())
        .text_size(theme.font_size())
}

pub fn card(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .gap(Spacing::Small.rems())
        .p(Spacing::Medium.rems())
        .bg(theme.elevated())
        .rounded(Radius::Large.rems())
}

pub fn divider(theme: &Theme) -> Div {
    div()
        .flex_shrink_0()
        .w_full()
        .h(Spacing::Hairline.rems())
        .bg(theme.border_variant())
}

pub fn vertical_divider(theme: &Theme) -> Div {
    div()
        .flex_shrink_0()
        .h_full()
        .w(Spacing::Hairline.rems())
        .bg(theme.border_variant())
}

pub fn caption(theme: &Theme, text: impl Into<SharedString>) -> Div {
    div()
        .min_w_0()
        .truncate()
        .text_size(TextSize::Small.pixels(theme.font_size))
        .text_color(theme.text_muted())
        .child(text.into())
}

pub fn section_label(theme: &Theme, text: impl Into<SharedString>) -> Div {
    caption(theme, text).font_weight(FontWeight::MEDIUM)
}

pub fn reveal(id: impl Into<ElementId>, content: impl IntoElement) -> impl IntoElement {
    let offset = Spacing::Medium.value();
    div().child(content).with_animation(
        id,
        Animation::new(motion::FAST).with_easing(ease_out_quint()),
        move |element, progress| {
            element
                .opacity(progress)
                .relative()
                .top(units(-offset * (1.0 - progress)))
        },
    )
}

pub fn fade_in(id: impl Into<ElementId>, content: impl IntoElement) -> impl IntoElement {
    div().size_full().child(content).with_animation(
        id,
        Animation::new(motion::FAST),
        |element, progress| element.opacity(progress),
    )
}
