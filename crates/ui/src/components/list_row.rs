use crate::components::button::ClickListener;
use crate::theme::{Theme, mix};
use crate::tokens::{IconSize, Radius, Spacing, TextSize, motion};
use gpui::{
    AnyElement, App, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement, MouseButton,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, Window, div,
    prelude::FluentBuilder, svg,
};

#[derive(IntoElement)]
pub struct ListRow {
    id: ElementId,
    icon: Option<SharedString>,
    icon_color: Option<Hsla>,
    title: SharedString,
    status: Option<SharedString>,
    status_accent: bool,
    selected: bool,
    disabled: bool,
    actions: Vec<AnyElement>,
    on_click: Option<ClickListener>,
}

impl ListRow {
    pub fn new(id: impl Into<ElementId>, title: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            icon: None,
            icon_color: None,
            title: title.into(),
            status: None,
            status_accent: false,
            selected: false,
            disabled: false,
            actions: Vec::new(),
            on_click: None,
        }
    }

    pub fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    pub fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = Some(color);
        self
    }

    pub fn status(mut self, status: impl Into<SharedString>) -> Self {
        self.status = Some(status.into());
        self
    }

    pub fn status_accent(mut self, accent: bool) -> Self {
        self.status_accent = accent;
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

    pub fn action(mut self, action: impl IntoElement) -> Self {
        self.actions.push(action.into_any_element());
        self
    }

    pub fn on_click(
        mut self,
        listener: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Box::new(listener));
        self
    }
}

impl RenderOnce for ListRow {
    fn render(self, _window: &mut Window, cx: &mut App) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let background = if self.selected {
            theme.element_selected()
        } else {
            gpui::transparent_black()
        };
        let base = if self.selected {
            theme.element_selected()
        } else {
            theme.elevated()
        };
        let hover = if self.selected {
            mix(base, theme.text(), 0.06)
        } else {
            theme.ghost_hover()
        };
        let active = mix(hover, theme.text(), 0.08);
        let status_color = if self.status_accent {
            theme.accent()
        } else {
            theme.text_muted()
        };
        let interactive = self.on_click.is_some() && !self.disabled;
        let small_text = TextSize::Small.pixels(theme.font_size);

        div()
            .id(self.id)
            .flex()
            .items_center()
            .w_full()
            .min_w_0()
            .gap(Spacing::Medium.rems())
            .px(Spacing::Medium.rems())
            .py(Spacing::Small.rems())
            .rounded(Radius::Medium.rems())
            .bg(background)
            .transitions(|transitions| transitions.bg(motion::fast_cubic()))
            .when(self.disabled, |row| row.opacity(0.5))
            .when(interactive, |row| {
                row.cursor_pointer()
                    .hover(move |style| style.bg(hover))
                    .active(move |style| style.bg(active))
            })
            .when_some(self.icon, |row, icon| {
                row.child(
                    svg()
                        .path(icon)
                        .flex_shrink_0()
                        .size(IconSize::Small.rems())
                        .text_color(self.icon_color.unwrap_or_else(|| theme.text_muted())),
                )
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .flex_1()
                    .min_w_0()
                    .child(
                        div()
                            .truncate()
                            .text_size(small_text)
                            .text_color(theme.text())
                            .child(self.title),
                    )
                    .when_some(self.status, |column, status| {
                        column.child(
                            div()
                                .truncate()
                                .text_size(small_text)
                                .text_color(status_color)
                                .child(status),
                        )
                    }),
            )
            .when(!self.actions.is_empty(), |row| {
                row.child(
                    div()
                        .flex()
                        .flex_shrink_0()
                        .items_center()
                        .gap(Spacing::XXSmall.rems())
                        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                        .children(self.actions),
                )
            })
            .when_some(self.on_click.filter(|_| interactive), |row, on_click| {
                row.on_click(on_click)
            })
    }
}
