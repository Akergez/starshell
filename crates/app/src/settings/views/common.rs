use gpui::{
    AnyElement, App, Bounds, Div, ElementId, FontWeight, IntoElement, ParentElement, Pixels,
    Styled, Window, div, prelude::*, svg,
};
use ui::components::{Button, ButtonSize, ButtonVariant, Slider, Switch};
use ui::theme::Theme;
use ui::tokens::{IconSize, Radius, Spacing, TextSize, size, units};

pub fn hero_header(icon: &'static str, title: &str, subtitle: &str, theme: &Theme) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(Spacing::Large.rems())
        .w_full()
        .min_w_0()
        .py(Spacing::XSmall.rems())
        .child(
            div()
                .size(units(size::CONTROL + Spacing::Small.value()))
                .flex_shrink_0()
                .rounded(Radius::Large.rems())
                .bg(theme.elevated())
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path(icon)
                        .size(IconSize::Large.rems())
                        .text_color(theme.accent()),
                ),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .gap(Spacing::XXSmall.rems())
                .child(
                    div()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_size(TextSize::XLarge.pixels(theme.font_size))
                        .text_color(theme.text())
                        .child(title.to_owned()),
                )
                .child(
                    div()
                        .text_size(TextSize::Small.pixels(theme.font_size))
                        .text_color(theme.text_muted())
                        .child(subtitle.to_owned()),
                ),
        )
        .into_any_element()
}

pub fn card(theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .overflow_hidden()
        .rounded(Radius::Large.rems())
        .bg(theme.elevated())
}

pub fn heading(title: &str, theme: &Theme) -> Div {
    div()
        .px(Spacing::Large.rems())
        .pt(Spacing::Large.rems())
        .pb(Spacing::XSmall.rems())
        .text_size(TextSize::Small.pixels(theme.font_size))
        .font_weight(FontWeight::MEDIUM)
        .text_color(theme.text_muted())
        .child(title.to_owned())
}

pub fn row_divider(theme: &Theme) -> AnyElement {
    div()
        .w_full()
        .h(Spacing::Hairline.rems())
        .flex_shrink_0()
        .bg(theme.border_variant())
        .into_any_element()
}

fn row_text(title: &str, subtitle: Option<&str>, theme: &Theme) -> Div {
    div()
        .flex()
        .flex_col()
        .flex_1()
        .min_w_0()
        .pr(Spacing::Large.rems())
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_color(theme.text())
                .child(title.to_string()),
        )
        .when_some(subtitle, |column, subtitle| {
            column.child(
                div()
                    .text_size(TextSize::Small.pixels(theme.font_size))
                    .text_color(theme.text_muted())
                    .child(subtitle.to_string()),
            )
        })
}

fn row() -> Div {
    div()
        .flex()
        .items_center()
        .justify_between()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .px(Spacing::Large.rems())
        .py(Spacing::Medium.rems())
}

pub fn control_row(
    title: &str,
    subtitle: Option<&str>,
    control: impl IntoElement,
    theme: &Theme,
) -> AnyElement {
    row()
        .child(row_text(title, subtitle, theme))
        .child(div().flex().items_center().flex_shrink_0().child(control))
        .into_any_element()
}

pub fn toggle_row(
    id: ElementId,
    title: &str,
    subtitle: Option<&str>,
    is_on: bool,
    theme: &Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    row()
        .child(row_text(title, subtitle, theme))
        .child(Switch::new(id).checked(is_on).on_click(on_click))
        .into_any_element()
}

pub fn slider_row(
    id: ElementId,
    title: &str,
    display_val: &str,
    pct: f32,
    theme: &Theme,
    on_change: impl Fn(f32, &mut Window, &mut App) + 'static,
) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .w_full()
        .min_w_0()
        .flex_shrink_0()
        .px(Spacing::Large.rems())
        .py(Spacing::Medium.rems())
        .gap(Spacing::Small.rems())
        .child(
            div()
                .flex()
                .items_center()
                .justify_between()
                .w_full()
                .min_w_0()
                .child(div().min_w_0().truncate().child(title.to_string()))
                .child(
                    div()
                        .flex_shrink_0()
                        .text_size(TextSize::Small.pixels(theme.font_size))
                        .text_color(theme.text_muted())
                        .child(display_val.to_string()),
                ),
        )
        .child(Slider::new(id).value(pct).on_change(on_change))
        .into_any_element()
}

pub fn pill_button(
    id: ElementId,
    label: &str,
    is_selected: bool,
    _theme: &Theme,
    on_click: impl Fn(&gpui::ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    Button::new(id)
        .label(label.to_string())
        .selected(is_selected)
        .variant(ButtonVariant::Outline)
        .size(ButtonSize::Sm)
        .on_click(on_click)
        .into_any_element()
}

pub fn compute_dropdown_placement(
    scroll_bounds: Bounds<Pixels>,
    trigger_bounds: (f32, f32),
    fallback_local: (f32, f32),
    item_count: usize,
) -> (bool, f32) {
    let scroll_top: f32 = scroll_bounds.top().into();
    let scroll_bottom: f32 = scroll_bounds.bottom().into();

    let (viewport_top, viewport_bottom) = if scroll_bottom > scroll_top {
        (scroll_top, scroll_bottom)
    } else {
        (0.0, 560.0)
    };

    let (trigger_top, trigger_bottom) = if trigger_bounds.1 > trigger_bounds.0 {
        (trigger_bounds.0, trigger_bounds.1)
    } else {
        (
            viewport_top + fallback_local.0,
            viewport_top + fallback_local.1,
        )
    };

    let margin = 12.0;
    let space_above = (trigger_top - viewport_top - margin).max(0.0);
    let space_below = (viewport_bottom - trigger_bottom - margin).max(0.0);

    let desired_height = (item_count as f32 * 32.0 + 14.0).clamp(40.0, 220.0);

    let (open_upwards, available_space) = if space_below >= desired_height {
        (false, space_below)
    } else if space_above >= desired_height || space_above > space_below {
        (true, space_above)
    } else {
        (false, space_below)
    };

    let max_height = available_space.clamp(48.0, 220.0);
    (open_upwards, max_height)
}
