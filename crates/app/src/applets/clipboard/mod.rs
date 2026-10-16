mod model;
mod worker;

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, ScrollHandle, StatefulInteractiveElement, Styled, Task, Window, div,
    img, prelude::FluentBuilder, svg,
};
use model::{ClipboardModel, ClipboardTab};
use services::{AppState, clipboard::ClipboardContent};
use ui::components::{
    Button, ButtonSize, ButtonVariant, IconButton, Input, InputState, ListRow, SegmentedControl,
    caption, panel,
};
use ui::theme::{Theme, mix};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};
use worker::Request;

use crate::shell::Shell;

pub struct ClipboardApplet {
    model: ClipboardModel,
    search: InputState,
    loading: bool,
    busy: bool,
    error: Option<String>,
    pinned: Vec<String>,
    images: HashMap<String, Arc<gpui::Image>>,
    scroll: ScrollHandle,
    focus: FocusHandle,
    sender: tokio::sync::mpsc::UnboundedSender<Request>,
    worker: tokio::task::JoinHandle<()>,
    _replies: Task<()>,
}

impl ClipboardApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (sender, mut receiver, worker) = {
            let _guard = services::tokio_handle().enter();
            worker::start(cx.global::<AppState>().clipboard.clone())
        };
        let replies = cx.spawn(async move |this, cx| {
            while let Some(reply) = receiver.recv().await {
                let updated = this.update(cx, |applet: &mut ClipboardApplet, cx| {
                    applet.loading = false;
                    if reply.finished {
                        applet.busy = false;
                    }
                    if reply.finished || reply.error.is_some() {
                        applet.error = reply.error;
                    }
                    applet.pinned = reply.pinned;
                    applet.images = reply.images;
                    applet.model.update(reply.items, reply.snippets);
                    if let Some((_, content)) = reply.copied {
                        if let ClipboardContent::Text(text) = content {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(text));
                        }
                        Shell::close(cx);
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        });
        let _ = sender.send(Request::Active(true));
        let _ = sender.send(Request::Refresh);
        Self {
            model: ClipboardModel::default(),
            search: InputState::new("")
                .with_placeholder("Search history")
                .with_focused(true)
                .with_max_len(120),
            loading: true,
            busy: false,
            error: None,
            pinned: Vec::new(),
            images: HashMap::new(),
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle(),
            sender,
            worker,
            _replies: replies,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn request(&mut self, request: Request, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.error = None;
        if self.sender.send(request).is_ok() {
            self.busy = true;
        } else {
            self.error = Some("Could not update clipboard history".to_string());
        }
        cx.notify();
    }

    fn copy(&mut self, position: usize, cx: &mut Context<Self>) {
        let Some(&index) = self.model.filtered.get(position) else {
            return;
        };
        let request = match self.model.tab {
            ClipboardTab::History => Request::Copy(self.model.items[index].clone(), 0),
            ClipboardTab::Snippets => {
                Request::CopyText(self.model.snippets[index].content.clone(), 0)
            }
        };
        self.request(request, cx);
    }

    fn switch_tab(&mut self, tab: ClipboardTab, cx: &mut Context<Self>) {
        self.model.switch(tab);
        self.scroll.scroll_to_item(0);
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        let plain = !modifiers.control && !modifiers.platform && !modifiers.shift;
        match event.keystroke.key.as_str() {
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
            "tab" if plain => {
                let tab = match self.model.tab {
                    ClipboardTab::History => ClipboardTab::Snippets,
                    ClipboardTab::Snippets => ClipboardTab::History,
                };
                self.switch_tab(tab, cx);
            }
            "up" | "down" if plain => {
                self.model
                    .navigate(if event.keystroke.key == "up" { -1 } else { 1 });
                self.scroll.scroll_to_item(self.model.selected);
            }
            _ => {
                if !self.search.handle_key_down(event, cx) {
                    return;
                }
                self.model.search(self.search.text());
                self.scroll.scroll_to_item(0);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn row(&self, position: usize, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let index = self.model.filtered[position];
        let selected = position == self.model.selected;
        let id = ElementId::named_usize("clipboard-item", position);
        match self.model.tab {
            ClipboardTab::History => {
                let item = &self.model.items[index];
                if item.is_image {
                    let hover = mix(theme.elevated(), theme.text(), 0.06);
                    return div()
                        .id(id)
                        .w_full()
                        .h(units(size::COVER))
                        .rounded(Radius::Medium.rems())
                        .overflow_hidden()
                        .border_1()
                        .border_color(if selected {
                            theme.accent()
                        } else {
                            gpui::transparent_black()
                        })
                        .bg(theme.elevated())
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .transitions(|transitions| {
                            transitions
                                .bg(motion::fast_cubic())
                                .border_color(motion::fast_cubic())
                        })
                        .on_click(cx.listener(move |applet, _, _, cx| applet.copy(position, cx)))
                        .flex()
                        .items_center()
                        .justify_center()
                        .map(|frame| match self.images.get(&item.id) {
                            Some(image) => frame.child(img(image.clone()).size_full()),
                            None => frame.child(
                                svg()
                                    .path("image.svg")
                                    .size(IconSize::XLarge.rems())
                                    .text_color(theme.text_muted()),
                            ),
                        })
                        .into_any_element();
                }
                let pinned = self.pinned.contains(&item.id);
                let preview = if item.preview.is_empty() {
                    "[Empty item]".to_string()
                } else {
                    item.preview.replace('\n', " ")
                };
                ListRow::new(id, preview)
                    .selected(selected)
                    .on_click(cx.listener(move |applet, _, _, cx| applet.copy(position, cx)))
                    .action(
                        IconButton::new(
                            ElementId::named_usize("clipboard-pin", position),
                            if pinned { "pin.svg" } else { "pin-tilted.svg" },
                        )
                        .tooltip("Pin snippet")
                        .when(pinned, |button| button.icon_color(theme.accent()))
                        .on_click(cx.listener(move |applet, _, _, cx| {
                            let Some(&index) = applet.model.filtered.get(position) else {
                                return;
                            };
                            let item = applet.model.items[index].clone();
                            applet.request(Request::Pin(item), cx);
                        })),
                    )
                    .into_any_element()
            }
            ClipboardTab::Snippets => {
                let snippet = &self.model.snippets[index];
                ListRow::new(id, snippet.title.clone())
                    .status(snippet.content.replace('\n', " "))
                    .selected(selected)
                    .on_click(cx.listener(move |applet, _, _, cx| applet.copy(position, cx)))
                    .action(
                        IconButton::new(
                            ElementId::named_usize("clipboard-remove", position),
                            "trash.svg",
                        )
                        .tooltip("Delete snippet")
                        .on_click(cx.listener(move |applet, _, _, cx| {
                            let Some(&index) = applet.model.filtered.get(position) else {
                                return;
                            };
                            let id = applet.model.snippets[index].id.clone();
                            applet.request(Request::Remove(id), cx);
                        })),
                    )
                    .into_any_element()
            }
        }
    }
}

impl Drop for ClipboardApplet {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Active(false));
        self.worker.abort();
    }
}

impl Render for ClipboardApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let rows: Vec<AnyElement> = (0..self.model.filtered.len())
            .map(|position| self.row(position, &theme, cx))
            .collect();
        let empty = if self.loading {
            "Working…"
        } else if !self.model.query.is_empty() {
            "No matching items"
        } else if self.model.tab == ClipboardTab::History {
            "No items in history"
        } else {
            "No pinned snippets"
        };
        let history = self.model.tab == ClipboardTab::History;

        panel(&theme)
            .id("clipboard")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .w_full()
            .child(
                Input::from_state("clipboard-search", &self.search)
                    .placeholder("Search history")
                    .prefix(
                        svg()
                            .path("search.svg")
                            .size(IconSize::Small.rems())
                            .text_color(theme.text_muted()),
                    )
                    .focused(true)
                    .w_full(),
            )
            .child(
                SegmentedControl::new("clipboard-tabs")
                    .option("History")
                    .option("Snippets")
                    .selected(Some(if history { 0 } else { 1 }))
                    .on_select({
                        let entity = cx.entity().downgrade();
                        move |index, _, cx| {
                            let tab = if index == 0 {
                                ClipboardTab::History
                            } else {
                                ClipboardTab::Snippets
                            };
                            let _ = entity.update(cx, |applet, cx| applet.switch_tab(tab, cx));
                        }
                    }),
            )
            .map(|panel| {
                if rows.is_empty() {
                    panel.child(caption(&theme, empty))
                } else {
                    panel.child(
                        div()
                            .id("clipboard-list")
                            .track_scroll(&self.scroll)
                            .flex()
                            .flex_col()
                            .gap(Spacing::XXSmall.rems())
                            .max_h(units(size::APPLET_HEIGHT))
                            .overflow_y_scroll()
                            .children(rows),
                    )
                }
            })
            .when_some(self.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .text_size(TextSize::Small.pixels(theme.font_size))
                        .text_color(theme.error())
                        .child(error),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(Spacing::Medium.rems())
                    .child(
                        div()
                            .min_w_0()
                            .truncate()
                            .text_size(TextSize::XSmall.pixels(theme.font_size))
                            .text_color(theme.text_muted())
                            .child("↑↓ select · Enter copy · Tab switch"),
                    )
                    .when(history, |footer| {
                        footer.child(
                            Button::new("clipboard-clear")
                                .label("Clear history")
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm)
                                .disabled(self.busy || self.model.items.is_empty())
                                .on_click(cx.listener(|applet, _, _, cx| {
                                    applet.request(Request::Clear, cx)
                                })),
                        )
                    }),
            )
    }
}
