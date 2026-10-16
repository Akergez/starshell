use gpui::{
    AnyElement, Context, ElementId, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div,
};
use services::{AppState, tray::dbus_menu::DBusMenuItem};
use ui::components::{ListRow, caption, divider, panel};
use ui::theme::Theme;
use ui::tokens::{Spacing, units};

use crate::shell::Shell;
use crate::shell::state::ShellState;

pub struct TrayMenuApplet {
    index: usize,
    state: Entity<ShellState>,
    _observe: Subscription,
}

impl TrayMenuApplet {
    pub fn new(index: usize, cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            index,
            state,
            _observe: observe,
        }
    }
}

fn entries(
    items: &[DBusMenuItem],
    depth: usize,
    bus: &str,
    menu: &str,
    theme: &Theme,
    output: &mut Vec<AnyElement>,
) {
    for item in items.iter().filter(|item| item.visible) {
        if item.is_separator {
            output.push(divider(theme).into_any_element());
            continue;
        }
        let label = item.label.replace('_', "");
        let id = item.id;
        let bus_name = bus.to_string();
        let menu_path = menu.to_string();
        let row = ListRow::new(
            ElementId::named_usize("tray-entry", id.max(0) as usize),
            label,
        )
        .disabled(!item.enabled || !item.children.is_empty());
        let row = if item.children.is_empty() {
            row.on_click(move |_, _, cx| {
                cx.global::<AppState>().sni_host.trigger_menu(
                    bus_name.clone(),
                    menu_path.clone(),
                    id,
                );
                Shell::close(cx);
            })
        } else {
            row
        };
        output.push(
            div()
                .pl(units(Spacing::Large.value() * depth as f32))
                .child(row)
                .into_any_element(),
        );
        entries(&item.children, depth + 1, bus, menu, theme, output);
    }
}

impl Render for TrayMenuApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let item = self.state.read(cx).snapshot.tray.get(self.index).cloned();
        let mut rows = Vec::new();
        if let Some(item) = &item
            && let Some(menu) = &item.menu_path
        {
            entries(&item.menu_items, 0, &item.bus_name, menu, &theme, &mut rows);
        }
        let title = item
            .as_ref()
            .map(|item| {
                if item.title.is_empty() {
                    item.id.clone()
                } else {
                    item.title.clone()
                }
            })
            .unwrap_or_default();
        panel(&theme)
            .w_full()
            .gap(Spacing::XSmall.rems())
            .child(caption(&theme, title))
            .children(rows)
    }
}
