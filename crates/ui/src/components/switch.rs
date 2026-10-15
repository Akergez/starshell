use crate::theme::{Theme, mix};
use crate::tokens::{motion, size, units};
use gpui::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    StatefulInteractiveElement, Styled, Window, div,
};

pub type ClickListener = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Switch {
    id: ElementId,
    checked: bool,
    disabled: bool,
    on_click: Option<ClickListener>,
}

impl Switch {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            checked: false,
            disabled: false,
            on_click: None,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn on_click<F>(mut self, listener: F) -> Self
    where
        F: Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    {
        self.on_click = Some(Box::new(listener));
        self
    }
}

impl RenderOnce for Switch {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let (track, knob) = if self.checked {
            (theme.accent(), theme.on_accent())
        } else {
            (theme.border_variant(), theme.text_muted())
        };
        let hover = mix(track, theme.text(), 0.1);
        let inset = (size::SWITCH_HEIGHT - size::SWITCH_KNOB) / 2.0;
        let travel = size::SWITCH_WIDTH - size::SWITCH_KNOB - inset * 2.0;

        let mut element = div()
            .id(self.id.clone())
            .flex()
            .flex_shrink_0()
            .items_center()
            .w(units(size::SWITCH_WIDTH))
            .h(units(size::SWITCH_HEIGHT))
            .px(units(inset))
            .rounded_full()
            .bg(track)
            .transitions(|transitions| transitions.bg(motion::fast_cubic()))
            .child(
                div()
                    .id((self.id, "knob"))
                    .size(units(size::SWITCH_KNOB))
                    .rounded_full()
                    .bg(knob)
                    .ml(units(if self.checked { travel } else { 0.0 }))
                    .transitions(|transitions| {
                        transitions
                            .ml(motion::fast_cubic())
                            .bg(motion::fast_cubic())
                    }),
            );

        if self.disabled {
            element = element.opacity(0.5).cursor_not_allowed();
        } else {
            element = element.cursor_pointer().hover(move |style| style.bg(hover));
            if let Some(on_click) = self.on_click {
                element = element.on_click(on_click);
            }
        }

        element
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_switch_builder() {
        let sw = Switch::new("my-switch").checked(true).disabled(false);
        assert!(sw.checked);
        assert!(!sw.disabled);

        let sw_disabled = Switch::new("disabled-sw").checked(false).disabled(true);
        assert!(!sw_disabled.checked);
        assert!(sw_disabled.disabled);
    }
}
