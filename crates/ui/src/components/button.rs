use crate::theme::{Theme, mix};
use crate::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};
use gpui::{
    App, ClickEvent, ColorExt, ElementId, FontWeight, Hsla, InteractiveElement, IntoElement,
    ParentElement, Pixels, Rems, RenderOnce, SharedString, StatefulInteractiveElement, Styled,
    Window, div, px, svg,
};

pub type ClickListener = Box<dyn Fn(&ClickEvent, &mut Window, &mut App) + 'static>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Outline,
    Ghost,
    Danger,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ButtonSize {
    Sm,
    #[default]
    Md,
    Lg,
}

impl ButtonSize {
    pub fn height(self) -> f32 {
        match self {
            Self::Sm => 24.0,
            Self::Md => 28.0,
            Self::Lg => size::CONTROL,
        }
    }

    pub fn text_size(self) -> TextSize {
        match self {
            Self::Sm | Self::Md => TextSize::Small,
            Self::Lg => TextSize::Default,
        }
    }

    pub fn icon_size(self) -> IconSize {
        match self {
            Self::Sm => IconSize::XSmall,
            Self::Md => IconSize::Small,
            Self::Lg => IconSize::Medium,
        }
    }

    pub fn padding_x(self) -> Spacing {
        match self {
            Self::Sm => Spacing::Medium,
            Self::Md | Self::Lg => Spacing::Large,
        }
    }
}

struct Colors {
    background: Hsla,
    hover: Hsla,
    active: Hsla,
    foreground: Hsla,
    border: Hsla,
}

fn colors(theme: &Theme, variant: ButtonVariant, selected: bool) -> Colors {
    let transparent = gpui::transparent_black();
    let ghost = |foreground: Hsla| Colors {
        background: transparent,
        hover: theme.ghost_hover(),
        active: theme.ghost_active(),
        foreground,
        border: transparent,
    };
    if selected {
        let background = theme.element_selected();
        return Colors {
            background,
            hover: mix(background, theme.text(), 0.06),
            active: mix(background, theme.text(), 0.12),
            foreground: theme.accent(),
            border: transparent,
        };
    }
    match variant {
        ButtonVariant::Primary => {
            let background = theme.accent();
            Colors {
                background,
                hover: mix(background, theme.text(), 0.15),
                active: mix(background, theme.background(), 0.15),
                foreground: theme.on_accent(),
                border: transparent,
            }
        }
        ButtonVariant::Secondary => {
            let background = theme.elevated();
            Colors {
                background,
                hover: theme.ghost_hover(),
                active: theme.ghost_active(),
                foreground: theme.text(),
                border: transparent,
            }
        }
        ButtonVariant::Outline => Colors {
            border: theme.border_variant(),
            ..ghost(theme.text())
        },
        ButtonVariant::Ghost => ghost(theme.text()),
        ButtonVariant::Danger => Colors {
            background: theme.error().opacity(0.12),
            hover: theme.error().opacity(0.2),
            active: theme.error().opacity(0.28),
            foreground: theme.error(),
            border: transparent,
        },
    }
}

#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: Option<SharedString>,
    icon: Option<SharedString>,
    right_icon: Option<SharedString>,
    variant: ButtonVariant,
    size: ButtonSize,
    pill: bool,
    selected: bool,
    disabled: bool,
    w_full: bool,
    height_px: Option<f32>,
    padding_x: Option<f32>,
    tooltip: Option<SharedString>,
    on_click: Option<ClickListener>,
}

impl Button {
    pub fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            label: None,
            icon: None,
            right_icon: None,
            variant: ButtonVariant::Primary,
            size: ButtonSize::Md,
            pill: false,
            selected: false,
            disabled: false,
            w_full: false,
            height_px: None,
            padding_x: None,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn height_px(mut self, height: f32) -> Self {
        self.height_px = Some(height);
        self
    }

    pub fn padding_x(mut self, padding: f32) -> Self {
        self.padding_x = Some(padding);
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
        self
    }

