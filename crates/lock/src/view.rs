use gpui::{ColorExt, Context, KeyDownEvent, Render, Window, div, img, prelude::*, px, svg};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use ui::components::InputState;
use ui::theme::Theme;
use ui::tokens::{IconSize, Radius, Spacing, TextSize, size, units};

use crate::components::{render_auth_form, render_clock, render_lyrics};
use crate::state::LockState;

pub struct LockScreen {
    pub username: String,
    pub password_input: InputState,
    auth_failed: bool,
    is_checking: bool,
    pub focus_handle: gpui::FocusHandle,
    pub entry_start_time: Option<Instant>,
    last_track_key: String,
    last_track_pos_micros: u64,
    last_track_pos_time: Instant,
    pub current_lyric_idx: usize,
    pub lyric_change_time: Instant,
    pub blurred_wallpaper: Option<PathBuf>,
    fallback_blur_slot: Arc<Mutex<Option<PathBuf>>>,
    _positions: services::PositionWatch,
}

impl LockScreen {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let username = std::env::var("USER").unwrap_or_else(|_| "User".to_string());
        let focus_handle = cx.focus_handle();

        cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor()
                    .timer(Duration::from_millis(250))
                    .await;
                if this.update(cx, |_, cx| cx.notify()).is_err() {
                    break;
                }
            }
        })
        .detach();

        let positions = cx.global::<LockState>().mpris.watch_positions();
        let mut screen = Self {
            _positions: positions,
            username,
            password_input: InputState::new("").with_masked(true).with_focused(true),
            auth_failed: false,
            is_checking: false,
            focus_handle,
            entry_start_time: None,
            last_track_key: String::new(),
            last_track_pos_micros: 0,
            last_track_pos_time: Instant::now(),
            current_lyric_idx: 0,
            lyric_change_time: Instant::now(),
            blurred_wallpaper: None,
            fallback_blur_slot: Arc::new(Mutex::new(None)),
        };
        screen.init_fallback_blur();
        screen
    }

    fn init_fallback_blur(&mut self) {
        let wallpaper_path = dirs::home_dir().and_then(|h| {
            let conf = h.join(".config/starshell/current_wallpaper");
            std::fs::read_to_string(conf)
                .ok()
                .map(|s| PathBuf::from(s.trim()))
                .filter(|p| p.exists())
        });

        let Some(target) = wallpaper_path else {
            return;
        };

        let slot = self.fallback_blur_slot.clone();
        services::spawn_blocking(move || {
            let res = services::wallpaper::WallpaperService::create_or_get_blur(&target);
            if let Ok(mut guard) = slot.lock() {
                *guard = res;
            }
        });
    }

    pub fn submit_password(&mut self, cx: &mut Context<Self>) {
        let pass = self.password_input.text();
        if pass.is_empty() || self.is_checking {
            return;
        }

        self.is_checking = true;
        self.auth_failed = false;
        cx.notify();

        let (tx, mut rx) = tokio::sync::oneshot::channel();
        std::thread::spawn(move || {
            let res =
                std::panic::catch_unwind(|| match services::PamService::authenticate_current_user(
                    &pass,
                ) {
                    Ok(valid) => valid,
                    Err(error) => {
                        services::log_error!("PAM", "Authentication failed: {error:#}");
                        false
                    }
                })
                .unwrap_or(false);
            let _ = tx.send(res);
        });

        cx.spawn(async move |this, cx| {
            let mut is_valid = false;
            for _ in 0..100 {
                if let Ok(res) = rx.try_recv() {
                    is_valid = res;
                    break;
                }
                cx.background_executor()
                    .timer(Duration::from_millis(50))
                    .await;
            }

            let _ = this.update(cx, |this: &mut Self, cx| {
                this.is_checking = false;
                if is_valid {
                    cx.defer(|cx| {
                        crate::surfaces::close_all(cx);
                    });
                } else {
                    this.auth_failed = true;
                    this.password_input.clear();
                    cx.notify();
                }
            });
        })
        .detach();
    }
}

