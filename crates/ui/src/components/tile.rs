use crate::components::button::ClickListener;
use crate::theme::{Theme, mix};
use crate::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};
use gpui::{
    App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, svg,
};

#[derive(IntoElement)]
pub struct Tile {
    id: ElementId,
    icon: SharedString,
    title: SharedString,
    status: SharedString,
    active: bool,
    expanded: bool,
    on_toggle: Option<ClickListener>,
    on_expand: Option<ClickListener>,
}

impl Tile {
    pub fn new(
        id: impl Into<ElementId>,
        icon: impl Into<SharedString>,
        title: impl Into<SharedString>,
    ) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            title: title.into(),
            status: SharedString::default(),
            active: false,
            expanded: false,
            on_toggle: None,
            on_expand: None,
        }
    }

    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = status.into();
        self
    }

    pub fn active(mut self, active: bool) -> Self {
        self.active = active;
        self
    }

    pub fn expanded(mut self, expanded: bool) -> Self {
        self.expanded = expanded;
        self
    }

    pub fn on_toggle(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_toggle = Some(Box::new(listener));
        self
    }

    pub fn on_expand(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_expand = Some(Box::new(listener));
        self
    }
}

pub struct TileColors {
    pub background: Hsla,
    pub border: Hsla,
    pub icon: Hsla,
    pub chevron: Hsla,
}

pub fn tile_colors(theme: &Theme, active: bool, expanded: bool) -> TileColors {
    TileColors {
        background: if active {
            theme.element_selected()
        } else {
            theme.elevated()
        },
        border: if expanded {
            theme.accent()
        } else {
            gpui::transparent_black()
        },
        icon: if active {
            theme.accent()
        } else {
            theme.text_muted()
        },
        chevron: if expanded {
            theme.accent()
        } else {
            theme.text_muted()
        },
    }
}

fn zone(id: ElementId, base: Hsla, interactive: bool, theme: &Theme) -> gpui::Stateful<gpui::Div> {
    let hover = mix(base, theme.text(), 0.06);
    let active = mix(base, theme.text(), 0.12);
    div()
        .id(id)
        .flex()
        .items_center()
        .bg(gpui::transparent_black())
        .transitions(|transitions| transitions.bg(motion::fast_cubic()))
        .when(interactive, |zone| {
            zone.cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(active))
        })
}

impl RenderOnce for Tile {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let colors = tile_colors(theme, self.active, self.expanded);
        let has_chevron = self.on_expand.is_some() && self.on_toggle.is_some();
        let (body_action, chevron_action) = if has_chevron {
            (self.on_toggle, self.on_expand)
        } else {
            (self.on_toggle.or(self.on_expand), None)
        };
        let inner_radius = units(Radius::Large.value() - Spacing::Hairline.value());

        let body = zone(
            (self.id.clone(), "body").into(),
            colors.background,
            body_action.is_some(),
            theme,
        )
        .flex_1()
        .min_w_0()
        .gap(Spacing::Medium.rems())
        .px(Spacing::Large.rems())
        .py(Spacing::Medium.rems())
        .rounded_l(inner_radius)
        .when(!has_chevron, |body| body.rounded_r(inner_radius))
        .child(
            svg()
                .path(self.icon)
                .flex_shrink_0()
                .size(IconSize::Large.rems())
                .text_color(colors.icon),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .min_w_0()
                .child(
                    div()
                        .truncate()
                        .text_size(TextSize::Default.pixels(theme.font_size))
                        .text_color(theme.text())
                        .child(self.title),
                )
                .child(
                    div()
                        .truncate()
                        .text_size(TextSize::Small.pixels(theme.font_size))
                        .text_color(theme.text_muted())
                        .child(self.status),
                ),
        )
        .when_some(body_action, |body, action| body.on_click(action));

        div()
            .id(self.id.clone())
            .flex()
            .flex_1()
            .flex_basis(gpui::relative(0.0))
            .min_w_0()
            .min_h(units(size::TILE))
            .overflow_hidden()
            .rounded(Radius::Large.rems())
            .border_1()
            .border_color(colors.border)
            .bg(colors.background)
            .transitions(|transitions| {
                transitions
                    .bg(motion::fast_cubic())
                    .border_color(motion::fast_cubic())
            })
            .child(body)
            .when_some(chevron_action, |tile, action| {
                tile.child(
                    div()
                        .flex_shrink_0()
                        .w(Spacing::Hairline.rems())
                        .bg(theme.border_variant()),
                )
                .child(
                    zone(
                        (self.id.clone(), "chevron").into(),
                        colors.background,
                        true,
                        theme,
                    )
                    .flex_shrink_0()
                    .w(units(size::CONTROL))
                    .justify_center()
                    .rounded_r(inner_radius)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_click(action)
                    .child(
                        svg()
                            .path(if self.expanded {
                                "chevron-up.svg"
                            } else {
                                "chevron-down.svg"
                            })
                            .size(IconSize::Small.rems())
                            .text_color(colors.chevron),
                    ),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_states_map_to_palette_roles() {
        let theme = Theme::default();
        let idle = tile_colors(&theme, false, false);
        assert_eq!(idle.background, theme.elevated());
        assert_eq!(idle.icon, theme.text_muted());
        assert_eq!(idle.border, gpui::transparent_black());
        let active = tile_colors(&theme, true, false);
        assert_eq!(active.background, theme.element_selected());
        assert_eq!(active.icon, theme.accent());
        let expanded = tile_colors(&theme, false, true);
        assert_eq!(expanded.border, theme.accent());
        assert_eq!(expanded.chevron, theme.accent());
    }
}
