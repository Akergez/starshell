use gpui::{
    ColorExt, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px, svg,
};
use ui::theme::Theme;

use crate::capsule::widgets::style;
use crate::settings::Settings;

pub fn render_header(
    search_query: &str,
    is_searching: bool,
    theme: &Theme,
    cx: &mut Context<Settings>,
) -> gpui::AnyElement {
    let placeholder = if cx.has_global::<services::AppState>() {
        cx.global::<services::AppState>()
            .language
            .get("settings.search_short")
    } else {
        "Search...".to_string()
    };

    let text_area = if is_searching {
        if search_query.is_empty() {
            div()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .gap(px(2.0))
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.accent())
                        .child("│"),
                )
                .child(
                    div()
                        .text_color(theme.foreground_muted().opacity(0.6))
                        .text_size(px(14.0))
                        .child(placeholder),
                )
        } else {
            div()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .gap(px(1.0))
                .child(
                    div()
                        .text_color(theme.foreground())
                        .text_size(px(14.0))
                        .child(search_query.to_string()),
                )
                .child(
                    div()
                        .font_weight(FontWeight::BOLD)
                        .text_color(theme.accent())
                        .child("│"),
                )
        }
    } else {
        if search_query.is_empty() {
            div()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_color(theme.foreground_muted())
                        .text_size(px(14.0))
                        .child(placeholder),
                )
        } else {
            div()
                .flex()
                .items_center()
                .flex_1()
                .min_w_0()
                .child(
                    div()
                        .text_color(theme.foreground())
                        .text_size(px(14.0))
                        .child(search_query.to_string()),
                )
        }
    };

    let search_box = div()
        .id("settings-search-box")
        .flex()
        .items_center()
        .gap(px(10.0))
        .w(px(420.0))
        .max_w_full()
        .min_w_0()
        .h(px(44.0))
        .px(px(14.0))
        .rounded_full()
        .bg(style::surface(theme))
        .border_1()
        .border_color(if is_searching {
            theme.accent()
        } else {
            style::border(theme)
        })
        .cursor_text()
        .on_click(cx.listener(|this, _, window, cx| {
            this.is_searching = true;
            window.focus(&this.focus_handle, cx);
            cx.notify();
        }))
        .child(
            svg()
                .path("search.svg")
                .size(px(15.0))
                .text_color(if is_searching {
                    theme.accent()
                } else {
                    theme.foreground_muted()
                }),
        )
        .child(text_area)
        .when(!search_query.is_empty(), |this| {
            this.child(
                div()
                    .id("settings-search-clear-btn")
                    .size(px(22.0))
                    .rounded_full()
                    .bg(style::raised(theme))
                    .hover(|s| s.bg(style::hover(theme)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .on_click(cx.listener(|this, _, window, cx| {
                        this.search_query.clear();
                        window.focus(&this.focus_handle, cx);
                        cx.notify();
                    }))
                    .child(
                        svg()
                            .path("close.svg")
                            .size(px(10.0))
                            .text_color(theme.foreground_muted()),
                    ),
            )
        })
        .when(search_query.is_empty() && !is_searching, |this| {
            this.child(
                div()
                    .px(px(6.0))
                    .py(px(2.0))
                    .rounded(px(4.0))
                    .bg(style::raised(theme))
                    .text_size(px(11.0))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(theme.foreground_muted().opacity(0.7))
                    .child("Ctrl+F"),
            )
        });

    div()
        .flex()
        .items_center()
        .gap(px(10.0))
        .w_full()
        .min_w_0()
        .h(px(48.0))
        .flex_shrink_0()
        .child(div().w(px(30.0)).flex_shrink_0())
        .child(
            div()
                .flex()
                .flex_1()
                .min_w_0()
                .justify_center()
                .child(search_box),
        )
        .child(
            style::circle_button("settings-close-btn", "close.svg", 30.0, theme)
                .flex_shrink_0()
                .on_click(cx.listener(|this, _, _, cx| this.request_close(cx))),
        )
        .into_any_element()
}
