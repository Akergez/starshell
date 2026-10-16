use crate::theme::Theme;
use crate::tokens::{Radius, Spacing, TextSize};
use gpui::{
    AnyView, App, AppContext, ColorExt, Context, IntoElement, ParentElement, Render, SharedString,
    Styled, Window, div, prelude::FluentBuilder,
};

pub struct TooltipView {
    pub text: SharedString,
    pub shortcut: Option<SharedString>,
}

impl Render for TooltipView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        div()
            .px(Spacing::Medium.rems())
            .py(Spacing::XSmall.rems())
            .rounded(Radius::Medium.rems())
            .bg(theme.elevated())
            .border_1()
            .border_color(theme.elevated().opacity(0.4))
            .text_size(TextSize::XSmall.pixels(theme.font_size))
            .text_color(theme.text())
            .font_family(theme.font_family())
            .flex()
            .items_center()
            .gap(Spacing::Small.rems())
            .child(self.text.clone())
            .when_some(self.shortcut.clone(), |el, sc| {
                el.child(
                    div()
                        .px(Spacing::XSmall.rems())
                        .py(Spacing::Hairline.rems())
                        .rounded(Radius::Small.rems())
                        .bg(theme.background())
                        .text_color(theme.text_muted())
                        .text_size(TextSize::XSmall.pixels(theme.font_size))
                        .child(sc),
                )
            })
    }
}

pub fn tooltip(
    text: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| {
        let text = text.clone();
        cx.new(|_| TooltipView {
            text,
            shortcut: None,
        })
        .into()
    }
}

pub fn tooltip_with_shortcut(
    text: impl Into<SharedString>,
    shortcut: impl Into<SharedString>,
) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    let shortcut = shortcut.into();
    move |_, cx| {
        let text = text.clone();
        let shortcut = shortcut.clone();
        cx.new(|_| TooltipView {
            text,
            shortcut: Some(shortcut),
        })
        .into()
    }
}
