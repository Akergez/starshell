use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    App, AppContext, Context, Global, InteractiveElement, IntoElement, LinuxWindowBackground,
    ParentElement, Pixels, Render, StatefulInteractiveElement, Styled, Task, Window, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, canvas, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px,
};
use services::{BarEdge, NotificationItem, NotificationStore};
use ui::components::fade_in;
use ui::theme::Theme;
use ui::tokens::{Spacing, size, units_to_pixels};

use crate::applets::AppletKind;
use crate::shell::{Placement, Shell, Surface};

#[derive(Default)]
struct PopupWindow(Option<WindowHandle<NotificationPopup>>);

impl Global for PopupWindow {}

pub fn init(cx: &mut App) {
    cx.set_global(PopupWindow::default());
    let mut changes = NotificationStore::global().subscribe();
    cx.spawn(async move |cx| {
        loop {
            cx.update(sync);
            if changes.changed().await.is_err() {
                break;
            }
        }
    })
    .detach();
}

fn sync(cx: &mut App) {
    let latest = NotificationStore::global().get_latest_active_notification();
    let open = cx.global::<PopupWindow>().0;
    match (latest, open) {
        (Some(_), None) => {
            let options = window_options(cx);
            match cx.open_window(options, |window, cx| {
                window.set_rem_size(cx.global::<Theme>().font_size());
                cx.new(NotificationPopup::new)
            }) {
                Ok(handle) => cx.global_mut::<PopupWindow>().0 = Some(handle),
                Err(error) => eprintln!("Failed to open notification popup: {error}"),
            }
        }
        (None, Some(handle)) => {
            cx.global_mut::<PopupWindow>().0 = None;
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
        (Some(_), Some(handle)) => {
            let _ = handle.update(cx, |popup, _, cx| popup.refresh(cx));
        }
        (None, None) => {}
    }
}

fn popup_width(cx: &App) -> Pixels {
    units_to_pixels(size::PANEL, cx.global::<Theme>().font_size)
}

fn window_options(cx: &App) -> WindowOptions {
    let font_size = cx.global::<Theme>().font_size;
    let gap = Spacing::Medium.pixels(font_size);
    let edge = match Shell::bar_position(cx) {
        BarEdge::Top => Anchor::TOP,
        BarEdge::Bottom => Anchor::BOTTOM,
    };
    WindowOptions {
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: gpui::size(popup_width(cx), units_to_pixels(size::TILE, font_size)),
        })),
        app_id: Some("starshell-notifications".to_string()),
        linux_window_background: LinuxWindowBackground::Transparent,
        inactive_frame_interval: None,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "starshell-notifications".to_string(),
            layer: Layer::Overlay,
            anchor: edge | Anchor::RIGHT,
            margin: Some((gap, gap, gap, gap)),
            exclusive_zone: Some(px(0.0)),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

pub struct NotificationPopup {
    item: Option<NotificationItem>,
    height: Rc<Cell<Pixels>>,
    _expiry: Option<Task<()>>,
}

impl NotificationPopup {
    fn new(cx: &mut Context<Self>) -> Self {
        let mut popup = Self {
            item: None,
            height: Rc::new(Cell::new(px(0.0))),
            _expiry: None,
        };
        popup.refresh(cx);
        popup
    }

    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.item = NotificationStore::global().get_latest_active_notification();
        self._expiry = self.item.as_ref().map(|item| {
            let remaining = item
                .timeout
                .saturating_sub(item.received_at.elapsed())
                .max(Duration::from_millis(100));
            cx.spawn(async move |_, cx| {
                cx.background_executor().timer(remaining).await;
                cx.update(sync);
            })
        });
        cx.notify();
    }
}

impl Render for NotificationPopup {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let Some(item) = self.item.clone() else {
            return div().into_any_element();
        };
        let height = self.height.clone();
        let width = popup_width(cx);
        let card = super::render_card(&item, &theme, |_, cx| {
            Shell::open(
                Surface::Applet(AppletKind::Notifications),
                Placement::default(),
                cx,
            );
        })
        .bg(theme.surface())
        .border_1()
        .border_color(theme.border_variant())
        .font_family(theme.font_family())
        .text_size(theme.font_size())
        .text_color(theme.text());

        div()
            .id("notification-popup")
            .w(width)
            .on_hover(|hovered, _, _| NotificationStore::global().set_hovered(*hovered))
            .child(fade_in(("notification-popup", item.id as usize), card))
            .child(
                canvas(
                    move |bounds, window, _| {
                        let measured = bounds.origin.y.max(px(0.0)).ceil();
                        if measured > px(0.0) && (measured - height.get()).abs() >= px(1.0) {
                            height.set(measured);
                            window.on_next_frame(move |window, _| {
                                window.resize(gpui::size(width, measured));
                            });
                        }
                    },
                    |_, _, _, _| {},
                )
                .w_full()
                .h(px(0.0)),
            )
            .into_any_element()
    }
}
