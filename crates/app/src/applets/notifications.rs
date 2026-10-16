use gpui::{
    Context, ElementId, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, StatefulInteractiveElement, Styled, Task, Window, div, prelude::FluentBuilder,
};
use services::{NotificationItem, NotificationStore};
use ui::components::{
    Button, ButtonSize, ButtonVariant, Input, InputState, Switch, caption, panel,
};
use ui::theme::Theme;
use ui::tokens::{Spacing, TextSize, size, units};

use crate::notifications::render_card;

struct Reply {
    id: u32,
    input: InputState,
    pending: bool,
    failed: bool,
}

pub struct NotificationsApplet {
    items: Vec<NotificationItem>,
    reply: Option<Reply>,
    focus: FocusHandle,
    _changes: Task<()>,
    _response: Option<Task<()>>,
}

impl NotificationsApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let mut changes = NotificationStore::global().subscribe();
        let listener = cx.spawn(async move |this, cx| {
            while changes.changed().await.is_ok() {
                if this
                    .update(cx, |applet: &mut NotificationsApplet, cx| {
                        applet.refresh(cx)
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let mut items = NotificationStore::global().get_all_notifications();
        items.reverse();
        Self {
            items,
            reply: None,
            focus: cx.focus_handle(),
            _changes: listener,
            _response: None,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        let store = NotificationStore::global();
        self.items = store.get_all_notifications();
        self.items.reverse();
        if self
            .reply
            .as_ref()
            .is_some_and(|reply| !store.contains_notification(reply.id))
        {
            self.reply = None;
        }
        cx.notify();
    }

    fn start_reply(&mut self, id: u32, cx: &mut Context<Self>) {
        self.reply = Some(Reply {
            id,
            input: InputState::new("").with_focused(true).with_max_len(250),
            pending: false,
            failed: false,
        });
        cx.notify();
    }

    fn send_reply(&mut self, cx: &mut Context<Self>) {
        let Some(reply) = self
            .reply
            .as_mut()
            .filter(|reply| !reply.pending && !reply.input.is_empty())
        else {
            return;
        };
        let id = reply.id;
        let text = reply.input.text();
        reply.pending = true;
        reply.failed = false;
        let (sender, receiver) = tokio::sync::oneshot::channel();
        services::spawn_tokio(async move {
            let _ = sender.send(NotificationStore::global().reply(id, text).await);
        });
        self._response = Some(cx.spawn(async move |this, cx| {
            let result = receiver.await;
            let _ = this.update(cx, |applet, cx| {
                if let Some(reply) = applet.reply.as_mut().filter(|reply| reply.id == id) {
                    reply.pending = false;
                    reply.failed = !matches!(result, Ok(Ok(())));
                    if !reply.failed {
                        applet.reply = None;
                    }
                    cx.notify();
                }
            });
        }));
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(reply) = self.reply.as_mut() else {
            return;
        };
        match event.keystroke.key.as_str() {
            "escape" => self.reply = None,
            "enter" => self.send_reply(cx),
            _ if reply.pending => return,
            _ => {
                if !reply.input.handle_key_down(event, cx) {
                    return;
                }
                reply.failed = false;
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Render for NotificationsApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let store = NotificationStore::global();
        let entity = cx.entity().downgrade();
        let cards = self.items.iter().map(|item| {
            let entity = entity.clone();
            let card = render_card(item, &theme, move |id, cx| {
                let _ = entity.update(cx, |applet, cx| applet.start_reply(id, cx));
            });
            match self.reply.as_ref().filter(|reply| reply.id == item.id) {
                Some(reply) => card
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Small.rems())
                            .child(
                                Input::from_state(
                                    ElementId::named_usize("notification-reply", item.id as usize),
                                    &reply.input,
                                )
                                .placeholder("Reply")
                                .focused(!reply.pending)
                                .disabled(reply.pending)
                                .error(reply.failed)
                                .flex_1(true),
                            )
                            .child(
                                Button::new(ElementId::named_usize(
                                    "notification-send",
                                    item.id as usize,
                                ))
                                .label("Send")
                                .size(ButtonSize::Sm)
                                .disabled(reply.pending || reply.input.is_empty())
                                .on_click(cx.listener(|applet, _, _, cx| applet.send_reply(cx))),
                            ),
                    )
                    .into_any_element(),
                None => card.into_any_element(),
            }
        });

        panel(&theme)
            .id("notifications")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Medium.rems())
                    .child(div().flex_1().child("Notifications"))
                    .child(
                        div()
                            .text_size(TextSize::Small.pixels(theme.font_size))
                            .text_color(theme.text_muted())
                            .child("Do not disturb"),
                    )
                    .child(
                        Switch::new("notifications-dnd")
                            .checked(store.is_dnd_enabled())
                            .on_click(|_, _, _| {
                                NotificationStore::global().toggle_dnd();
                            }),
                    ),
            )
            .when(self.items.is_empty(), |panel| {
                panel.child(caption(&theme, "No notifications"))
            })
            .when(!self.items.is_empty(), |panel| {
                panel
                    .child(
                        div()
                            .id("notification-list")
                            .flex()
                            .flex_col()
                            .gap(Spacing::Small.rems())
                            .max_h(units(size::APPLET_HEIGHT))
                            .overflow_y_scroll()
                            .children(cards),
                    )
                    .child(
                        Button::new("notifications-clear")
                            .label("Clear all")
                            .variant(ButtonVariant::Outline)
                            .w_full(true)
                            .on_click(|_, _, _| {
                                NotificationStore::global().clear_all_notifications()
                            }),
                    )
            })
    }
}