    pub fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn right_icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.right_icon = Some(icon.into());
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn pill(mut self, pill: bool) -> Self {
        self.pill = pill;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn w_full(mut self, w_full: bool) -> Self {
        self.w_full = w_full;
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

impl RenderOnce for Button {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let colors = colors(theme, self.variant, self.selected);
        let height: gpui::AbsoluteLength = self
            .height_px
            .map(|height| px(height).into())
            .unwrap_or_else(|| units(self.size.height()).into());
        let padding_x: gpui::AbsoluteLength = self
            .padding_x
            .map(|padding| px(padding).into())
            .unwrap_or_else(|| self.size.padding_x().rems().into());
        let icon_size = self.size.icon_size().rems();
        let (hover, active) = (colors.hover, colors.active);

        let mut element = div()
            .id(self.id)
            .h(height)
            .px(padding_x)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .gap(Spacing::Small.rems())
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .text_color(colors.foreground)
            .text_size(self.size.text_size().pixels(theme.font_size))
            .font_weight(FontWeight::MEDIUM)
            .transitions(|transitions| {
                transitions
                    .bg(motion::fast_cubic())
                    .border_color(motion::fast_cubic())
                    .text_color(motion::fast_cubic())
            });

        element = if self.pill {
            element.rounded_full()
        } else {
            element.rounded(Radius::Medium.rems())
        };

        if self.w_full {
            element = element.w_full();
        }

        if let Some(tip) = self.tooltip {
            element = element.tooltip(crate::components::tooltip::tooltip(tip));
        }

        if self.disabled {
            element = element.opacity(0.45).cursor_not_allowed();
        } else {
            element = element
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(active));
            if let Some(on_click) = self.on_click {
                element = element.on_click(on_click);
            }
        }

        if let Some(icon) = self.icon {
            element = element.child(icon_element(icon, icon_size, colors.foreground));
        }

        if let Some(label) = self.label {
            element = element.child(div().min_w_0().truncate().child(label));
        }

        if let Some(right_icon) = self.right_icon {
            element = element.child(icon_element(right_icon, icon_size, colors.foreground));
        }

        element
    }
}

fn icon_element(path: SharedString, size: Rems, color: Hsla) -> impl IntoElement {
    svg()
        .path(path)
        .flex_shrink_0()
        .size(size)
        .text_color(color)
}

#[derive(IntoElement)]
pub struct IconButton {
    id: ElementId,
    icon: SharedString,
    icon_size: Option<f32>,
    size: Option<f32>,
    icon_color: Option<Hsla>,
    variant: ButtonVariant,
    shape_circle: bool,
    selected: bool,
    disabled: bool,
    tooltip: Option<SharedString>,
    on_click: Option<ClickListener>,
}

impl IconButton {
    pub fn new(id: impl Into<ElementId>, icon: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon: icon.into(),
            icon_size: None,
            size: None,
            icon_color: None,
            variant: ButtonVariant::Ghost,
            shape_circle: false,
            selected: false,
            disabled: false,
            tooltip: None,
            on_click: None,
        }
    }

    pub fn icon_size(mut self, size: f32) -> Self {
        self.icon_size = Some(size);
        self
    }

    pub fn button_size(mut self, size: f32) -> Self {
        self.size = Some(size);
        self
    }

    pub fn size(self, size: f32) -> Self {
        self.button_size(size)
    }

    pub fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = Some(color);
        self
    }

    pub fn shape_circle(mut self, circle: bool) -> Self {
        self.shape_circle = circle;
        self
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    pub fn tooltip(mut self, tooltip: impl Into<SharedString>) -> Self {
        self.tooltip = Some(tooltip.into());
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

impl RenderOnce for IconButton {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let colors = colors(theme, self.variant, self.selected);
        let foreground = self.icon_color.unwrap_or(colors.foreground);
        let button_size: Pixels = self
            .size
            .map(px)
            .unwrap_or_else(|| crate::tokens::units_to_pixels(size::CONTROL, theme.font_size));
        let icon_size: Pixels = self.icon_size.map(px).unwrap_or_else(|| {
            crate::tokens::units_to_pixels(IconSize::Medium.value(), theme.font_size)
        });
        let (hover, active) = (colors.hover, colors.active);

        let mut element = div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .size(button_size)
            .bg(colors.background)
            .border_1()
            .border_color(colors.border)
            .transitions(|transitions| transitions.bg(motion::fast_cubic()))
            .child(svg().path(self.icon).size(icon_size).text_color(foreground));

        element = if self.shape_circle {
            element.rounded_full()
        } else {
            element.rounded(Radius::Medium.rems())
        };

        if let Some(tip) = self.tooltip {
            element = element.tooltip(crate::components::tooltip::tooltip(tip));
        }

        if self.disabled {
            element = element.opacity(0.45).cursor_not_allowed();
        } else {
            element = element
                .cursor_pointer()
                .hover(move |style| style.bg(hover))
                .active(move |style| style.bg(active))
                .on_mouse_down(gpui::MouseButton::Left, |_, _, cx| cx.stop_propagation());
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
    fn test_button_builder() {
        let btn = Button::new("test-btn")
            .label("Submit")
            .variant(ButtonVariant::Outline)
            .size(ButtonSize::Lg)
            .pill(true)
            .disabled(false);

        assert_eq!(btn.size, ButtonSize::Lg);
        assert_eq!(btn.size.height(), size::CONTROL);
        assert_eq!(btn.variant, ButtonVariant::Outline);
        assert!(btn.pill);
        assert!(!btn.disabled);
    }

    #[test]
    fn test_icon_button_builder() {
        let btn = IconButton::new("test-icon-btn", "close.svg")
            .button_size(32.0)
            .icon_size(16.0)
            .variant(ButtonVariant::Ghost)
            .selected(true);

        assert_eq!(btn.size, Some(32.0));
        assert_eq!(btn.icon_size, Some(16.0));
        assert_eq!(btn.variant, ButtonVariant::Ghost);
        assert!(btn.selected);
    }

    #[test]
    fn selected_buttons_use_the_selection_role() {
        let theme = Theme::default();
        let selected = colors(&theme, ButtonVariant::Ghost, true);
        assert_eq!(selected.background, theme.element_selected());
        assert_eq!(selected.foreground, theme.accent());
        let outline = colors(&theme, ButtonVariant::Outline, false);
        assert_eq!(outline.border, theme.border_variant());
    }
}
