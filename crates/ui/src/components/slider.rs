use std::rc::Rc;

use crate::theme::Theme;
use crate::tokens::{motion, size, units};
use gpui::{
    App, Bounds, DispatchPhase, ElementId, Hitbox, HitboxBehavior, Hsla, InteractiveElement,
    IntoElement, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, ParentElement, Pixels,
    RenderOnce, Styled, Window, canvas, div, prelude::FluentBuilder, relative,
};

pub type ValueListener = Rc<dyn Fn(f32, &mut Window, &mut App) + 'static>;

#[derive(IntoElement)]
pub struct Slider {
    id: ElementId,
    value: f32,
    disabled: bool,
    color: Option<Hsla>,
    on_change: Option<ValueListener>,
    on_release: Option<ValueListener>,
}

impl Slider {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            value: 0.0,
            disabled: false,
            color: None,
            on_change: None,
            on_release: None,
        }
    }

    pub fn value(mut self, value: f32) -> Self {
        self.value = value.clamp(0.0, 1.0);
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn color(mut self, color: Hsla) -> Self {
        self.color = Some(color);
        self
    }

    pub fn on_change(mut self, listener: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(listener));
        self
    }

    pub fn on_release(mut self, listener: impl Fn(f32, &mut Window, &mut App) + 'static) -> Self {
        self.on_release = Some(Rc::new(listener));
        self
    }
}

pub fn fraction_at(bounds: Bounds<Pixels>, x: Pixels) -> f32 {
    let width = f32::from(bounds.size.width);
    if width <= 0.0 {
        return 0.0;
    }
    (f32::from(x - bounds.origin.x) / width).clamp(0.0, 1.0)
}

impl RenderOnce for Slider {
    fn render(self, window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let fill_color = self.color.unwrap_or_else(|| theme.accent());
        let track_color = theme.border_variant();
        let track_top = (size::SLIDER_HITBOX - size::SLIDER_TRACK) / 2.0;
        let thumb_top = (size::SLIDER_HITBOX - size::SLIDER_THUMB) / 2.0;
        let dragging = window.use_keyed_state(
            ElementId::from((self.id.clone(), "dragging")),
            cx,
            |_, _| false,
        );
        let interactive = !self.disabled;
        let on_change = self.on_change;
        let on_release = self.on_release;

        div()
            .id(self.id.clone())
            .relative()
            .w_full()
            .min_w_0()
            .h(units(size::SLIDER_HITBOX))
            .when(interactive, |element| element.cursor_pointer())
            .when(!interactive, |element| element.opacity(0.5))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(units(track_top))
                    .h(units(size::SLIDER_TRACK))
                    .rounded_full()
                    .bg(track_color),
            )
            .child(
                div()
                    .id((self.id.clone(), "fill"))
                    .absolute()
                    .left_0()
                    .top(units(track_top))
                    .h(units(size::SLIDER_TRACK))
                    .w(relative(self.value))
                    .rounded_full()
                    .bg(fill_color)
                    .transitions(|transitions| transitions.w(motion::fast_cubic())),
            )
            .child(
                div()
                    .id((self.id.clone(), "thumb"))
                    .absolute()
                    .top(units(thumb_top))
                    .left(relative(self.value))
                    .w_0()
                    .transitions(|transitions| transitions.left(motion::fast_cubic()))
                    .child(
                        div()
                            .ml(-units(size::SLIDER_THUMB / 2.0))
                            .size(units(size::SLIDER_THUMB))
                            .rounded_full()
                            .bg(fill_color),
                    ),
            )
            .when(interactive, |element| {
                element.child(
                    canvas(
                        |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
                        move |bounds, hitbox: Hitbox, window, _| {
                            register_drag(bounds, hitbox, dragging, on_change, on_release, window);
                        },
                    )
                    .absolute()
                    .size_full(),
                )
            })
    }
}

fn register_drag(
    bounds: Bounds<Pixels>,
    hitbox: Hitbox,
    dragging: gpui::Entity<bool>,
    on_change: Option<ValueListener>,
    on_release: Option<ValueListener>,
    window: &mut Window,
) {
    window.on_mouse_event({
        let dragging = dragging.clone();
        let on_change = on_change.clone();
        move |event: &MouseDownEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.button != MouseButton::Left
                || !hitbox.is_hovered(window)
            {
                return;
            }
            dragging.update(cx, |dragging, _| *dragging = true);
            if let Some(on_change) = &on_change {
                on_change(fraction_at(bounds, event.position.x), window, cx);
            }
            cx.stop_propagation();
        }
    });
    window.on_mouse_event({
        let dragging = dragging.clone();
        move |event: &MouseMoveEvent, phase, window, cx| {
            if phase != DispatchPhase::Bubble
                || event.pressed_button != Some(MouseButton::Left)
                || !*dragging.read(cx)
            {
                return;
            }
            if let Some(on_change) = &on_change {
                on_change(fraction_at(bounds, event.position.x), window, cx);
            }
        }
    });
    window.on_mouse_event(move |event: &MouseUpEvent, phase, window, cx| {
        if phase != DispatchPhase::Bubble
            || event.button != MouseButton::Left
            || !*dragging.read(cx)
        {
            return;
        }
        dragging.update(cx, |dragging, _| *dragging = false);
        if let Some(on_release) = &on_release {
            on_release(fraction_at(bounds, event.position.x), window, cx);
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{point, px};

    #[test]
    fn value_is_clamped() {
        assert_eq!(Slider::new("slider").value(1.5).value, 1.0);
        assert_eq!(Slider::new("slider").value(-0.5).value, 0.0);
    }

    #[test]
    fn fraction_follows_the_pointer_and_stays_in_range() {
        let bounds = Bounds::new(point(px(10.0), px(0.0)), gpui::size(px(100.0), px(16.0)));
        assert_eq!(fraction_at(bounds, px(60.0)), 0.5);
        assert_eq!(fraction_at(bounds, px(0.0)), 0.0);
        assert_eq!(fraction_at(bounds, px(500.0)), 1.0);
    }
}
