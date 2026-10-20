use std::time::Duration;

use gpui::{
    App, AppContext, DisplayId, Global, LinuxWindowBackground, PlatformDisplay, WindowBounds,
    WindowHandle, WindowKind, WindowOptions, session_lock::SessionLockOptions,
};
use ui::theme::theme_manager::ThemeManager;

use crate::view::LockScreen;

// Outputs come back with new ids after sleep or hotplug, and every output needs a lock surface.
const DISPLAY_CHECK_INTERVAL: Duration = Duration::from_secs(1);
const FIRST_FRAME_DELAY: Duration = Duration::from_millis(200);

#[derive(Default)]
struct Surfaces {
    locked: bool,
    windows: Vec<(DisplayId, WindowHandle<LockScreen>)>,
}

impl Global for Surfaces {}

pub fn init(cx: &mut App) {
    cx.set_global(Surfaces::default());
}

pub fn lock(cx: &mut App) {
    if cx.global::<Surfaces>().locked {
        services::report_session_locked();
        return;
    }

    cx.set_global(ThemeManager::load_current());
    cx.global_mut::<Surfaces>().locked = true;
    cover_new_displays(cx);

    let count = cx.global::<Surfaces>().windows.len();
    if count == 0 {
        services::log_error!("Lock", "No lock surface could be opened");
        cx.global_mut::<Surfaces>().locked = false;
        return;
    }
    services::log_info!("Lock", "Session locked on {count} displays");

    cx.spawn(async move |cx| {
        cx.background_executor().timer(FIRST_FRAME_DELAY).await;
        services::report_session_locked();
    })
    .detach();
    watch_displays(cx);
}

pub fn close_all(cx: &mut App) {
    let windows = {
        let surfaces = cx.global_mut::<Surfaces>();
        surfaces.locked = false;
        std::mem::take(&mut surfaces.windows)
    };
    for (_, handle) in windows {
        let _ = handle.update(cx, |_, window, _| window.remove_window());
    }
}

fn cover_new_displays(cx: &mut App) {
    for display in cx.displays() {
        let id = display.id();
        if cx
            .global::<Surfaces>()
            .windows
            .iter()
            .any(|(covered, _)| *covered == id)
        {
            continue;
        }
        let opened = cx.open_window(window_options(&*display), |window, cx| {
            let view = cx.new(LockScreen::new);
            let focus = view.read(cx).focus_handle.clone();
            window.focus(&focus, cx);
            view
        });
        match opened {
            Ok(handle) => cx.global_mut::<Surfaces>().windows.push((id, handle)),
            Err(error) => services::log_error!("Lock", "Failed to lock display {id:?}: {error}"),
        }
    }
}

fn watch_displays(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(DISPLAY_CHECK_INTERVAL).await;
            let locked = cx.update(|cx| {
                let locked = cx.global::<Surfaces>().locked;
                if locked {
                    cover_new_displays(cx);
                }
                locked
            });
            if !locked {
                break;
            }
        }
    })
    .detach();
}

fn window_options(display: &dyn PlatformDisplay) -> WindowOptions {
    WindowOptions {
        display_id: Some(display.id()),
        titlebar: None,
        window_bounds: Some(WindowBounds::Windowed(display.bounds())),
        app_id: Some("starshell-lock".to_string()),
        linux_window_background: LinuxWindowBackground::Opaque,
        kind: WindowKind::SessionLock(SessionLockOptions {}),
        ..Default::default()
    }
}