impl Render for LockScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if self.blurred_wallpaper.is_none()
            && let Ok(mut guard) = self.fallback_blur_slot.lock()
            && let Some(path) = guard.take()
        {
            self.blurred_wallpaper = Some(path);
        }

        let theme = cx.global::<Theme>().clone();
        let lock_config = cx.global::<LockState>().config.get().lockscreen.clone();

        {
            let app_state = cx.global::<LockState>();
            let track = app_state.mpris.get_current_track();
            if track.has_media {
                let id = format!("{} - {}", track.title, track.artist);
                let raw_pos = track.position_micros.unwrap_or(0).max(0) as u64;

                if self.last_track_key != id || raw_pos != self.last_track_pos_micros {
                    self.last_track_key = id;
                    self.last_track_pos_micros = raw_pos;
                    self.last_track_pos_time = Instant::now();
                }

                let estimated_pos = if track.is_playing {
                    raw_pos + self.last_track_pos_time.elapsed().as_micros() as u64
                } else {
                    raw_pos
                };
                let pos = Duration::from_micros(estimated_pos);

                if let Some(cached_opt) = app_state
                    .lyrics
                    .get_cached_lyrics(&track.title, &track.artist)
                {
                    if let Some(lyrics) = cached_opt.filter(|l| !l.synced_lines.is_empty()) {
                        let active_idx = lyrics
                            .synced_lines
                            .partition_point(|l| l.timestamp <= pos)
                            .saturating_sub(1);

                        if active_idx != self.current_lyric_idx {
                            self.current_lyric_idx = active_idx;
                            self.lyric_change_time = Instant::now();
                        }
                    }
                } else {
                    let dur_secs = track.length_micros.map(|l| (l.max(0) / 1_000_000) as u64);
                    app_state.lyrics.fetch_lyrics_in_background(
                        track.title.clone(),
                        track.artist.clone(),
                        if track.album.is_empty() {
                            None
                        } else {
                            Some(track.album.clone())
                        },
                        dur_secs,
                    );
                }
            }
        }

        let start_time = *self.entry_start_time.get_or_insert_with(Instant::now);
        let elapsed = start_time.elapsed().as_secs_f32() * 1000.0;
        let anim_duration = 600.0;
        let entry_t = (elapsed / anim_duration).min(1.0);
        let eased = 1.0 - (1.0 - entry_t).powi(3);

        if entry_t < 1.0 {
            window.request_animation_frame();
        }

        let lyric_elapsed = self.lyric_change_time.elapsed().as_secs_f32() * 1000.0;
        let lyric_anim_progress = (lyric_elapsed / 300.0).min(1.0);
        if lyric_anim_progress < 1.0 {
            window.request_animation_frame();
        }

        let clock_y = (1.0 - eased) * -28.0;
        let clock_opacity = eased;

        let lyrics_y = (1.0 - eased) * 16.0;
        let lyrics_opacity = eased;

        let user_y = (1.0 - eased) * 36.0;
        let user_opacity = eased;

        let top_bar_opacity = eased;

        let (battery_opt, wifi_icon_opt) = {
            let app_state = cx.global::<LockState>();
            let battery = app_state.power.get_battery();
            let net = app_state.network.get_status();
            let wifi_icon = if net.ethernet_connected {
                Some("ethernet.svg")
            } else if net.wifi_enabled && !net.wifi_ssid.is_empty() {
                if net.wifi_signal > 60 {
                    Some("wifi-high.svg")
                } else if net.wifi_signal > 20 {
                    Some("wifi-low.svg")
                } else {
                    Some("wifi.svg")
                }
            } else if net.wifi_enabled {
                Some("wifi-zero.svg")
            } else {
                None
            };
            (battery, wifi_icon)
        };

        let top_bar = div()
            .absolute()
            .top(Spacing::XXXLarge.rems())
            .right(units(size::CONTROL))
            .flex()
            .flex_row()
            .items_center()
            .gap(Spacing::Large.rems())
            .opacity(top_bar_opacity)
            .when_some(battery_opt, |d, bat| {
                d.child(
                    div()
                        .flex()
                        .flex_row()
                        .items_center()
                        .gap(Spacing::XSmall.rems())
                        .px(Spacing::Small.rems())
                        .py(Spacing::XXSmall.rems())
                        .rounded(Radius::Medium.rems())
                        .bg(theme.success().opacity(0.85))
                        .child(
                            div()
                                .font_family(theme.font_family())
                                .text_size(TextSize::XSmall.pixels(theme.font_size))
                                .font_weight(gpui::FontWeight::BOLD)
                                .text_color(theme.background())
                                .child(bat.percentage.to_string()),
                        )
                        .child(
                            svg()
                                .path(if bat.is_charging {
                                    "battery-charging.svg"
                                } else if bat.percentage <= 15 {
                                    "battery-low.svg"
                                } else if bat.percentage <= 40 {
                                    "battery-medium.svg"
                                } else {
                                    "battery-full.svg"
                                })
                                .size(IconSize::XSmall.rems())
                                .text_color(theme.background()),
                        ),
                )
            })
            .when_some(wifi_icon_opt, |d, icon| {
                d.child(
                    svg()
                        .path(icon)
                        .size(IconSize::Medium.rems())
                        .text_color(theme.text()),
                )
            });

        let content = div()
            .flex()
            .flex_col()
            .justify_between()
            .items_center()
            .size_full()
            .pt(units(size::LAUNCHER_OFFSET))
            .pb(units(size::LAUNCHER_OFFSET))
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .opacity(clock_opacity)
                    .mt(px(clock_y))
                    .child(render_clock(&theme, &lock_config)),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .w_full()
                    .max_w(units(size::LAUNCHER))
                    .px(Spacing::XXLarge.rems())
                    .opacity(lyrics_opacity)
                    .mt(px(lyrics_y))
                    .child(render_lyrics(
                        &theme,
                        self.current_lyric_idx,
                        lyric_anim_progress,
                        cx,
                    )),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .items_center()
                    .gap(Spacing::Medium.rems())
                    .opacity(user_opacity)
                    .mt(px(user_y))
                    .child(
                        div()
                            .flex()
                            .items_center()
                            .justify_center()
                            .w(units(size::PLAYBACK))
                            .h(units(size::PLAYBACK))
                            .rounded_full()
                            .bg(theme.elevated().opacity(0.45))
                            .border_1()
                            .border_color(theme.elevated().opacity(0.35))
                            .child(
                                svg()
                                    .path("user.svg")
                                    .size(IconSize::XLarge.rems())
                                    .text_color(theme.text().opacity(0.95)),
                            ),
                    )
                    .child(
                        div()
                            .font_family(theme.font_family())
                            .font_weight(gpui::FontWeight::MEDIUM)
                            .text_size(TextSize::Small.pixels(theme.font_size))
                            .text_color(theme.text().opacity(0.9))
                            .child(self.username.clone()),
                    )
                    .child(render_auth_form(
                        &theme,
                        &self.password_input,
                        self.auth_failed,
                        self.is_checking,
                    )),
            );

        div()
            .id("lockscreen-root")
            .key_context("LockScreen")
            .track_focus(&self.focus_handle)
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if this.is_checking {
                    return;
                }

                let key = event.keystroke.key.as_str();

                match key {
                    "escape" | "Escape" | "esc" | "\u{1b}" => {
                        this.password_input.clear();
                        this.auth_failed = false;
                        cx.notify();
                        return;
                    }
                    _ => {}
                }

                window.focus(&this.focus_handle, cx);

                match key {
                    "enter" | "return" | "numpad_enter" | "kp_enter" | "numpadenter" => {
                        this.submit_password(cx);
                    }
                    _ => {
                        if this.password_input.handle_key_down(event, cx) {
                            this.auth_failed = false;
                            cx.notify();
                        }
                    }
                }
            }))
            .on_mouse_down(
                gpui::MouseButton::Left,
                cx.listener(|this, _, window, cx| {
                    window.focus(&this.focus_handle, cx);
                }),
            )
            .relative()
            .w_full()
            .h_full()
            .bg(theme.background())
            .when_some(self.blurred_wallpaper.as_ref(), |this, blur_path| {
                this.child(
                    img(blur_path.clone())
                        .size_full()
                        .absolute()
                        .top_0()
                        .left_0()
                        .object_fit(gpui::ObjectFit::Cover),
                )
            })
            .child(
                div()
                    .size_full()
                    .absolute()
                    .top_0()
                    .left_0()
                    .bg(theme.background().opacity(0.35)),
            )
            .child(top_bar)
            .child(div().relative().size_full().child(content))
            .into_any_element()
    }
}
