use gpui::{
    App, AppContext, Context, Entity, FocusHandle, Global, InteractiveElement, IntoElement,
    KeyDownEvent, LinuxWindowBackground, MouseButton, ParentElement, Render, Styled, Window,
    WindowBounds, WindowHandle, WindowKind, WindowOptions, div,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px,
};
use ui::components::fade_in;
use ui::theme::Theme;
use ui::theme::theme_manager::ThemeManager;
use ui::tokens::{size, units_to_pixels};

use crate::view::Launcher;

#[derive(Default)]
struct LauncherWindow {
    handle: Option<WindowHandle<Root>>,
    launcher: Option<Entity<Launcher>>,
}

impl Global for LauncherWindow {}

pub fn init(cx: &mut App) {
    cx.set_global(LauncherWindow::default());
}

pub fn toggle(cx: &mut App) {
    if cx.global::<LauncherWindow>().handle.is_some() {
        close(cx);
    } else {
        show(cx);
    }
}

pub fn show(cx: &mut App) {
    cx.defer(|cx| {
        if cx.global::<LauncherWindow>().handle.is_some() {
            return;
        }
        cx.set_global(ThemeManager::load_current());

        let display = cx.primary_display();
        let font_size = cx.global::<Theme>().font_size;
        let size = display
            .as_ref()
            .map(|display| display.bounds().size)
            .unwrap_or_else(|| {
                gpui::size(
                    units_to_pixels(size::PANEL_MIN_WIDTH, font_size),
                    units_to_pixels(size::PANEL_MIN_HEIGHT, font_size),
                )
            });
        let options = window_options(display.map(|display| display.id()), size);
        let opened = cx.open_window(options, |window, cx| {
            window.set_rem_size(cx.global::<Theme>().font_size());
            let launcher = shared_launcher(window, cx);
            cx.new(|cx| Root::new(launcher, window, cx))
        });
        match opened {
            Ok(handle) => cx.global_mut::<LauncherWindow>().handle = Some(handle),
            Err(error) => services::log_error!("Launcher", "Failed to open the launcher: {error}"),
        }
    });
}

pub fn close(cx: &mut App) {
    cx.defer(|cx| {
        if let Some(handle) = cx.global_mut::<LauncherWindow>().handle.take() {
            let _ = handle.update(cx, |_, window, _| window.remove_window());
        }
    });
}

// The launcher keeps its pop-launcher connection and results between openings.
fn shared_launcher(window: &mut Window, cx: &mut App) -> Entity<Launcher> {
    if let Some(launcher) = cx.global::<LauncherWindow>().launcher.clone() {
        launcher.update(cx, |launcher, cx| launcher.reset(cx));
        return launcher;
    }
    let launcher = cx.new(|cx| Launcher::new(window, cx));
    cx.global_mut::<LauncherWindow>().launcher = Some(launcher.clone());
    launcher
}

struct Root {
    launcher: Entity<Launcher>,
    focus: FocusHandle,
}

impl Root {
    fn new(launcher: Entity<Launcher>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        window.focus(&launcher.read(cx).focus_handle(), cx);
        Self {
            launcher,
            focus: cx.focus_handle(),
        }
    }
}

impl Render for Root {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let font_size = theme.font_size;
        let viewport = window.viewport_size();
        let width = units_to_pixels(size::LAUNCHER, font_size);

        div()
            .id("launcher-root")
            .track_focus(&self.focus)
            .size_full()
            .font_family(theme.font_family())
            .text_size(theme.font_size())
            .text_color(theme.text())
            .on_key_down(cx.listener(|_, event: &KeyDownEvent, _, cx| {
                if event.keystroke.key == "escape" {
                    close(cx);
                    cx.stop_propagation();
                }
            }))
            .on_mouse_down(MouseButton::Left, |_, _, cx| close(cx))
            .on_mouse_down(MouseButton::Right, |_, _, cx| close(cx))
            .child(
                div()
                    .absolute()
                    .top(units_to_pixels(size::LAUNCHER_OFFSET, font_size))
                    .left(((viewport.width - width) / 2.0).max(px(0.0)))
                    .w(width)
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_mouse_down(MouseButton::Right, |_, _, cx| cx.stop_propagation())
                    .child(fade_in("launcher-content", self.launcher.clone())),
            )
    }
}

fn window_options(
    display: Option<gpui::DisplayId>,
    size: gpui::Size<gpui::Pixels>,
) -> WindowOptions {
    WindowOptions {
        display_id: display,
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size,
        })),
        app_id: Some("starshell-launcher".to_string()),
        linux_window_background: LinuxWindowBackground::Transparent,
        inactive_frame_interval: None,
        kind: WindowKind::LayerShell(LayerShellOptions {
            namespace: "starshell-launcher".to_string(),
            layer: Layer::Overlay,
            anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
            exclusive_zone: Some(px(0.0)),
            keyboard_interactivity: KeyboardInteractivity::Exclusive,
            ..Default::default()
        }),
        ..Default::default()
    }
}
