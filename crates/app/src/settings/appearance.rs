use gpui::{App, Global, Task};
use services::AppState;
use std::time::{Duration, Instant};
use ui::theme::animator::ThemeAnimator;
use ui::theme::{Theme, theme_manager::ThemeManager};

pub(crate) struct AppearanceStatus {
    pub error: Option<String>,
    animator: ThemeAnimator,
    animation: Option<Task<()>>,
    persistence: tokio::sync::mpsc::UnboundedSender<Theme>,
}

impl Global for AppearanceStatus {}

pub(crate) fn initialize(cx: &mut App) {
    let state = cx.global::<AppState>().clone();
    let mut configs = state.config.subscribe();
    let mut wallpapers = state.wallpaper.subscribe();
    let (sender, mut receiver) = tokio::sync::mpsc::channel(1);
    let (persist, mut persistence) = tokio::sync::mpsc::unbounded_channel::<Theme>();
    cx.set_global(AppearanceStatus {
        error: None,
        animator: ThemeAnimator::new(cx.global::<Theme>().clone()),
        animation: None,
        persistence: persist.clone(),
    });
    services::spawn_tokio(async move {
        while let Some(theme) = persistence.recv().await {
            match tokio::task::spawn_blocking(move || ThemeManager::save_current_theme(&theme))
                .await
            {
                Ok(Ok(())) => {}
                Ok(Err(error)) => eprintln!("Failed to persist appearance: {error}"),
                Err(error) => eprintln!("Failed to persist appearance: {error}"),
            }
        }
    });
    services::spawn_tokio(async move {
        let mut previous = None;
        loop {
            let config = state.config.get();
            let settings = (config.ui.dynamic_colors, config.ui.dark_mode);
            let path = state.wallpaper.get_current_wallpaper();
            let source = (settings, if settings.0 { path.clone() } else { None });
            if previous.as_ref() != Some(&source) {
                previous = Some(source.clone());
                let palette = if settings.0 {
                    if let Some(path) = path {
                        tokio::task::spawn_blocking(move || {
                            services::wallpaper::WallpaperService::dominant_color(&path)
                        })
                        .await
                        .map_err(|error| error.to_string())
                        .and_then(|result| result)
                        .map(Some)
                    } else {
                        Err("settings.dynamic_no_wallpaper".to_owned())
                    }
                } else {
                    Ok(None)
                };
                let current = state.config.get();
                if settings == (current.ui.dynamic_colors, current.ui.dark_mode)
                    && (!settings.0 || source.1 == state.wallpaper.get_current_wallpaper())
                    && sender.send((source, palette)).await.is_err()
                {
                    break;
                }
            }
            tokio::select! {
                event = configs.recv() => {
                    if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; }
                }
                event = wallpapers.recv() => {
                    if matches!(event, Err(tokio::sync::broadcast::error::RecvError::Closed)) { break; }
                    if state.config.get().ui.dynamic_colors { previous = None; }
                }
            }
        }
    });
    cx.spawn(async move |cx| {
        while let Some((source, palette)) = receiver.recv().await {
            let settings = source.0;
            cx.update(|cx| {
                let state = cx.global::<AppState>();
                let config = state.config.get();
                if settings != (config.ui.dynamic_colors, config.ui.dark_mode)
                    || (settings.0
                        && state
                            .wallpaper
                            .get_current()
                            .is_some_and(|path| Some(path) != source.1))
                {
                    return;
                }
                let selected = cx.global::<ThemeManager>().selected_theme.clone();
                let theme = match palette {
                    Ok(Some(rgb)) => {
                        cx.global_mut::<AppearanceStatus>().error = None;
                        Theme::from_wallpaper(rgb, settings.1, selected.font_family.clone())
                    }
                    Ok(None) => {
                        cx.global_mut::<AppearanceStatus>().error = None;
                        selected.with_mode(settings.1)
                    }
                    Err(error) => {
                        cx.global_mut::<AppearanceStatus>().error =
                            Some(if error.starts_with("settings.") {
                                cx.global::<AppState>().language.get(&error)
                            } else {
                                error
                            });
                        selected.with_mode(settings.1)
                    }
                };
                let manager = cx.global_mut::<ThemeManager>();
                manager.current_theme = theme.clone();
                manager.apply_theme_to_apps();
                transition_theme(theme.clone(), cx);
                let _ = persist.send(theme);
            });
        }
    })
    .detach();
}

pub(crate) fn persist_theme(theme: Theme, cx: &App) {
    let _ = cx.global::<AppearanceStatus>().persistence.send(theme);
}

pub(crate) fn transition_theme(theme: Theme, cx: &mut App) {
    let duration = Duration::from_millis(
        cx.global::<AppState>()
            .config
            .get()
            .ui
            .animation_duration_ms
            .min(180) as u64,
    );
    let interval = cx
        .global::<AppState>()
        .compositor
        .get_frame_duration()
        .max(Duration::from_millis(2));
    let status = cx.global_mut::<AppearanceStatus>();
    status.animation = None;
    let running = status
        .animator
        .transition_to(theme, duration, Instant::now());
    let displayed = status.animator.theme().clone();
    cx.set_global(displayed);
    cx.refresh_windows();
    if !running {
        return;
    }
    let animation = cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(interval).await;
            let running = cx.update(|cx| {
                let animator = &mut cx.global_mut::<AppearanceStatus>().animator;
                let running = animator.advance(Instant::now());
                let displayed = animator.theme().clone();
                cx.set_global(displayed);
                cx.refresh_windows();
                running
            });
            if !running {
                break;
            }
        }
    });
    cx.global_mut::<AppearanceStatus>().animation = Some(animation);
}
