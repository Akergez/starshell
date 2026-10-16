pub mod ipc;
mod launch_apps;
pub mod overlay;
pub mod state;

use gpui::{
    App, AppContext, DisplayId, Global, LinuxWindowBackground, Pixels, WindowBounds, WindowHandle,
    WindowKind, WindowOptions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px,
};
use services::{AppState, BarEdge};
use ui::theme::Theme;
use ui::tokens::{size, units_to_pixels};

use crate::applets::AppletKind;
use crate::bar::Bar;
use overlay::Overlay;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Surface {
    Applet(AppletKind),
    Launcher,
    Polkit,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Placement {
    pub display: Option<DisplayId>,
    pub x: Option<Pixels>,
}

impl Placement {
    pub fn at(display: Option<DisplayId>, x: Pixels) -> Self {
        Self {
            display,
            x: Some(x),
        }
    }
}

#[derive(Default)]
pub struct Shell {
    bars: Vec<WindowHandle<Bar>>,
    overlay: Option<WindowHandle<Overlay>>,
    overlay_display: Option<DisplayId>,
    overlay_sizes: Vec<(Option<DisplayId>, gpui::Size<Pixels>)>,
    surface: Option<Surface>,
}

impl Global for Shell {}

impl Shell {
    pub fn init(cx: &mut App) {
        cx.set_global(Shell::default());
        cx.spawn(async move |cx| {
            let mut delay = std::time::Duration::from_millis(20);
            loop {
                let ready = cx.update(|cx| {
                    if cx.displays().is_empty() {
                        return false;
                    }
                    Self::open_bars(cx);
                    true
                });
                if ready {
                    break;
                }
                cx.background_executor().timer(delay).await;
                delay = (delay * 2).min(std::time::Duration::from_secs(1));
            }
        })
        .detach();
        Self::watch_config(cx);
    }

    pub fn current(cx: &App) -> Option<Surface> {
        cx.try_global::<Shell>().and_then(|shell| shell.surface)
    }

    pub fn bar_height(cx: &App) -> Pixels {
        let size = cx.global::<AppState>().config.get().bar.clamped_height();
        units_to_pixels(size, cx.global::<Theme>().font_size).round()
    }

    pub fn bar_position(cx: &App) -> BarEdge {
        cx.global::<AppState>().config.get().bar.edge
    }

    pub fn toggle(surface: Surface, placement: Placement, cx: &mut App) {
        cx.defer(move |cx| {
            if Self::current(cx) == Some(surface) {
                Self::close_now(cx);
            } else {
                Self::open_now(surface, placement, cx);
            }
        });
    }

    pub fn open(surface: Surface, placement: Placement, cx: &mut App) {
        cx.defer(move |cx| Self::open_now(surface, placement, cx));
    }

    pub fn close(cx: &mut App) {
        cx.defer(Self::close_now);
    }

    pub fn replace(surface: Surface, cx: &mut App) {
        cx.defer(move |cx| {
            let shell = cx.global::<Shell>();
            let display = shell.overlay_display;
            let x = shell
                .overlay
                .and_then(|handle| handle.read(cx).ok())
                .and_then(|overlay| overlay.anchor());
            Self::open_now(surface, Placement { display, x }, cx);
        });
    }

    pub fn open_settings(cx: &mut App) {
        cx.defer(|cx| {
            Self::close_now(cx);
            crate::panel::SettingsPanel::open(cx);
        });
    }

    pub fn close_surface(surface: Surface, cx: &mut App) {
        cx.defer(move |cx| {
            if Self::current(cx) == Some(surface) {
                Self::close_now(cx);
            }
        });
    }

    fn open_now(surface: Surface, placement: Placement, cx: &mut App) {
        if Self::current(cx) == Some(Surface::Polkit) && surface != Surface::Polkit {
            return;
        }
        let display = placement
            .display
            .or_else(|| cx.primary_display().map(|display| display.id()));
        let existing = cx
            .global::<Shell>()
            .overlay
            .filter(|_| cx.global::<Shell>().overlay_display == display);
        if let Some(handle) = existing
            && handle
                .update(cx, |overlay, window, cx| {
                    overlay.show(surface, placement.x, window, cx)
                })
                .is_ok()
        {
            cx.global_mut::<Shell>().surface = Some(surface);
            Self::sync_bar_visibility(cx);
            cx.refresh_windows();
            return;
        }
        Self::close_now(cx);
        let size = cx
            .global::<Shell>()
            .overlay_sizes
            .iter()
            .find(|(known, _)| *known == display)
            .map(|(_, size)| *size)
            .unwrap_or_else(|| initial_overlay_size(display, cx));
        let options = overlay_options(display, size);
        let opened = cx.open_window(options, |window, cx| {
            window.set_rem_size(cx.global::<Theme>().font_size());
            cx.new(|cx| Overlay::new(surface, placement.x, window, cx))
        });
        match opened {
            Ok(handle) => {
                let shell = cx.global_mut::<Shell>();
                shell.overlay = Some(handle);
                shell.overlay_display = display;
                shell.surface = Some(surface);
                Self::sync_bar_visibility(cx);
                cx.refresh_windows();
            }
            Err(error) => services::log_error!("Shell", "Failed to open overlay: {error}"),
        }
    }

    fn close_now(cx: &mut App) {
        let shell = cx.global_mut::<Shell>();
        shell.surface = None;
        let display = shell.overlay_display.take();
        if let Some(handle) = shell.overlay.take() {
            let size = handle
                .update(cx, |_, window, _| {
                    let size = window.viewport_size();
                    window.remove_window();
                    size
                })
                .ok()
                .filter(|size| size.width > px(0.0) && size.height > px(0.0));
            if let Some(size) = size {
                let sizes = &mut cx.global_mut::<Shell>().overlay_sizes;
                sizes.retain(|(known, _)| *known != display);
                sizes.push((display, size));
            }
        }
        Self::sync_bar_visibility(cx);
        cx.refresh_windows();
    }

    fn sync_bar_visibility(cx: &mut App) {
        let bars = cx.global::<Shell>().bars.clone();
        for bar in bars {
            let _ = bar.update(cx, |bar, window, cx| bar.sync_visibility(window, cx));
        }
    }

    fn open_bars(cx: &mut App) {
        let displays: Vec<Option<DisplayId>> = cx
            .displays()
            .iter()
            .map(|display| Some(display.id()))
            .collect();
        let mut bars = Vec::new();
        for id in displays {
            let options = bar_options(id, cx);
            match cx.open_window(options, |window, cx| {
                window.set_rem_size(cx.global::<Theme>().font_size());
                cx.new(|cx| Bar::new(id, window, cx))
            }) {
                Ok(handle) => bars.push(handle),
                Err(error) => services::log_error!("Shell", "Failed to open bar: {error}"),
            }
        }
        cx.global_mut::<Shell>().bars = bars;
    }

    pub fn rebuild_bars(cx: &mut App) {
        cx.defer(|cx| {
            let bars = std::mem::take(&mut cx.global_mut::<Shell>().bars);
            for bar in bars {
                let _ = bar.update(cx, |_, window, _| window.remove_window());
            }
            Self::close_now(cx);
            Self::open_bars(cx);
        });
    }

    pub fn apply_rem_size(cx: &mut App) {
        let rem = cx.global::<Theme>().font_size();
        for window in cx.windows() {
            let _ = window.update(cx, |_, window, _| window.set_rem_size(rem));
        }
    }

    fn watch_config(cx: &mut App) {
        let mut changes = cx.global::<AppState>().config.subscribe();
        let initial = cx.global::<AppState>().config.get();
        cx.spawn(async move |cx| {
            let mut previous = (initial.bar.clone(), initial.ui.reduce_motion);
            while let Ok(config) = changes.recv().await {
                let current = (config.bar.clone(), config.ui.reduce_motion);
                if current == previous {
                    continue;
                }
                let bar_changed = current.0 != previous.0;
                previous = current;
                let reduce_motion = config.ui.reduce_motion;
                cx.update(|cx| {
                    cx.set_reduce_motion(reduce_motion);
                    if bar_changed {
                        Self::rebuild_bars(cx);
                    }
                });
            }
        })
        .detach();
    }
}

fn bar_options(display: Option<DisplayId>, cx: &App) -> WindowOptions {
    let height = Shell::bar_height(cx);
    let edge = match Shell::bar_position(cx) {
        BarEdge::Top => Anchor::TOP,
        BarEdge::Bottom => Anchor::BOTTOM,
    };
    WindowOptions {
        display_id: display,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: gpui::size(display_width(display, cx).unwrap_or(height), height),
        })),
        app_id: Some("starshell-bar".to_string()),
        linux_window_background: LinuxWindowBackground::Transparent,
        inactive_frame_interval: None,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "starshell-bar".to_string(),
            layer: Layer::Top,
            anchor: edge | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(if cx.global::<AppState>().config.get().bar.autohide {
                units_to_pixels(size::BAR_HIDDEN_STRIP, cx.global::<Theme>().font_size)
            } else {
                height
            }),
            keyboard_interactivity: KeyboardInteractivity::None,
            ..Default::default()
        }),
        ..Default::default()
    }
}

fn display_width(display: Option<DisplayId>, cx: &App) -> Option<Pixels> {
    cx.displays()
        .into_iter()
        .find(|candidate| display.is_none_or(|id| candidate.id() == id))
        .map(|display| display.bounds().size.width)
}

pub(crate) fn initial_overlay_size(display: Option<DisplayId>, cx: &App) -> gpui::Size<Pixels> {
    if let Some(display) = cx
        .displays()
        .into_iter()
        .find(|candidate| display.is_none_or(|id| candidate.id() == id))
    {
        return display.bounds().size;
    }
    let font_size = cx.global::<Theme>().font_size;
    gpui::size(
        units_to_pixels(size::PANEL_MIN_WIDTH, font_size),
        units_to_pixels(size::PANEL_MIN_HEIGHT, font_size),
    )
}

fn overlay_options(display: Option<DisplayId>, size: gpui::Size<Pixels>) -> WindowOptions {
    WindowOptions {
        display_id: display,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size,
        })),
        app_id: Some("starshell-overlay".to_string()),
        linux_window_background: LinuxWindowBackground::Transparent,
        inactive_frame_interval: None,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "starshell-overlay".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(px(0.0)),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
        ..Default::default()
    }
}
