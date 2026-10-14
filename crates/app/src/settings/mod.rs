pub(crate) mod animator;
pub(crate) mod appearance;
pub(crate) mod header;
pub(crate) mod sidebar;
pub mod views;

use std::time::{Duration, Instant};

use gpui::{
    Context, FocusHandle, IntoElement, KeyDownEvent, ParentElement, Render, ScrollHandle,
    Styled, Window, div, prelude::*, px,
};
use ui::theme::Theme;

use crate::capsule::widgets::style;
use crate::panel::SettingsPanel;
use animator::SettingsAnimator;
use sidebar::{SettingsTab, Sidebar};
use views::{
    SettingsView, appearance::AppearanceView, apps::AppsView, lockscreen::LockScreenView,
    media::MediaView, record::RecordView, system::SystemView,
};

pub const WIDTH: f32 = 1120.0;
pub const HEIGHT: f32 = 780.0;

pub struct Settings {
    focus_handle: FocusHandle,
    animator: SettingsAnimator,
    handle: gpui::AnyWindowHandle,
    frame_pending: bool,
    active_tab: SettingsTab,
    sidebar: Sidebar,
    views: Vec<(SettingsTab, Box<dyn SettingsView>)>,
    pub search_query: String,
    pub is_searching: bool,
    scroll_handle: ScrollHandle,
}

impl Settings {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let focus_handle = cx.focus_handle();
        let animation_duration_ms = if cx.has_global::<services::AppState>() {
            cx.global::<services::AppState>()
                .config
                .get()
                .ui
                .animation_duration_ms as u64
        } else {
            250
        };

        let views: Vec<(SettingsTab, Box<dyn SettingsView>)> = vec![
            (SettingsTab::Appearance, Box::new(AppearanceView::new())),
            (SettingsTab::Apps, Box::new(AppsView::new())),
            (SettingsTab::Media, Box::new(MediaView::new())),
            (SettingsTab::LockScreen, Box::new(LockScreenView::new())),
            (SettingsTab::System, Box::new(SystemView::new())),
            (SettingsTab::Record, Box::new(RecordView::new())),
        ];

        let settings = Self {
            focus_handle,
            animator: SettingsAnimator::new(animation_duration_ms),
            handle: window.window_handle(),
            frame_pending: true,
            active_tab: SettingsTab::Appearance,
            sidebar: Sidebar::new(),
            views,
            search_query: String::new(),
            is_searching: false,
            scroll_handle: ScrollHandle::new(),
        };

