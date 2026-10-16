mod header;
mod panels;
mod tiles;

use gpui::{
    AnyElement, Context, Entity, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent,
    ParentElement, Render, Styled, Subscription, Window, div, prelude::FluentBuilder,
};
use ui::components::{InputState, card, divider, panel, reveal};
use ui::theme::Theme;
use ui::tokens::Spacing;

use super::common;
use crate::shell::state::{ShellState, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expanded {
    Wifi,
    Bluetooth,
    Vpn,
    Performance,
}

pub struct PasswordPrompt {
    pub ssid: String,
    pub input: InputState,
}

pub struct ControlCenter {
    focus: FocusHandle,
    state: Entity<ShellState>,
    expanded: Option<Expanded>,
    power_open: bool,
    password: Option<PasswordPrompt>,
    _observe: Subscription,
}

impl ControlCenter {
    pub fn new(_window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            focus: cx.focus_handle(),
            state,
            expanded: None,
            power_open: false,
            password: None,
            _observe: observe,
        }
    }

    pub fn focus_handle(&self) -> FocusHandle {
        self.focus.clone()
    }

    fn toggle_expanded(&mut self, expanded: Expanded, cx: &mut Context<Self>) {
        self.expanded = if self.expanded == Some(expanded) {
            None
        } else {
            Some(expanded)
        };
        self.password = None;
        cx.notify();
    }

    fn toggle_power(&mut self, cx: &mut Context<Self>) {
        self.power_open = !self.power_open;
        cx.notify();
    }

    fn handle_key(&mut self, event: &KeyDownEvent, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(prompt) = self.password.as_mut() else {
            return;
        };
        match event.keystroke.key.as_str() {
            "escape" => {
                self.password = None;
            }
            "enter" => self.submit_password(cx),
            _ => {
                prompt.input.handle_key_down(event, cx);
            }
        }
        cx.stop_propagation();
        cx.notify();
    }

    fn submit_password(&mut self, cx: &mut Context<Self>) {
        let Some(prompt) = self.password.take() else {
            return;
        };
        let password = prompt.input.text();
        if !password.is_empty() {
            cx.global::<services::AppState>()
                .network
                .connect_wifi(&prompt.ssid, Some(&password));
        }
        cx.notify();
    }

    fn row(
        &mut self,
        pair: [tiles::TileKind; 2],
        snapshot: &Snapshot,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let tiles = pair.map(|kind| tiles::tile(kind, snapshot, self.expanded, cx));
        let mut elements = vec![
            div()
                .flex()
                .w_full()
                .gap(Spacing::Medium.rems())
                .children(tiles)
                .into_any_element(),
        ];
        if let Some(expanded) = self.expanded
            && pair.iter().any(|kind| kind.panel() == Some(expanded))
        {
            let content = panels::render(expanded, self, snapshot, theme, cx);
            elements.push(
                reveal(
                    gpui::ElementId::Name(format!("panel-{expanded:?}").into()),
                    card(theme).w_full().child(content),
                )
                .into_any_element(),
            );
        }
        elements
    }
}

impl Render for ControlCenter {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let snapshot = self.state.read(cx).snapshot.clone();
        let mut grid = Vec::new();
        for pair in tiles::ROWS {
            grid.extend(self.row(pair, &snapshot, &theme, cx));
        }
        let power_open = self.power_open;

        panel(&theme)
            .id("control-center")
            .track_focus(&self.focus)
            .on_key_down(cx.listener(Self::handle_key))
            .w_full()
            .child(header::render(&snapshot, &theme, power_open, cx))
            .when(power_open, |panel| {
                panel.child(reveal("power-actions", header::power_actions(&theme, cx)))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::Medium.rems())
                    .children(grid),
            )
            .child(divider(&theme))
            .child(common::volume_row(&theme, &snapshot.audio))
            .children(
                snapshot
                    .displays
                    .iter()
                    .map(|display| common::display_row(&theme, display)),
            )
    }
}
