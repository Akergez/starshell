mod model;

use gpui::{
    Context, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, StatefulInteractiveElement, Styled, Window, div, svg,
};
use model::{COLUMNS, EmojiModel, PAGE_SIZE};
use services::EmojiService;
use ui::components::{Input, InputState, caption, panel};
use ui::theme::{Theme, mix};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};

use crate::shell::Shell;

const CATEGORIES: &[(Option<&str>, &str)] = &[
    (None, "All"),
    (Some("people"), "😊"),
    (Some("animals"), "🐾"),
    (Some("nature"), "🌿"),
    (Some("food"), "🍔"),
    (Some("activity"), "⚽"),
    (Some("travel"), "✈️"),
    (Some("objects"), "💡"),
    (Some("symbols"), "🔣"),
    (Some("flags"), "🏁"),
];

pub struct EmojiApplet {
    model: EmojiModel,
    search: InputState,
    service: EmojiService,
    focus: FocusHandle,
}

impl EmojiApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let service = EmojiService::new();
        Self {
            model: EmojiModel::new(service.load_emojis()),
            search: InputState::new("")
                .with_placeholder("Search emoji")
                .with_focused(true)
                .with_max_len(120),
            service,
            focus: cx.focus_handle(),
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn copy(&mut self, index: usize, cx: &mut Context<Self>) {
        self.model.selected = index.min(self.model.filtered.len().saturating_sub(1));
        let Some(item) = self.model.selected_item() else {
            return;
        };
        let emoji = item.emoji.clone();
        cx.write_to_clipboard(gpui::ClipboardItem::new_string(emoji.clone()));
        let service = self.service.clone();
        services::spawn_blocking(move || service.copy_emoji(&emoji));
        Shell::close(cx);
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.control && !modifiers.platform && !modifiers.shift;
        let key = event.keystroke.key.as_str();
        match key {
            "escape" => {
                if self.search.is_empty() {
                    return;
                }
                self.search.clear();
                self.model.search(String::new());
            }
            "enter" => {
                let selected = self.model.selected;
                self.copy(selected, cx);
            }
            "up" | "down" | "pageup" | "pagedown" if plain => self.model.navigate(key),
            "left" | "right" | "home" | "end" if plain && self.search.is_empty() => {
                self.model.navigate(key)
            }
            _ => {
                if !self.search.handle_key_down(event, cx) {
                    return;
                }
                self.model.search(self.search.text());
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for EmojiApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let page = self.model.page();
        let start = page * PAGE_SIZE;
        let visible: Vec<_> = self
            .model
            .filtered
            .iter()
            .enumerate()
            .skip(start)
            .take(PAGE_SIZE)
            .map(|(index, item)| (index, item.emoji.clone()))
            .collect();
        let hover = theme.ghost_hover();
        let selected_name = self
            .model
            .selected_item()
            .map(|item| item.name.clone())
            .unwrap_or_default();
        let emoji_size = TextSize::XLarge.pixels(theme.font_size);

        let categories =
            div()
                .flex()
                .gap(Spacing::XXSmall.rems())
                .children(
                    CATEGORIES
                        .iter()
                        .enumerate()
                        .map(|(index, &(category, label))| {
                            let selected = self.model.category == category;
                            let base = if selected {
                                theme.element_selected()
                            } else {
                                gpui::transparent_black()
                            };
                            let category_hover = if selected {
                                mix(base, theme.text(), 0.06)
                            } else {
                                hover
                            };
                            div()
                                .id(ElementId::named_usize("emoji-category", index))
                                .flex_1()
                                .h(units(size::BAR_CHIP))
                                .flex()
                                .items_center()
                                .justify_center()
                                .rounded(Radius::Medium.rems())
                                .bg(base)
                                .text_color(if selected {
                                    theme.accent()
                                } else {
                                    theme.text_muted()
                                })
                                .text_size(TextSize::Small.pixels(theme.font_size))
                                .cursor_pointer()
                                .hover(move |style| style.bg(category_hover))
                                .transitions(|transitions| transitions.bg(motion::fast_cubic()))
                                .on_click(cx.listener(move |applet, _, _, cx| {
                                    applet.model.set_category(category);
                                    cx.notify();
                                }))
                                .child(label)
                        }),
                );

        let rows = visible.chunks(COLUMNS).map(|row| {
            div().flex().gap(Spacing::XXSmall.rems()).children(
                row.iter()
                    .map(|(index, emoji)| {
                        let index = *index;
                        let selected = index == self.model.selected;
                        div()
                            .id(ElementId::named_usize("emoji", index))
                            .flex_1()
                            .h(units(size::CONTROL + Spacing::Medium.value()))
                            .flex()
                            .items_center()
                            .justify_center()
                            .rounded(Radius::Medium.rems())
                            .text_size(emoji_size)
                            .bg(if selected {
                                theme.element_selected()
                            } else {
                                gpui::transparent_black()
                            })
                            .cursor_pointer()
                            .hover(move |style| style.bg(hover))
                            .transitions(|transitions| transitions.bg(motion::fast_cubic()))
                            .on_click(cx.listener(move |applet, _, _, cx| applet.copy(index, cx)))
                            .child(emoji.clone())
                    })
                    .map(IntoElement::into_any_element)
                    .chain((row.len()..COLUMNS).map(|_| div().flex_1().into_any_element())),
            )
        });

        panel(&theme)
            .id("emoji")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .w_full()
            .child(
                Input::from_state("emoji-search", &self.search)
                    .placeholder("Search emoji")
                    .prefix(
                        svg()
                            .path("search.svg")
                            .size(IconSize::Small.rems())
                            .text_color(theme.text_muted()),
                    )
                    .focused(true)
                    .w_full(),
            )
            .child(categories)
            .child(if visible.is_empty() {
                caption(&theme, "No emoji found").into_any_element()
            } else {
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::XXSmall.rems())
                    .children(rows)
                    .into_any_element()
            })
            .child(
                div()
                    .flex()
                    .justify_between()
                    .gap(Spacing::Medium.rems())
                    .text_size(TextSize::XSmall.pixels(theme.font_size))
                    .text_color(theme.text_muted())
                    .child(div().min_w_0().truncate().child(selected_name))
                    .child(format!("{}/{}", page + 1, self.model.pages())),
            )
    }
}
