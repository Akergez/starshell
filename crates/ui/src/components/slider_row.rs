use crate::components::{IconButton, Slider};
use crate::theme::Theme;
use crate::tokens::{IconSize, Spacing, TextSize, size, units};
use gpui::{
    AnyElement, App, ElementId, IntoElement, ParentElement, RenderOnce, SharedString, Styled,
    Window, div, prelude::FluentBuilder, svg,
};

pub fn percentage(value: f32) -> String {
    format!("{}%", (value.clamp(0.0, 1.0) * 100.0).round() as u32)
}

#[derive(IntoElement)]
pub struct SliderRow {
    leading: AnyElement,
    label: SharedString,
    slider: Slider,
    value: f32,
}

impl SliderRow {
    pub fn new(
        leading: impl IntoElement,
        label: impl Into<SharedString>,
        slider: Slider,
        value: f32,
    ) -> Self {
        Self {
            leading: leading.into_any_element(),
            label: label.into(),
            slider: slider.value(value),
            value,
        }
    }

    pub fn icon(theme: &Theme, icon: impl Into<SharedString>) -> impl IntoElement {
        div()
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(units(size::CONTROL))
            .child(
                svg()
                    .path(icon.into())
                    .size(IconSize::Medium.rems())
                    .text_color(theme.text_muted()),
            )
    }

    pub fn mute_button(
        id: impl Into<ElementId>,
        theme: &Theme,
        icon: impl Into<SharedString>,
        muted: bool,
    ) -> IconButton {
        IconButton::new(id, icon).when(muted, |button| button.icon_color(theme.error()))
    }
}

impl RenderOnce for SliderRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let small_text = TextSize::Small.pixels(theme.font_size);
        div()
            .flex()
            .items_center()
            .w_full()
            .min_w_0()
            .gap(Spacing::Medium.rems())
            .child(self.leading)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .gap(Spacing::XXSmall.rems())
                    .child(
                        div()
                            .truncate()
                            .text_size(small_text)
                            .text_color(theme.text_muted())
                            .child(self.label),
                    )
                    .child(self.slider),
            )
            .child(
                div()
                    .flex_shrink_0()
                    .w(units(size::PERCENTAGE_LABEL))
                    .flex()
                    .justify_end()
                    .text_size(small_text)
                    .text_color(theme.text_muted())
                    .child(percentage(self.value)),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_rounds_and_clamps() {
        assert_eq!(percentage(0.794), "79%");
        assert_eq!(percentage(1.2), "100%");
        assert_eq!(percentage(-1.0), "0%");
    }
}
