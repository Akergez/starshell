pub mod items;

use gpui::{
    AnyElement, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, svg,
};
use ui::theme::Theme;
use ui::tokens::{IconSize, Radius, Spacing, TextSize, size, units};

use crate::settings::Settings;
use crate::settings::style;
pub use items::{SettingsTab, SidebarItem, TabSidebarItem};

pub struct Sidebar {
    pub items: Vec<Box<dyn SidebarItem>>,
}

impl Sidebar {
    pub fn new() -> Self {
        let items: Vec<Box<dyn SidebarItem>> = vec![
            Box::new(TabSidebarItem::new(
                SettingsTab::Appearance,
                "Appearance",
                "palette.svg",
            )),
            Box::new(TabSidebarItem::new(
                SettingsTab::Apps,
                "Applications",
                "grid-dots.svg",
            )),
            Box::new(TabSidebarItem::new(
                SettingsTab::Media,
                "Media",
                "music.svg",
            )),
            Box::new(TabSidebarItem::new(
                SettingsTab::LockScreen,
                "Lock Screen",
                "lock.svg",
            )),
            Box::new(TabSidebarItem::new(
                SettingsTab::System,
                "System",
                "settings.svg",
            )),
            Box::new(TabSidebarItem::new(
                SettingsTab::Record,
                "Recording",
                "play.svg",
            )),
        ];

        Self { items }
    }

    pub fn render(
        &self,
        active_tab: SettingsTab,
        has_search_query: bool,
        theme: &Theme,
        cx: &mut Context<Settings>,
    ) -> AnyElement {
        div()
            .id("settings-tabs-scroll")
            .flex()
            .flex_col()
            .gap(Spacing::Small.rems())
            .w(units(size::PANEL_MIN_WIDTH))
            .max_w(gpui::relative(0.5))
            .h_full()
            .min_h_0()
            .min_w_0()
            .flex_shrink_0()
            .overflow_y_scroll()
            .children(self.items.iter().map(|item| {
                let tab = item.tab();
                let selected = active_tab == tab && !has_search_query;
                let foreground = if selected {
                    theme.text()
                } else {
                    theme.text_muted()
                };
                let title = item.title(cx);
                let icon = item.icon();

                div()
                    .id(("settings-tab", tab as usize))
                    .flex()
                    .items_center()
                    .gap(Spacing::Large.rems())
                    .w_full()
                    .min_w_0()
                    .whitespace_nowrap()
                    .h(units(size::TILE))
                    .px(Spacing::XLarge.rems())
                    .flex_shrink_0()
                    .rounded(Radius::XLarge.rems())
                    .when(selected, |s| s.bg(style::surface(theme)))
                    .text_color(foreground)
                    .text_size(TextSize::Small.pixels(theme.font_size))
                    .font_weight(if selected {
                        FontWeight::MEDIUM
                    } else {
                        FontWeight::NORMAL
                    })
                    .cursor_pointer()
                    .hover(|s| s.bg(style::hover(theme)))
                    .on_click(cx.listener(move |this, _, _window, cx| {
                        this.set_tab(tab, cx);
                    }))
                    .child(
                        svg()
                            .path(icon)
                            .size(IconSize::Large.rems())
                            .flex_shrink_0()
                            .text_color(foreground),
                    )
                    .child(div().flex_1().min_w_0().text_ellipsis().child(title))
            }))
            .into_any_element()
    }
}
