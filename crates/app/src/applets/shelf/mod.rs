mod worker;

use std::collections::HashMap;
use std::sync::Arc;

use gpui::{
    AnyElement, Context, ElementId, ExternalPaths, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, ParentElement, Render, ScrollHandle, StatefulInteractiveElement, Styled, Task,
    Window, div, img, prelude::FluentBuilder, svg,
};
use services::{AppState, ShelfItem};
use ui::components::{Button, ButtonSize, ButtonVariant, IconButton, panel};
use ui::theme::{Theme, mix};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};
use worker::Request;

pub struct ShelfApplet {
    items: Vec<ShelfItem>,
    previews: HashMap<String, Arc<gpui::Image>>,
    selected: usize,
    busy: bool,
    error: bool,
    copied: bool,
    scroll: ScrollHandle,
    focus: FocusHandle,
    sender: tokio::sync::mpsc::UnboundedSender<Request>,
    worker: tokio::task::JoinHandle<()>,
    _updates: Task<()>,
}

pub fn size_label(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

impl ShelfApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let (sender, mut receiver, worker) = {
            let _guard = services::tokio_handle().enter();
            worker::start(cx.global::<AppState>().shelf.clone())
        };
        let updates = cx.spawn(async move |this, cx| {
            while let Some(reply) = receiver.recv().await {
                let updated = this.update(cx, |applet: &mut ShelfApplet, cx| {
                    let selected = applet
                        .items
                        .get(applet.selected)
                        .map(|item| item.id.clone());
                    applet.items = reply.items;
                    applet.previews = reply.previews;
                    applet.selected = selected
                        .and_then(|id| applet.items.iter().position(|item| item.id == id))
                        .unwrap_or(applet.selected.min(applet.items.len().saturating_sub(1)));
                    if reply.finished {
                        applet.busy = false;
                        applet.error = reply.error;
                    }
                    if reply.copied.is_some() {
                        applet.copied = true;
                    }
                    cx.notify();
                });
                if updated.is_err() {
                    break;
                }
            }
        });
        let _ = sender.send(Request::Refresh);
        Self {
            items: Vec::new(),
            previews: HashMap::new(),
            selected: 0,
            busy: false,
            error: false,
            copied: false,
            scroll: ScrollHandle::new(),
            focus: cx.focus_handle(),
            sender,
            worker,
            _updates: updates,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn request(&mut self, request: Request, cx: &mut Context<Self>) {
        if self.busy {
            return;
        }
        self.busy = self.sender.send(request).is_ok();
        self.error = !self.busy;
        self.copied = false;
        cx.notify();
    }

    fn copy(&mut self, all: bool, cx: &mut Context<Self>) {
        let paths = if all {
            self.items.iter().map(|item| item.path.clone()).collect()
        } else {
            self.items
                .get(self.selected)
                .map(|item| vec![item.path.clone()])
                .unwrap_or_default()
        };
        self.request(Request::Copy(paths, 0), cx);
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let control = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        match event.keystroke.key.as_str() {
            "enter" => self.copy(false, cx),
            "c" if control => self.copy(false, cx),
            "a" if control => self.copy(true, cx),
            "delete" | "backspace" => {
                if let Some(item) = self.items.get(self.selected) {
                    let id = item.id.clone();
                    self.request(Request::Remove(id), cx);
                }
            }
            "up" => self.selected = self.selected.saturating_sub(1),
            "down" => self.selected = (self.selected + 1).min(self.items.len().saturating_sub(1)),
            _ => return,
        }
        self.scroll.scroll_to_item(self.selected);
        cx.stop_propagation();
        cx.notify();
    }

    fn thumbnail(&self, item: &ShelfItem, theme: &Theme) -> AnyElement {
        let frame = div()
            .size(units(size::APPLICATION_ICON))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .rounded(Radius::Small.rems())
            .overflow_hidden();
        if let Some(preview) = self.previews.get(&item.id) {
            return frame
                .child(img(preview.clone()).size_full())
                .into_any_element();
        }
        if let Some(icon) = item.icon_path.clone() {
            return frame.child(img(icon).size_full()).into_any_element();
        }
        frame
            .child(
                svg()
                    .path(if item.is_dir {
                        "folder.svg"
                    } else {
                        "file.svg"
                    })
                    .size(IconSize::Large.rems())
                    .text_color(theme.text_muted()),
            )
            .into_any_element()
    }
}

impl Drop for ShelfApplet {
    fn drop(&mut self) {
        self.worker.abort();
    }
}

impl Render for ShelfApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let small = TextSize::Small.pixels(theme.font_size);
        let hover = theme.ghost_hover();
        let rows: Vec<AnyElement> = self
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let selected = index == self.selected;
                let base = if selected {
                    theme.element_selected()
                } else {
                    gpui::transparent_black()
                };
                let row_hover = if selected {
                    mix(base, theme.text(), 0.06)
                } else {
                    hover
                };
                let id = item.id.clone();
                div()
                    .id(ElementId::named_usize("shelf-item", index))
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.rems())
                    .px(Spacing::Medium.rems())
                    .py(Spacing::Small.rems())
                    .rounded(Radius::Medium.rems())
                    .bg(base)
                    .cursor_pointer()
                    .hover(move |style| style.bg(row_hover))
                    .transitions(|transitions| transitions.bg(motion::fast_cubic()))
                    .on_click(cx.listener(move |applet, _, _, cx| {
                        applet.selected = index;
                        applet.copy(false, cx);
                    }))
                    .child(self.thumbnail(item, &theme))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().text_size(small).child(item.name.clone()))
                            .child(
                                div()
                                    .truncate()
                                    .text_size(small)
                                    .text_color(theme.text_muted())
                                    .child(if item.is_dir {
                                        "Folder".to_string()
                                    } else {
                                        size_label(item.size_bytes)
                                    }),
                            ),
                    )
                    .child(
                        IconButton::new(ElementId::named_usize("shelf-remove", index), "x.svg")
                            .tooltip("Remove")
                            .on_click(cx.listener(move |applet, _, _, cx| {
                                applet.request(Request::Remove(id.clone()), cx)
                            })),
                    )
                    .into_any_element()
            })
            .collect();
        let status = if self.error {
            Some(("Could not update the shelf", theme.error()))
        } else if self.copied {
            Some(("Copied to the clipboard", theme.accent()))
        } else {
            None
        };

        panel(&theme)
            .id("shelf")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .on_drop(cx.listener(|applet, paths: &ExternalPaths, _, cx| {
                let paths = paths.paths().to_vec();
                applet.request(Request::Add(paths), cx);
            }))
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .child("Shelf")
                    .child(
                        div()
                            .text_size(small)
                            .text_color(theme.text_muted())
                            .child(format!("{} items", self.items.len())),
                    ),
            )
            .map(|panel| {
                if rows.is_empty() {
                    panel.child(
                        div()
                            .flex()
                            .flex_col()
                            .items_center()
                            .gap(Spacing::Small.rems())
                            .py(Spacing::XLarge.rems())
                            .border_1()
                            .border_dashed()
                            .border_color(theme.border_variant())
                            .rounded(Radius::Large.rems())
                            .text_size(small)
                            .text_color(theme.text_muted())
                            .child(
                                svg()
                                    .path("folder-open.svg")
                                    .size(IconSize::XLarge.rems())
                                    .text_color(theme.text_muted()),
                            )
                            .child("Drop files here to keep them at hand"),
                    )
                } else {
                    panel.child(
                        div()
                            .id("shelf-list")
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
            .when_some(status, |panel, (text, color)| {
                panel.child(div().text_size(small).text_color(color).child(text))
            })
            .when(!self.items.is_empty(), |panel| {
                panel.child(
                    div()
                        .flex()
                        .gap(Spacing::Small.rems())
                        .child(
                            div().flex_1().child(
                                Button::new("shelf-copy-all")
                                    .label("Copy all")
                                    .variant(ButtonVariant::Outline)
                                    .size(ButtonSize::Md)
                                    .w_full(true)
                                    .disabled(self.busy)
                                    .on_click(
                                        cx.listener(|applet, _, _, cx| applet.copy(true, cx)),
                                    ),
                            ),
                        )
                        .child(
                            div().flex_1().child(
                                Button::new("shelf-clear")
                                    .label("Clear")
                                    .variant(ButtonVariant::Ghost)
                                    .size(ButtonSize::Md)
                                    .w_full(true)
                                    .disabled(self.busy)
                                    .on_click(cx.listener(|applet, _, _, cx| {
                                        applet.request(Request::Clear, cx)
                                    })),
                            ),
                        ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes_use_binary_units() {
        assert_eq!(size_label(512), "512 B");
        assert_eq!(size_label(1536), "1.5 KB");
        assert_eq!(size_label(5 * 1024 * 1024), "5.0 MB");
    }
}