        window.focus(&settings.focus_handle, cx);
        Self::queue_frame(window, cx.entity().downgrade());
        settings
    }

    pub fn view_as_mut<T: 'static>(&mut self) -> Option<&mut T> {
        for (_, view) in &mut self.views {
            if let Some(v) = view.as_any_mut().downcast_mut::<T>() {
                return Some(v);
            }
        }
        None
    }

    pub fn request_close(&mut self, cx: &mut Context<Self>) {
        if self.animator.is_closing() {
            return;
        }
        self.animator.request_close();
        if !self.frame_pending {
            self.frame_pending = true;
            let entity = cx.entity().downgrade();
            let handle = self.handle;
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, _| Self::queue_frame(window, entity));
            });
        }
        cx.notify();
    }

    pub fn set_tab(&mut self, tab: SettingsTab, cx: &mut Context<Self>) {
        if self.active_tab != tab {
            self.active_tab = tab;
            self.search_query.clear();
            self.is_searching = false;
            self.scroll_handle.scroll_to_item(0);

            let duration = if cx.has_global::<services::AppState>() {
                Duration::from_millis(
                    cx.global::<services::AppState>()
                        .config
                        .get()
                        .ui
                        .animation_duration_ms as u64,
                )
            } else {
                Duration::from_millis(200)
            };
            self.animator.start_section_transition(duration);
            if !self.frame_pending {
                self.frame_pending = true;
                let entity = cx.entity().downgrade();
                let handle = self.handle;
                cx.defer(move |cx| {
                    let _ = handle.update(cx, |_, window, _| Self::queue_frame(window, entity));
                });
            }
            cx.notify();
        }
    }

    fn queue_frame(window: &Window, entity: gpui::WeakEntity<Self>) {
        window.on_next_frame(move |window, cx| {
            let _ = entity.update(cx, |this, cx| {
                let (running, closed) = this.animator.advance(Instant::now());
                if closed {
                    window.remove_window();
                    return;
                }
                this.frame_pending = running;
                cx.notify();
                if running {
                    Self::queue_frame(window, cx.entity().downgrade());
                }
            });
        });
    }

    fn handle_key_down(
        &mut self,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        cx.stop_propagation();
        let key = event.keystroke.key.as_str();
        let ctrl = event.keystroke.modifiers.control || event.keystroke.modifiers.platform;
        let alt = event.keystroke.modifiers.alt;

        // Global hotkey: Ctrl+F to activate search
        if ctrl && key == "f" {
            self.is_searching = true;
            window.focus(&self.focus_handle, cx);
            cx.notify();
            return;
        }

        // Escape: clear query, exit search, or close modal
        if key == "escape" {
            if self.is_searching && !self.search_query.is_empty() {
                self.search_query.clear();
                self.scroll_handle.scroll_to_item(0);
                cx.notify();
                return;
            }
            if self.is_searching {
                self.is_searching = false;
                cx.notify();
                return;
            }
            self.request_close(cx);
            return;
        }

        // When search is active, handle text entry and navigation
        if self.is_searching {
            if ctrl {
                match key {
                    "v" => {
                        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
                            let clean: String = text.chars().filter(|c| !c.is_control()).collect();
                            if !clean.is_empty() {
                                self.search_query.push_str(&clean);
                                self.scroll_handle.scroll_to_item(0);
                                cx.notify();
                            }
                        }
                        return;
                    }
                    "c" => {
                        if !self.search_query.is_empty() {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.search_query.clone()));
                        }
                        return;
                    }
                    "x" => {
                        if !self.search_query.is_empty() {
                            cx.write_to_clipboard(gpui::ClipboardItem::new_string(self.search_query.clone()));
                            self.search_query.clear();
                            self.scroll_handle.scroll_to_item(0);
                            cx.notify();
                        }
                        return;
                    }
                    "u" | "w" | "a" => {
                        self.search_query.clear();
                        self.scroll_handle.scroll_to_item(0);
                        cx.notify();
                        return;
                    }
                    _ => return,
                }
            }

            if alt {
                return;
            }

            match key {
                "backspace" => {
                    if self.search_query.pop().is_some() {
                        self.scroll_handle.scroll_to_item(0);
                        cx.notify();
                    }
                    return;
                }
                "space" => {
                    self.search_query.push(' ');
                    self.scroll_handle.scroll_to_item(0);
                    cx.notify();
                    return;
                }
                "enter" | "return" => {
                    return;
                }
                _ => {
                    if let Some(ref ch) = event.keystroke.key_char {
                        let clean: String = ch.chars().filter(|c| !c.is_control()).collect();
                        if !clean.is_empty() {
                            self.search_query.push_str(&clean);
                            self.scroll_handle.scroll_to_item(0);
                            cx.notify();
                            return;
                        }
                    } else if key.chars().count() == 1 {
                        let c = key.chars().next().unwrap();
                        if !c.is_control() {
                            self.search_query.push(c);
                            self.scroll_handle.scroll_to_item(0);
                            cx.notify();
                            return;
                        }
                    }
                }
            }
            return;
        }

        // Tab switches sidebar sections when not searching
        if key == "tab" && !self.is_searching {
            let next_tab = match self.active_tab {
                SettingsTab::Appearance => SettingsTab::Apps,
                SettingsTab::Apps => SettingsTab::Media,
                SettingsTab::Media => SettingsTab::LockScreen,
                SettingsTab::LockScreen => SettingsTab::System,
                SettingsTab::System => SettingsTab::Record,
                SettingsTab::Record => SettingsTab::Appearance,
            };
            self.set_tab(next_tab, cx);
            return;
        }

        if let Some((_, view)) = self.views.iter_mut().find(|(t, _)| *t == self.active_tab) {
            if view.handle_key_down(event, window, cx) {
                return;
            }
        }
    }
}

