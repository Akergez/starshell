use super::{HEIGHT, Settings, WIDTH, header, views};
use gpui::{AnyElement, Context, IntoElement, ParentElement, Styled, Window, div, prelude::*, px};
use ui::theme::Theme;
use ui::tokens::{Radius, Spacing, size, units_to_pixels};

pub(crate) fn render_settings(
    settings: &mut Settings,
    window: &mut Window,
    cx: &mut Context<Settings>,
) -> AnyElement {
    let theme = cx.global::<Theme>().clone();
    let viewport = window.viewport_size();
    let font_size = theme.font_size;
    let inset = units_to_pixels(size::VIEWPORT_INSET, font_size);
    let width = (viewport.width - inset).clamp(px(0.0), units_to_pixels(WIDTH, font_size));
    let height = (viewport.height - inset).clamp(px(0.0), units_to_pixels(HEIGHT, font_size));

    let active_tab = settings.active_tab;
    let progress = settings.animator.section_progress();

    let query = settings.search_input.text();
    let active_view_content = if !query.trim().is_empty() {
        views::search::render_search_results(&query, &theme, cx)
    } else if let Some((_, view)) = settings.views.iter_mut().find(|(t, _)| *t == active_tab) {
        view.render(window, cx)
    } else {
        div().into_any_element()
    };

    let header = header::render_header(&settings.search_input, &theme, cx);
    let sidebar_element =
        settings
            .sidebar
            .render(settings.active_tab, !query.is_empty(), &theme, cx);

    let card = div()
        .id("settings-modal-card")
        .occlude()
        .on_click(cx.listener(|this, _, window, cx| {
            window.focus(&this.focus_handle, cx);
        }))
        .relative()
        .top(px(settings
            .animator
            .offset_y
            .min(f32::from(viewport.height - height).max(0.0) / 2.0)))
        .w(width)
        .h(height)
        .max_w_full()
        .max_h_full()
        .flex()
        .flex_col()
        .gap(Spacing::XLarge.rems())
        .p(Spacing::XXLarge.rems())
        .flex_shrink_0()
        .rounded(Radius::XLarge.rems())
        .bg(theme.surface())
        .border_1()
        .border_color(theme.border_variant())
        .shadow_xl()
        .font_family(theme.font_family())
        .text_size(theme.font_size())
        .text_color(theme.text())
        .overflow_hidden()
        .opacity(settings.animator.card_opacity)
        .child(header)
        .child(
            div()
                .flex()
                .gap(Spacing::XXXLarge.rems())
                .flex_1()
                .min_h_0()
                .min_w_0()
                .w_full()
                .child(sidebar_element)
                .child(
                    div()
                        .id("settings-content-scroll")
                        .track_scroll(&settings.scroll_handle)
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_h_0()
                        .min_w_0()
                        .overflow_y_scroll()
                        .child(
                            div()
                                .flex()
                                .flex_col()
                                .w_full()
                                .min_w_0()
                                .flex_shrink_0()
                                .when(progress < 1.0, |s| {
                                    s.blur(Spacing::Small.pixels(font_size) * (1.0 - progress))
                                })
                                .child(active_view_content),
                        ),
                ),
        );

    div()
        .id("settings-window-root")
        .track_focus(&settings.focus_handle)
        .on_key_down(cx.listener(Settings::handle_key_down))
        .relative()
        .size_full()
        .overflow_hidden()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .id("settings-backdrop")
                .absolute()
                .inset_0()
                .on_click(cx.listener(|this, _, _, cx| {
                    this.request_close(cx);
                })),
        )
        .child(card)
        .into_any_element()
}
