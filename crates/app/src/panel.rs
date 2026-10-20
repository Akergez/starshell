use std::sync::atomic::{AtomicBool, Ordering};

use gpui::{
    AppContext, LinuxWindowBackground, WindowBounds, WindowHandle, WindowKind, WindowOptions,
    layer_shell::{Anchor, KeyboardInteractivity, Layer, LayerShellOptions},
    px,
};

use crate::settings::Settings;

static IS_SETTINGS_OPEN: AtomicBool = AtomicBool::new(false);

pub struct SettingsPanel;

impl SettingsPanel {
    pub fn is_open() -> bool {
        IS_SETTINGS_OPEN.load(Ordering::SeqCst)
    }

    pub fn mark_closed() {
        IS_SETTINGS_OPEN.store(false, Ordering::SeqCst);
    }

    pub fn window_options(cx: &gpui::App) -> WindowOptions {
        let display_id = cx.displays().first().map(|display| display.id());
        Self::surface_options(
            display_id,
            crate::shell::initial_overlay_size(display_id, cx),
        )
    }

    fn surface_options(
        display_id: Option<gpui::DisplayId>,
        size: gpui::Size<gpui::Pixels>,
    ) -> WindowOptions {
        WindowOptions {
            display_id,
            titlebar: None,
            window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                origin: gpui::point(px(0.0), px(0.0)),
                size,
            })),
            app_id: Some("starshell-settings".to_string()),
            inactive_frame_interval: None,
            linux_window_background: LinuxWindowBackground::Transparent,
            kind: WindowKind::LayerShell(LayerShellOptions {
                namespace: "starshell-settings".to_string(),
                layer: Layer::Overlay,
                anchor: Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT,
                exclusive_zone: Some(px(-1.0)),
                keyboard_interactivity: KeyboardInteractivity::OnDemand,
                ..Default::default()
            }),
            ..Default::default()
        }
    }

    pub fn open(cx: &mut gpui::App) -> Option<WindowHandle<Settings>> {
        if IS_SETTINGS_OPEN
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return None;
        }

        let options = Self::window_options(cx);

        match cx.open_window(options, |window, cx| cx.new(|cx| Settings::new(window, cx))) {
            Ok(w) => Some(w),
            Err(err) => {
                IS_SETTINGS_OPEN.store(false, Ordering::SeqCst);
                services::log_error!("Shell", "Failed to open settings window: {err}");
                None
            }
        }
    }

    pub fn close(cx: &mut gpui::App) {
        for window in cx.windows() {
            if let Some(settings_window) = window.downcast::<Settings>() {
                let _ = settings_window.update(cx, |this, _, cx| {
                    this.request_close(cx);
                });
            }
        }
    }

    pub fn toggle(cx: &mut gpui::App) {
        if Self::is_open() {
            Self::close(cx);
        } else {
            Self::open(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_surface_starts_with_a_drawable_size() {
        let size = gpui::size(px(640.0), px(480.0));
        for display_id in [
            None,
            Some(gpui::DisplayId::new(1)),
            Some(gpui::DisplayId::new(2)),
        ] {
            let options = SettingsPanel::surface_options(display_id, size);
            assert_eq!(options.display_id, display_id);
            let bounds = options.window_bounds.map(|bounds| bounds.get_bounds().size);
            assert_eq!(bounds, Some(size));
            let WindowKind::LayerShell(layer) = options.kind else {
                panic!("Settings must use layer shell");
            };
            assert_eq!(
                layer.anchor,
                Anchor::TOP | Anchor::BOTTOM | Anchor::LEFT | Anchor::RIGHT
            );
            assert_eq!(layer.exclusive_zone, Some(px(-1.0)));
        }
    }
}
