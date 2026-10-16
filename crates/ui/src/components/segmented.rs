use std::rc::Rc;

use crate::theme::Theme;
use crate::tokens::{Radius, Spacing, TextSize, motion, size, units};
use gpui::{
    App, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder,
};

pub type SelectListener = Rc<dyn Fn(usize, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SharedString>,
    selected: Option<usize>,
    disabled: bool,
    on_select: Option<SelectListener>,
}

impl SegmentedControl {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            options: Vec::new(),
            selected: None,
            disabled: false,
            on_select: None,
        }
    }

    pub fn option(mut self, label: impl Into<SharedString>) -> Self {
        self.options.push(label.into());
        self
    }

    pub fn selected(mut self, index: Option<usize>) -> Self {
        self.selected = index;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_select(mut self, listener: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_select = Some(Rc::new(listener));
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let hover = theme.ghost_hover();
        let active = theme.ghost_active();
        let inner_radius = units(Radius::Medium.value() - Spacing::XXSmall.value());
        let segments = self.options.into_iter().enumerate().map(|(index, label)| {
            let selected = self.selected == Some(index);
            let on_select = self.on_select.clone();
            div()
                .id(ElementId::NamedChild(
                    std::sync::Arc::new(self.id.clone()),
                    index.to_string().into(),
                ))
                .flex_1()
                .min_w_0()
                .flex()
                .items_center()
                .justify_center()
                .h(units(size::CONTROL - Spacing::XXSmall.value() * 2.0))
                .px(Spacing::Small.rems())
                .rounded(inner_radius)
                .text_size(TextSize::Small.pixels(theme.font_size))
                .font_weight(FontWeight::MEDIUM)
                .bg(if selected {
                    theme.element_selected()
                } else {
                    gpui::transparent_black()
                })
                .text_color(if selected {
                    theme.accent()
                } else {
                    theme.text_muted()
                })
                .transitions(|transitions| {
                    transitions
                        .bg(motion::fast_cubic())
                        .text_color(motion::fast_cubic())
                })
                .child(div().truncate().child(label))
                .when(!self.disabled && !selected, |segment| {
                    segment
                        .cursor_pointer()
                        .hover(move |style| style.bg(hover))
                        .active(move |style| style.bg(active))
                })
                .when_some(
                    on_select.filter(|_| !self.disabled),
                    |segment, on_select| {
                        segment.on_click(move |_, window, cx| on_select(index, window, cx))
                    },
                )
        });

        div()
            .id(self.id.clone())
            .flex()
            .w_full()
            .gap(Spacing::XXSmall.rems())
            .p(Spacing::XXSmall.rems())
            .rounded(Radius::Medium.rems())
            .bg(theme.background())
            .when(self.disabled, |control| control.opacity(0.5))
            .children(segments)
    }
}
