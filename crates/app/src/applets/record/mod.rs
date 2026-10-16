mod worker;

use gpui::{
    Context, FocusHandle, FontWeight, InteractiveElement, IntoElement, KeyDownEvent, ParentElement,
    Render, Styled, Task, Window, div, prelude::FluentBuilder, svg,
};
use services::{AppState, RecordStatus};
use ui::components::{Button, ButtonVariant, caption, panel};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize};
use worker::Request;

use crate::panel::SettingsPanel;
use crate::shell::Shell;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum PendingAction {
    Start,
    Pause,
    Resume,
    Stop,
}

pub struct RecordApplet {
    status: RecordStatus,
    seconds: u64,
    pending: Option<PendingAction>,
    error: Option<String>,
    focus: FocusHandle,
    sender: tokio::sync::mpsc::UnboundedSender<Request>,
    worker: tokio::task::JoinHandle<()>,
    _updates: Task<()>,
}

pub fn duration_label(seconds: u64) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        seconds / 3600,
        (seconds / 60) % 60,
        seconds % 60
    )
}

impl RecordApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let service = cx.global::<AppState>().record.clone();
        let status = service.get_status();
        let (sender, mut receiver, worker) = {
            let _guard = services::tokio_handle().enter();
            worker::start(service)
        };
        let updates = cx.spawn(async move |this, cx| {
            while let Some(reply) = receiver.recv().await {
                let updated = this.update(cx, |applet: &mut RecordApplet, cx| {
                    applet.status = reply.status;
                    applet.seconds = reply.seconds;
                    if reply.finished.is_some() && reply.finished == applet.pending {
                        applet.pending = None;
                        applet.error = reply.error;
                    }
                    if reply.stopped.is_some() {
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
            status,
            seconds: 0,
            pending: None,
            error: None,
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
        let action = match request {
            Request::Start(_) => PendingAction::Start,
            Request::Pause => PendingAction::Pause,
            Request::Resume => PendingAction::Resume,
            Request::Stop(_) => PendingAction::Stop,
            Request::Refresh | Request::Active(_) => return,
        };
        if self.pending.is_some()
            && !(self.pending == Some(PendingAction::Start) && action == PendingAction::Stop)
        {
            return;
        }
        self.error = None;
        self.pending = Some(action);
        if self.sender.send(request).is_err() {
            self.pending = None;
            self.error = Some("Could not control the recorder".to_string());
        }
        cx.notify();
    }

    fn primary(&mut self, cx: &mut Context<Self>) {
        let request = match self.status {
            RecordStatus::Stopped => {
                Request::Start(cx.global::<AppState>().config.get().record.clone())
            }
            RecordStatus::Recording => Request::Pause,
            RecordStatus::Paused => Request::Resume,
        };
        self.request(request, cx);
    }

    fn stop(&mut self, cx: &mut Context<Self>) {
        if self.status != RecordStatus::Stopped || self.pending == Some(PendingAction::Start) {
            self.request(Request::Stop(0), cx);
        }
    }

    fn key_down(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        match event.keystroke.key.as_str() {
            "enter" | "space" => self.primary(cx),
            "s" if event.keystroke.modifiers.control => self.stop(cx),
            _ => return,
        }
        cx.stop_propagation();
    }
}

impl Drop for RecordApplet {
    fn drop(&mut self) {
        let _ = self.sender.send(Request::Active(false));
        self.worker.abort();
    }
}

impl Render for RecordApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let config = cx.global::<AppState>().config.get().record.clone();
        let recording = self.status != RecordStatus::Stopped;
        let (label, icon) = match (self.status, self.pending) {
            (_, Some(PendingAction::Start)) => ("Starting…", "loader-circle.svg"),
            (_, Some(PendingAction::Stop)) => ("Stopping…", "loader-circle.svg"),
            (RecordStatus::Stopped, _) => ("Start recording", "video.svg"),
            (RecordStatus::Recording, _) => ("Pause", "pause.svg"),
            (RecordStatus::Paused, _) => ("Resume", "play.svg"),
        };
        let state = match self.status {
            RecordStatus::Stopped => "Ready",
            RecordStatus::Recording => "Recording",
            RecordStatus::Paused => "Paused",
        };

        panel(&theme)
            .id("record")
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
                            .path("video.svg")
                            .size(IconSize::Large.rems())
                            .text_color(if recording {
                                theme.error()
                            } else {
                                theme.text_muted()
                            }),
                    )
                    .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(state))
                    .child(
                        div()
                            .font_weight(FontWeight::MEDIUM)
                            .text_size(TextSize::Large.pixels(theme.font_size))
                            .text_color(if recording {
                                theme.text()
                            } else {
                                theme.text_muted()
                            })
                            .child(duration_label(self.seconds)),
                    ),
            )
            .child(caption(
                &theme,
                format!(
                    "{} · {} fps · {} · audio: {}",
                    config.output, config.fps, config.container, config.audio
                ),
            ))
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
                    .gap(Spacing::Small.rems())
                    .child(
                        div().flex_1().child(
                            Button::new("record-primary")
                                .label(label)
                                .icon(icon)
                                .w_full(true)
                                .disabled(self.pending.is_some())
                                .on_click(cx.listener(|applet, _, _, cx| applet.primary(cx))),
                        ),
                    )
                    .when(
                        recording || self.pending == Some(PendingAction::Start),
                        |row| {
                            row.child(
                                div().flex_1().child(
                                    Button::new("record-stop")
                                        .label("Stop")
                                        .icon("square.svg")
                                        .variant(ButtonVariant::Danger)
                                        .w_full(true)
                                        .on_click(cx.listener(|applet, _, _, cx| applet.stop(cx))),
                                ),
                            )
                        },
                    ),
            )
            .child(
                Button::new("record-settings")
                    .label("Recording settings")
                    .icon("settings.svg")
                    .variant(ButtonVariant::Ghost)
                    .w_full(true)
                    .on_click(|_, _, cx| {
                        Shell::close(cx);
                        SettingsPanel::open(cx);
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn durations_show_hours_minutes_and_seconds() {
        assert_eq!(duration_label(0), "00:00:00");
        assert_eq!(duration_label(3725), "01:02:05");
    }
}
