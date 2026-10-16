use gpui::{Div, Hsla, Styled, div};
use ui::theme::Theme;
use ui::tokens::Radius;

pub(crate) fn background(theme: &Theme) -> Hsla {
    theme.background()
}

pub(crate) fn surface(theme: &Theme) -> Hsla {
    theme.surface()
}

pub(crate) fn raised(theme: &Theme) -> Hsla {
    theme.elevated()
}

pub(crate) fn hover(theme: &Theme) -> Hsla {
    theme.ghost_hover()
}

pub(crate) fn border(theme: &Theme) -> Hsla {
    theme.border_variant()
}

pub(crate) fn accent_card(theme: &Theme) -> Hsla {
    theme.element_selected()
}

pub(crate) fn card(theme: &Theme) -> Div {
    div().rounded(Radius::Large.rems()).bg(theme.elevated())
}
