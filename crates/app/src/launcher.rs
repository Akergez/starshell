use std::path::PathBuf;

use gpui::{
    AnyElement, AppContext, Context, ElementId, FocusHandle, InteractiveElement, IntoElement,
    KeyDownEvent, MouseButton, ParentElement, Render, ScrollHandle, SharedString,
    StatefulInteractiveElement, Styled, Task, Window, div, img, prelude::FluentBuilder, svg,
};
use services::{
    AppState, ContextOption, PopEvent, PopLauncher, PopRequest, PopResponse, SearchResult,
    description_line,
};
use ui::components::{Button, ButtonSize, ButtonVariant, Input, InputState, divider, panel};
use ui::theme::{Theme, mix};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};

use crate::shell::Shell;

pub const PREFIX_HINTS: &str = "/ files  ·  = calculate  ·  ? plugins";

struct GlobalLauncher(gpui::Entity<Launcher>);

impl gpui::Global for GlobalLauncher {}

pub struct Launcher {
    focus: FocusHandle,
    query: InputState,
    results: Vec<SearchResult>,
    icons: Vec<Option<PathBuf>>,
    selected: usize,
    actions: Option<(u32, Vec<ContextOption>)>,
    error: Option<String>,
    backend: PopLauncher,
    scroll: ScrollHandle,
    _events: Task<()>,
}

