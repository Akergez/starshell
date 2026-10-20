use gpui::{
    AnyView, Context, FocusHandle, InteractiveElement, IntoElement, KeyDownEvent, MouseButton,
    ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, px,
};
use services::BarEdge;
use ui::components::fade_in;
use ui::theme::Theme;
use ui::tokens::{Spacing, size, units_to_pixels};

use super::{Shell, Surface};
use crate::applets;

pub struct Content {
    pub view: AnyView,
    pub focus: Option<FocusHandle>,
}

pub struct Overlay {
    surface: Surface,
    anchor: Option<Pixels>,
    content: Content,
    focus: FocusHandle,
    generation: usize,
}

impl Overlay {
    pub fn new(
        surface: Surface,
        anchor: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let content = applets::build(surface, window, cx);
        let focus = cx.focus_handle();
        let overlay = Self {
            surface,
            anchor,
            content,
            focus,
            generation: 0,
        };
        overlay.request_focus(window, cx);
        overlay
    }

    pub fn show(
        &mut self,
        surface: Surface,
        anchor: Option<Pixels>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.surface != surface {
            self.content = applets::build(surface, window, cx);
            self.generation += 1;
        }
        self.surface = surface;
        self.anchor = anchor;
        self.request_focus(window, cx);
        cx.notify();
    }

    pub fn anchor(&self) -> Option<Pixels> {
        self.anchor
    }

    fn request_focus(&self, window: &mut Window, cx: &mut Context<Self>) {
        let focus = self.content.focus.as_ref().unwrap_or(&self.focus);
        window.focus(focus, cx);
    }

    fn width(&self, font_size: f32) -> Pixels {
        let units = match self.surface {
            Surface::Applet(kind) => kind.width(),
            Surface::Polkit => size::PANEL,
        };
        units_to_pixels(units, font_size)
    }
}

pub fn horizontal_offset(
    anchor: Option<Pixels>,
    width: Pixels,
    viewport: Pixels,
    inset: Pixels,
) -> Pixels {
    let max = (viewport - width - inset).max(inset);
    match anchor {
        Some(anchor) => (anchor - width / 2.0).clamp(inset, max),
        None => max,
    }
}

impl Render for Overlay {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let font_size = theme.font_size;
        let viewport = window.viewport_size();
        let bar = cx.global::<services::AppState>().config.get().bar.clone();
        let hidden_bar = if bar.autohide {
            units_to_pixels(bar.clamped_height() - size::BAR_HIDDEN_STRIP, font_size)
        } else {
            px(0.0)
        };
        let gap = Spacing::Medium.pixels(font_size);
        let applet_gap = gap + hidden_bar;
        let inset = Spacing::XLarge.pixels(font_size);
        let width = self.width(font_size);
        let bottom = Shell::bar_position(cx) == BarEdge::Bottom;
        let max_height = (viewport.height - gap - hidden_bar - inset).max(px(0.0));
        let content = fade_in(
            ("overlay-content", self.generation),
            div()
                .id("overlay-scroll")
                .w(width)
                .max_h(max_height)
                .overflow_y_scroll()
                .child(self.content.view.clone()),
        );

        let positioned = div()
            .absolute()
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
            .child(content);
        let positioned = match self.surface {
            Surface::Polkit => positioned
                .top((viewport.height / 3.0).max(gap))
                .left(((viewport.width - width) / 2.0).max(px(0.0))),
            Surface::Applet(kind) => positioned
                .left(horizontal_offset(
                    self.anchor.or_else(|| kind.default_anchor(viewport.width)),
                    width,
                    viewport.width,
                    inset,
                ))
                .when(bottom, |element| element.bottom(applet_gap))
                .when(!bottom, |element| element.top(applet_gap)),
        };

        div()
            .id("overlay")
            .track_focus(&self.focus)
            .size_full()
            .font_family(theme.font_family())
            .text_size(theme.font_size())
            .text_color(theme.text())
            .on_key_down(cx.listener(|overlay, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" && overlay.surface != Surface::Polkit {
                    Shell::close(cx);
                    cx.stop_propagation();
                }
            }))
            .when(self.surface != Surface::Polkit, |element| {
                element
                    .on_mouse_down(MouseButton::Left, |_, _, cx| Shell::close(cx))
                    .on_mouse_down(MouseButton::Right, |_, _, cx| Shell::close(cx))
            })
            .child(positioned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applets_follow_the_click_and_stay_on_screen() {
        let inset = px(16.0);
        assert_eq!(
            horizontal_offset(Some(px(500.0)), px(200.0), px(1000.0), inset),
            px(400.0)
        );
        assert_eq!(
            horizontal_offset(Some(px(10.0)), px(200.0), px(1000.0), inset),
            inset
        );
        assert_eq!(
            horizontal_offset(Some(px(990.0)), px(200.0), px(1000.0), inset),
            px(784.0)
        );
        assert_eq!(
            horizontal_offset(None, px(200.0), px(1000.0), inset),
            px(784.0)
        );
    }
}
