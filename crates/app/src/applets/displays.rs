use gpui::{
    Context, ElementId, Entity, Global, IntoElement, ParentElement, Render, Styled, Subscription,
    Window, div, prelude::FluentBuilder, svg,
};
use services::AppState;
use ui::components::{Slider, SliderRow, Switch, caption, card, panel};
use ui::theme::Theme;
use ui::tokens::{IconSize, Spacing, TextSize};

use crate::shell::state::ShellState;

#[derive(Default)]
struct LinkedBrightness(bool);

impl Global for LinkedBrightness {}

pub struct DisplaysApplet {
    state: Entity<ShellState>,
    _observe: Subscription,
}

impl DisplaysApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        if !cx.has_global::<LinkedBrightness>() {
            cx.set_global(LinkedBrightness::default());
        }
        Self {
            state,
            _observe: observe,
        }
    }
}

impl Render for DisplaysApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let displays = self.state.read(cx).snapshot.displays.clone();
        let linked = cx.global::<LinkedBrightness>().0;
        let small = TextSize::Small.pixels(theme.font_size);

        panel(&theme)
            .w_full()
            .when(displays.is_empty(), |panel| {
                panel.child(caption(&theme, "No display supports brightness control"))
            })
            .children(displays.iter().enumerate().map(|(index, display)| {
                let id = display.id.clone();
                card(&theme)
                    .w_full()
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .gap(Spacing::Small.rems())
                            .child(
                                svg()
                                    .path("monitor.svg")
                                    .flex_shrink_0()
                                    .size(IconSize::Medium.rems())
                                    .text_color(theme.text_muted()),
                            )
                            .child(
                                div()
                                    .flex()
                                    .flex_col()
                                    .flex_1()
                                    .min_w_0()
                                    .child(div().truncate().child(display.name.clone()))
                                    .child(
                                        div()
                                            .truncate()
                                            .text_size(small)
                                            .text_color(theme.text_muted())
                                            .child(display.connector.clone()),
                                    ),
                            )
                            .child(
                                div()
                                    .flex_shrink_0()
                                    .text_size(small)
                                    .text_color(theme.text_muted())
                                    .child(display.backend.label()),
                            ),
                    )
                    .child(SliderRow::new(
                        SliderRow::icon(&theme, "sun.svg"),
                        "Brightness",
                        Slider::new(ElementId::named_usize("display-brightness", index)).on_change(
                            move |fraction, _, cx| {
                                let percent = (fraction * 100.0).round() as u32;
                                let displays = &cx.global::<AppState>().displays;
                                if cx.global::<LinkedBrightness>().0 {
                                    displays.set_all(percent);
                                } else {
                                    displays.set_brightness(&id, percent);
                                }
                                ShellState::refresh_now(cx);
                            },
                        ),
                        display.brightness.min(100) as f32 / 100.0,
                    ))
            }))
            .when(displays.len() > 1, |panel| {
                panel.child(
                    div()
                        .flex()
                        .items_center()
                        .justify_between()
                        .gap(Spacing::Medium.rems())
                        .child("Change brightness together")
                        .child(Switch::new("displays-linked").checked(linked).on_click(
                            |_, _, cx| {
                                let linked = &mut cx.global_mut::<LinkedBrightness>().0;
                                *linked = !*linked;
                                cx.refresh_windows();
                            },
                        )),
                )
            })
    }
}