impl Launcher {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let backend = PopLauncher::new();
        let mut events = {
            let _guard = services::tokio_handle().enter();
            backend.connect()
        };
        let listener = cx.spawn(async move |this, cx| {
            while let Some(event) = events.recv().await {
                if this
                    .update(cx, |launcher: &mut Launcher, cx| {
                        launcher.handle_event(event, cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        backend.send(PopRequest::Search(String::new()));
        Self {
            focus: cx.focus_handle(),
            query: InputState::new("")
                .with_placeholder("Search apps, windows and files")
                .with_focused(true)
                .with_max_len(256),
            results: Vec::new(),
            icons: Vec::new(),
            selected: 0,
            actions: None,
            error: None,
            backend,
            scroll: ScrollHandle::new(),
            _events: listener,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    pub fn shared(window: &mut Window, cx: &mut gpui::App) -> gpui::Entity<Self> {
        if let Some(launcher) = cx.try_global::<GlobalLauncher>() {
            let launcher = launcher.0.clone();
            launcher.update(cx, |launcher, cx| launcher.reset(cx));
            return launcher;
        }
        let launcher = cx.new(|cx| Launcher::new(window, cx));
        cx.set_global(GlobalLauncher(launcher.clone()));
        launcher
    }

    fn reset(&mut self, cx: &mut Context<Self>) {
        self.query.clear();
        self.selected = 0;
        self.actions = None;
        self.scroll.scroll_to_item(0);
        if self.error.take().is_some() {
            let mut events = {
                let _guard = services::tokio_handle().enter();
                self.backend.connect()
            };
            self._events = cx.spawn(async move |this, cx| {
                while let Some(event) = events.recv().await {
                    if this
                        .update(cx, |launcher: &mut Launcher, cx| {
                            launcher.handle_event(event, cx)
                        })
                        .is_err()
                    {
                        break;
                    }
                }
            });
        }
        self.search();
        cx.notify();
    }

    fn search(&mut self) {
        self.actions = None;
        self.backend.send(PopRequest::Search(self.query.text()));
    }

    fn handle_event(&mut self, event: PopEvent, cx: &mut Context<Self>) {
        match event {
            PopEvent::Unavailable(error) => self.error = Some(error),
            PopEvent::Response(PopResponse::Update(results)) => {
                self.icons = results
                    .iter()
                    .map(|result| {
                        result
                            .icon
                            .as_ref()
                            .or(result.category_icon.as_ref())
                            .and_then(|icon| icon.resolve())
                    })
                    .collect();
                self.results = results;
                self.selected = 0;
                self.actions = None;
                self.error = None;
                self.scroll.scroll_to_item(0);
            }
            PopEvent::Response(PopResponse::Fill(text)) => {
                self.query.set_text(text);
                self.search();
            }
            PopEvent::Response(PopResponse::Context { id, options }) => {
                self.actions = (!options.is_empty()).then_some((id, options));
            }
            PopEvent::Response(PopResponse::DesktopEntry { path, .. }) => {
                if let Err(error) = cx.global::<AppState>().launcher.launch_desktop_entry(&path) {
                    self.error = Some(error.to_string());
                } else {
                    Shell::close(cx);
                }
            }
            PopEvent::Response(PopResponse::Close) => Shell::close(cx),
        }
        cx.notify();
    }

    fn selected_id(&self) -> Option<u32> {
        self.results.get(self.selected).map(|result| result.id)
    }

    fn select(&mut self, index: usize) {
        if index < self.results.len() {
            self.selected = index;
            self.actions = None;
            self.scroll.scroll_to_item(index);
        }
    }

    fn activate(&mut self, index: usize) {
        self.select(index);
        if let Some(id) = self.selected_id() {
            self.backend.send(PopRequest::Activate(id));
        }
    }

    fn show_actions(&mut self, index: usize) {
        self.select(index);
        if let Some(id) = self.selected_id() {
            self.backend.send(PopRequest::Context(id));
        }
    }

    fn handle_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => {
                if self.query.is_empty() {
                    return;
                }
                self.query.clear();
                self.search();
            }
            "enter" => {
                let index = self.selected;
                self.activate(index);
            }
            "tab" => {
                if let Some(id) = self.selected_id() {
                    self.backend.send(PopRequest::Complete(id));
                }
            }
            "down" => {
                let next = (self.selected + 1).min(self.results.len().saturating_sub(1));
                self.select(next);
            }
            "up" => {
                let previous = self.selected.saturating_sub(1);
                self.select(previous);
            }
            _ => {
                let before = self.query.text();
                if !self.query.handle_key_down(event, cx) {
                    return;
                }
                if self.query.text() != before {
                    self.search();
                }
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn result_row(&self, index: usize, theme: &Theme, cx: &mut Context<Self>) -> AnyElement {
        let result = &self.results[index];
        let selected = index == self.selected;
        let hover = if selected {
            mix(theme.element_selected(), theme.text(), 0.06)
        } else {
            theme.ghost_hover()
        };
        let icon: AnyElement = match self.icons.get(index).cloned().flatten() {
            Some(path) => img(path)
                .size(units(size::APPLICATION_ICON))
                .flex_shrink_0()
                .into_any_element(),
            None => div()
                .size(units(size::APPLICATION_ICON))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(
                    svg()
                        .path("app-window.svg")
                        .size(IconSize::XLarge.rems())
                        .text_color(theme.text_muted()),
                )
                .into_any_element(),
        };
        let description: SharedString = description_line(&result.description).into();
        let actions = self
            .actions
            .as_ref()
            .filter(|(id, _)| selected && *id == result.id)
            .map(|(id, options)| (*id, options.clone()));

        div()
            .flex()
            .flex_col()
            .w_full()
            .child(
                div()
                    .id(ElementId::named_usize("launcher-result", index))
                    .flex()
                    .items_center()
                    .w_full()
                    .gap(Spacing::Medium.rems())
                    .px(Spacing::Medium.rems())
                    .py(Spacing::Small.rems())
                    .rounded(Radius::Medium.rems())
                    .bg(if selected {
                        theme.element_selected()
                    } else {
                        gpui::transparent_black()
                    })
                    .cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .transitions(|transitions| transitions.bg(motion::fast_cubic()))
                    .on_click(cx.listener(move |launcher, _, _, cx| {
                        launcher.activate(index);
                        cx.notify();
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |launcher, _, _, cx| {
                            launcher.show_actions(index);
                            cx.stop_propagation();
                            cx.notify();
                        }),
                    )
                    .child(icon)
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .child(div().truncate().child(result.name.clone()))
                            .when(!description.is_empty(), |column| {
                                column.child(
                                    div()
                                        .truncate()
                                        .text_size(TextSize::Small.pixels(theme.font_size))
                                        .text_color(theme.text_muted())
                                        .child(description),
                                )
                            }),
                    ),
            )
            .when_some(actions, |row, (id, options)| {
                row.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(Spacing::Small.rems())
                        .pl(Spacing::XXXLarge.rems())
                        .py(Spacing::XSmall.rems())
                        .children(options.into_iter().map(|option| {
                            let context = option.id;
                            Button::new(ElementId::named_usize("launcher-action", context as usize))
                                .label(option.name)
                                .variant(ButtonVariant::Outline)
                                .size(ButtonSize::Sm)
                                .on_click(cx.listener(move |launcher, _, _, cx| {
                                    launcher
                                        .backend
                                        .send(PopRequest::ActivateContext { id, context });
                                    launcher.actions = None;
                                    cx.notify();
                                }))
                        })),
                )
            })
            .into_any_element()
    }
}

fn key_hint(theme: &Theme, action: &'static str, key: &'static str) -> AnyElement {
    div()
        .flex()
        .items_center()
        .gap(Spacing::XSmall.rems())
        .child(action)
        .child(
            div()
                .px(Spacing::XSmall.rems())
                .rounded(Radius::Small.rems())
                .bg(theme.elevated())
                .text_color(theme.text())
                .child(key),
        )
        .into_any_element()
}

impl Render for Launcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let rows: Vec<AnyElement> = (0..self.results.len())
            .map(|index| self.result_row(index, &theme, cx))
            .collect();
        let body: AnyElement = match &self.error {
            Some(error) => div()
                .flex_1()
                .flex()
                .items_center()
                .justify_center()
                .text_color(theme.error())
                .child(error.clone())
                .into_any_element(),
            None => div()
                .id("launcher-results")
                .track_scroll(&self.scroll)
                .flex_1()
                .min_h_0()
                .flex()
                .flex_col()
                .gap(Spacing::XXSmall.rems())
                .overflow_y_scroll()
                .children(rows)
                .into_any_element(),
        };
        let escape_label = if self.query.is_empty() {
            "Close"
        } else {
            "Clear"
        };

        panel(&theme)
            .id("launcher")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .w_full()
            .h(units(size::APPLET_HEIGHT))
            .gap(Spacing::Medium.rems())
            .p(Spacing::Large.rems())
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Small.rems())
                    .child(
                        svg()
                            .path("search.svg")
                            .size(IconSize::Medium.rems())
                            .text_color(theme.text_muted()),
                    )
                    .child(
                        Input::from_state("launcher-query", &self.query)
                            .placeholder("Search apps, windows and files")
                            .focused(true)
                            .bg(gpui::transparent_black())
                            .border_color(gpui::transparent_black())
                            .focused_border_color(gpui::transparent_black())
                            .max_visible_chars(64)
                            .flex_1(true),
                    ),
            )
            .child(divider(&theme))
            .child(body)
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(Spacing::Medium.rems())
                    .text_size(TextSize::XSmall.pixels(theme.font_size))
                    .text_color(theme.text_muted())
                    .child(div().min_w_0().truncate().child(PREFIX_HINTS))
                    .child(
                        div()
                            .flex()
                            .flex_shrink_0()
                            .gap(Spacing::Medium.rems())
                            .child(key_hint(&theme, "Open", "Enter"))
                            .child(key_hint(&theme, "Complete", "Tab"))
                            .child(key_hint(&theme, "Actions", "Right click"))
                            .child(key_hint(&theme, escape_label, "Esc")),
                    ),
            )
    }
}
