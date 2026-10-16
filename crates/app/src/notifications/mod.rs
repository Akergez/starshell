pub mod popup;

use gpui::{
    AnyElement, App, ElementId, FontWeight, IntoElement, ParentElement, Styled, div, img,
    prelude::FluentBuilder, svg,
};
use services::{NotificationItem, NotificationStore};
use ui::components::{Button, ButtonSize, ButtonVariant, IconButton, card};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize, units};

pub const REPLY_ACTION: &str = "inline-reply";

pub fn icon(item: &NotificationItem, theme: &Theme) -> AnyElement {
    let path = if item.app_icon.is_empty() {
        services::icons::resolve_icon_name(&item.app_name.to_lowercase())
    } else {
        services::icons::resolve_icon_name(item.app_icon.trim_start_matches("file://"))
    };
    match path {
        Some(path) => img(path)
            .flex_shrink_0()
            .size(IconSize::XLarge.rems())
            .into_any_element(),
        None => svg()
            .path("bell.svg")
            .flex_shrink_0()
            .size(IconSize::XLarge.rems())
            .text_color(theme.text_muted())
            .into_any_element(),
    }
}

pub fn age(item: &NotificationItem) -> String {
    let seconds = item.received_at.elapsed().as_secs();
    match seconds {
        0..60 => "now".to_string(),
        60..3600 => format!("{}m", seconds / 60),
        _ => format!("{}h", seconds / 3600),
    }
}

pub fn render_card(
    item: &NotificationItem,
    theme: &Theme,
    on_reply: impl Fn(u32, &mut App) + Clone + 'static,
) -> gpui::Div {
    let id = item.id;
    let small = TextSize::Small.pixels(theme.font_size);
    let actions: Vec<(String, String)> = item
        .actions
        .iter()
        .filter(|(key, label)| key != "default" && !label.is_empty())
        .cloned()
        .collect();
    card(theme)
        .w_full()
        .child(
            div()
                .flex()
                .items_start()
                .gap(Spacing::Medium.rems())
                .child(icon(item, theme))
                .child(
                    div()
                        .flex()
                        .flex_col()
                        .flex_1()
                        .min_w_0()
                        .gap(Spacing::XXSmall.rems())
                        .child(
                            div()
                                .flex()
                                .gap(Spacing::Small.rems())
                                .text_size(small)
                                .text_color(theme.text_muted())
                                .child(
                                    div()
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .child(item.app_name.clone()),
                                )
                                .child(age(item)),
                        )
                        .child(
                            div()
                                .font_weight(FontWeight::MEDIUM)
                                .truncate()
                                .child(item.summary.clone()),
                        )
                        .when(!item.body.is_empty(), |column| {
                            column.child(
                                div()
                                    .text_size(small)
                                    .text_color(theme.text_muted())
                                    .line_clamp(4)
                                    .child(item.body.clone()),
                            )
                        }),
                )
                .child(
                    IconButton::new(
                        ElementId::named_usize("notification-dismiss", id as usize),
                        "x.svg",
                    )
                    .tooltip("Dismiss")
                    .on_click(move |_, _, _| NotificationStore::global().remove_notification(id)),
                ),
        )
        .when(!actions.is_empty(), |card| {
            card.child(
                div()
                    .flex()
                    .flex_wrap()
                    .gap(Spacing::Small.rems())
                    .pl(units(IconSize::XLarge.value() + Spacing::Medium.value()))
                    .children(
                        actions
                            .into_iter()
                            .enumerate()
                            .map(|(index, (key, label))| {
                                let on_reply = on_reply.clone();
                                Button::new(ElementId::named_usize(
                                    format!("notification-{id}-action"),
                                    index,
                                ))
                                .label(label)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .on_click(move |_, _, cx| {
                                    if key == REPLY_ACTION {
                                        on_reply(id, cx);
                                    } else {
                                        NotificationStore::global().invoke_action(id, key.clone());
                                    }
                                })
                            }),
                    ),
            )
        })
}
