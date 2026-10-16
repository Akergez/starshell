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
use services::{AppState, BarPosition};
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
    surface: Option<Surface>,
}

impl Global for Shell {}

impl Shell {
    pub fn init(cx: &mut App) {
        cx.set_global(Shell::default());
        Self::open_bars(cx);
        Self::watch_config(cx);
    }

    pub fn current(cx: &App) -> Option<Surface> {
        cx.try_global::<Shell>().and_then(|shell| shell.surface)
    }

    pub fn bar_height(cx: &App) -> Pixels {
        let size = cx.global::<AppState>().config.get().bar.clamped_size();
        units_to_pixels(size, cx.global::<Theme>().font_size).round()
    }

    pub fn bar_position(cx: &App) -> BarPosition {
        cx.global::<AppState>().config.get().bar.position
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
            cx.refresh_windows();
            return;
        }
        Self::close_now(cx);
        let options = overlay_options(display, cx);
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
                cx.refresh_windows();
            }
            Err(error) => eprintln!("Failed to open overlay: {error}"),
        }
    }

    fn close_now(cx: &mut App) {
        let shell = cx.global_mut::<Shell>();
        shell.surface = None;
        shell.overlay_display = None;
        if let Some(handle) = shell.overlay.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
        cx.refresh_windows();
    }

    fn open_bars(cx: &mut App) {
        let mut displays: Vec<Option<DisplayId>> = cx
            .displays()
            .iter()
            .map(|display| Some(display.id()))
            .collect();
        if displays.is_empty() {
            displays.push(None);
        }
        let mut bars = Vec::new();
        for id in displays {
            let options = bar_options(id, cx);
            match cx.open_window(options, |window, cx| {
                window.set_rem_size(cx.global::<Theme>().font_size());
                cx.new(|cx| Bar::new(id, window, cx))
            }) {
                Ok(handle) => bars.push(handle),
                Err(error) => eprintln!("Failed to open bar: {error}"),
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
        BarPosition::Top => Anchor::TOP,
        BarPosition::Bottom => Anchor::BOTTOM,
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
            exclusive_zone: Some(if cx.global::<AppState>().config.get().bar.auto_hide {
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

fn initial_overlay_size(display: Option<DisplayId>, cx: &App) -> gpui::Size<Pixels> {
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

fn overlay_options(display: Option<DisplayId>, cx: &App) -> WindowOptions {
    WindowOptions {
        display_id: display,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: initial_overlay_size(display, cx),
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
