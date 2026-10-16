use gpui::{
    App, AppContext, Context, Entity, FocusHandle, FontWeight, Global, InteractiveElement,
    IntoElement, KeyDownEvent, ParentElement, Render, Styled, Task, Window, div,
    prelude::FluentBuilder, svg,
};
use services::{AppState, PolkitAuthRequest};
use ui::components::{Button, ButtonVariant, Input, InputState, caption, panel};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize};

use crate::shell::{Placement, Shell, Surface};

type Responder = tokio::sync::oneshot::Sender<Result<(), String>>;

pub struct PolkitAgent {
    request: Option<PolkitAuthRequest>,
    responder: Option<Responder>,
    password: InputState,
    error: Option<String>,
    authenticating: bool,
    focus: FocusHandle,
    authentication: Option<tokio::task::JoinHandle<()>>,
    response: Option<Task<()>>,
    _events: Task<()>,
}

struct GlobalPolkitAgent(Entity<PolkitAgent>);

impl Global for GlobalPolkitAgent {}

impl PolkitAgent {
    pub fn init(cx: &mut App) {
        let agent = cx.new(|cx| {
            let service = cx.global::<AppState>().polkit.clone();
            let events = cx.spawn(async move |this, cx| {
                loop {
                    service.wait_for_change().await;
                    if this
                        .update(cx, |agent: &mut PolkitAgent, cx| agent.process_requests(cx))
                        .is_err()
                    {
                        break;
                    }
                }
            });
            Self {
                request: None,
                responder: None,
                password: InputState::new("").with_masked(true).with_focused(true),
                error: None,
                authenticating: false,
                focus: cx.focus_handle(),
                authentication: None,
                response: None,
                _events: events,
            }
        });
        cx.set_global(GlobalPolkitAgent(agent));
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalPolkitAgent>().0.clone()
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn process_requests(&mut self, cx: &mut Context<Self>) {
        let service = cx.global::<AppState>().polkit.clone();
        while let Some(cookie) = service.pop_cancelled() {
            if self
                .request
                .as_ref()
                .is_some_and(|request| request.cookie == cookie)
            {
                self.finish(Err("Cancelled by Polkit Authority".into()));
            }
        }
        if self
            .responder
            .as_ref()
            .is_some_and(|responder| responder.is_closed())
        {
            self.finish(Err("Authentication request expired".into()));
        }
        if self.request.is_none() {
            while let Some((request, responder)) = service.pop_request() {
                if responder.is_closed() {
                    continue;
                }
                self.request = Some(request);
                self.responder = Some(responder);
                self.password.clear();
                self.error = None;
                break;
            }
        }
        if self.request.is_some() {
            Shell::open(Surface::Polkit, Placement::default(), cx);
        } else {
            Shell::close_surface(Surface::Polkit, cx);
        }
        cx.notify();
    }

    fn finish(&mut self, result: Result<(), String>) {
        if let Some(authentication) = self.authentication.take() {
            authentication.abort();
        }
        self.response = None;
        if let Some(responder) = self.responder.take() {
            let _ = responder.send(result);
        }
        self.request = None;
        self.password.clear();
        self.error = None;
        self.authenticating = false;
    }

    fn cancel(&mut self, cx: &mut Context<Self>) {
        self.finish(Err("Cancelled by user".into()));
        self.process_requests(cx);
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        if self.authenticating || self.password.is_empty() {
            return;
        }
        let Some(request) = &self.request else {
            return;
        };
        let user = request.user_name.clone();
        let cookie = request.cookie.clone();
        let response_cookie = cookie.clone();
        let password = self.password.text();
        self.password.clear();
        let (sender, receiver) = tokio::sync::oneshot::channel();
        self.authenticating = true;
        self.error = None;
        self.authentication = Some(services::spawn_tokio(async move {
            let result = tokio::time::timeout(
                std::time::Duration::from_secs(30),
                services::authenticate_user(&user, &cookie, &password),
            )
            .await
            .unwrap_or(Err("Authentication timed out".to_string()));
            let _ = sender.send(result);
        }));
        self.response = Some(cx.spawn(async move |this, cx| {
            let Ok(result) = receiver.await else {
                return;
            };
            let _ = this.update(cx, |agent, cx| {
                if agent
                    .request
                    .as_ref()
                    .is_none_or(|request| request.cookie != response_cookie)
                {
                    return;
                }
                agent.authenticating = false;
                agent.authentication = None;
                match result {
                    Ok(()) => {
                        agent.finish(Ok(()));
                        agent.process_requests(cx);
                    }
                    Err(error) => agent.error = Some(error),
                }
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "escape" => self.cancel(cx),
            "enter" => self.submit(cx),
            _ if self.authenticating => return,
            _ => {
                if !self.password.handle_key_down(event, cx) {
                    return;
                }
                self.error = None;
            }
        }
        cx.stop_propagation();
        cx.notify();
    }
}

impl Drop for PolkitAgent {
    fn drop(&mut self) {
        self.finish(Err("Authentication agent closed".into()));
    }
}

impl Render for PolkitAgent {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let message = self
            .request
            .as_ref()
            .map(|request| request.message.clone())
            .unwrap_or_default();
        let user = self
            .request
            .as_ref()
            .map(|request| request.user_name.clone())
            .unwrap_or_default();
        let placeholder = if self.authenticating {
            "Checking…".to_string()
        } else {
            format!("Password for {user}")
        };

        panel(&theme)
            .id("polkit")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::key_down))
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Small.rems())
                    .child(
                        svg()
                            .path("shield-check.svg")
                            .size(IconSize::Large.rems())
                            .text_color(theme.accent()),
                    )
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .child("Authentication required"),
                    ),
            )
            .child(caption(&theme, message).whitespace_normal())
            .child(
                Input::from_state("polkit-password", &self.password)
                    .placeholder(placeholder)
                    .masked(true)
                    .focused(!self.authenticating)
                    .disabled(self.authenticating)
                    .error(self.error.is_some())
                    .w_full(),
            )
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
                    .justify_end()
                    .gap(Spacing::Small.rems())
                    .child(
                        Button::new("polkit-cancel")
                            .label("Cancel")
                            .variant(ButtonVariant::Ghost)
                            .on_click(cx.listener(|agent, _, _, cx| agent.cancel(cx))),
                    )
                    .child(
                        Button::new("polkit-authenticate")
                            .label("Authenticate")
                            .disabled(self.authenticating || self.password.is_empty())
                            .on_click(cx.listener(|agent, _, _, cx| agent.submit(cx))),
                    ),
            )
    }
}