impl Drop for Settings {
    fn drop(&mut self) {
        SettingsPanel::mark_closed();
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let capsule_radius = if cx.has_global::<services::AppState>() {
            cx.global::<services::AppState>()
                .config
                .get()
                .ui
                .capsule_round
        } else {
            24.0
        };

        let viewport = window.viewport_size();
        let width = (f32::from(viewport.width) - 48.0).clamp(0.0, WIDTH);
        let height = (f32::from(viewport.height) - 48.0).clamp(0.0, HEIGHT);

        let active_tab = self.active_tab;
        let progress = self.animator.section_progress();

        let active_view_content = if !self.search_query.trim().is_empty() {
            views::search::render_search_results(&self.search_query, &theme, cx)
        } else if let Some((_, view)) = self.views.iter_mut().find(|(t, _)| *t == active_tab) {
            view.render(window, cx)
        } else {
            div().into_any_element()
        };

        let header = header::render_header(&self.search_query, self.is_searching, &theme, cx);
        let sidebar_element = self
            .sidebar
            .render(self.active_tab, !self.search_query.is_empty(), &theme, cx);

        let card = div()
            .id("settings-modal-card")
            .occlude()
            .on_click(cx.listener(|this, _, window, cx| {
                window.focus(&this.focus_handle, cx);
            }))
            .relative()
            .top(px(self
                .animator
                .offset_y
                .min((f32::from(viewport.height) - height).max(0.0) / 2.0)))
            .w(px(width))
            .h(px(height))
            .max_w_full()
            .max_h_full()
            .flex()
            .flex_col()
            .gap(px(20.0))
            .p(px(24.0))
            .flex_shrink_0()
            .rounded(px(capsule_radius))
            .bg(style::background(&theme))
            .border_1()
            .border_color(style::border(&theme))
            .shadow_xl()
            .font_family(theme.font_family())
            .text_color(theme.foreground())
            .overflow_hidden()
            .opacity(self.animator.card_opacity)
            .on_mouse_move(cx.listener(|this, event: &gpui::MouseMoveEvent, _, cx| {
                if let Some((_, view)) = this.views.iter_mut().find(|(t, _)| *t == this.active_tab) {
                    if view.handle_mouse_move(event, cx) {
                        cx.notify();
                    }
                }
            }))
            .capture_any_mouse_up(cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                if let Some((_, view)) = this.views.iter_mut().find(|(t, _)| *t == this.active_tab) {
                    if view.handle_mouse_up(event, cx) {
                        cx.notify();
                    }
                }
            }))
            .on_mouse_up(
                gpui::MouseButton::Left,
                cx.listener(|this, event: &gpui::MouseUpEvent, _, cx| {
                    if let Some((_, view)) = this.views.iter_mut().find(|(t, _)| *t == this.active_tab) {
                        if view.handle_mouse_up(event, cx) {
                            cx.notify();
                        }
                    }
                }),
            )
            .child(header)
            .child(
                div()
                    .flex()
                    .gap(px(28.0))
                    .flex_1()
                    .min_h_0()
                    .min_w_0()
                    .w_full()
                    .child(sidebar_element)
                    .child(
                        div()
                            .id("settings-content-scroll")
                            .track_scroll(&self.scroll_handle)
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
                                    .when(progress < 1.0, |s| s.blur(px((1.0 - progress) * 5.0)))
                                    .child(active_view_content),
                            ),
                    ),
            );

        let entity = cx.entity().downgrade();
        let entity_move = entity.clone();
        let entity_up = entity.clone();

        div()
            .id("settings-window-root")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(Self::handle_key_down))
            .on_mouse_move_all(move |event, phase, _hitbox, _window, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    let _ = entity_move.update(cx, |this, cx| {
                        if let Some((_, view)) = this.views.iter_mut().find(|(t, _)| *t == this.active_tab) {
                            if view.handle_mouse_move(event, cx) {
                                cx.notify();
                            }
                        }
                    });
                }
            })
            .on_mouse_up_all(move |event, phase, _hitbox, _window, cx| {
                if phase == gpui::DispatchPhase::Bubble {
                    let _ = entity_up.update(cx, |this, cx| {
                        if let Some((_, view)) = this.views.iter_mut().find(|(t, _)| *t == this.active_tab) {
                            if view.handle_mouse_up(event, cx) {
                                cx.notify();
                            }
                        }
                    });
                }
            })
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
}
