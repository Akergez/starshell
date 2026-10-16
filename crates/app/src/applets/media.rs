use gpui::{
    AnyElement, Context, ElementId, Entity, FontWeight, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, img, prelude::FluentBuilder, svg,
};
use services::{MediaTrack, MprisService};
use ui::components::{Button, ButtonVariant, IconButton, Slider, caption, panel};
use ui::theme::Theme;
use ui::tokens::{IconSize, Radius, Spacing, TextSize, size, units};

use crate::shell::state::ShellState;

pub struct MediaApplet {
    state: Entity<ShellState>,
    selected: Option<String>,
    _positions: services::PositionWatch,
    _observe: Subscription,
}

impl MediaApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            selected: None,
            _positions: cx.global::<services::AppState>().mpris.watch_positions(),
            _observe: observe,
        }
    }
}

pub fn format_time(micros: i64) -> String {
    let seconds = (micros.max(0) / 1_000_000) as u64;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn control(id: &'static str, icon: &'static str, bus: String, action: fn(String)) -> IconButton {
    IconButton::new(id, icon).on_click(move |_, _, _| action(bus.clone()))
}

fn previous(bus: String) {
    services::spawn_tokio(async move {
        MprisService::previous_bus(&bus).await;
    });
}

fn next(bus: String) {
    services::spawn_tokio(async move {
        MprisService::next_bus(&bus).await;
    });
}

fn play_pause(bus: String) {
    services::spawn_tokio(async move {
        MprisService::play_pause_bus(&bus).await;
    });
}

fn cover(track: &MediaTrack, theme: &Theme) -> AnyElement {
    let frame = div()
        .size(units(size::COVER))
        .flex_shrink_0()
        .rounded(Radius::Large.rems())
        .overflow_hidden()
        .bg(theme.elevated());
    match track.local_art_path.clone() {
        Some(path) => frame.child(img(path).size_full()).into_any_element(),
        None => frame
            .flex()
            .items_center()
            .justify_center()
            .child(
                svg()
                    .path("music.svg")
                    .size(IconSize::XLarge.rems())
                    .text_color(theme.text_muted()),
            )
            .into_any_element(),
    }
}

impl Render for MediaApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let snapshot = self.state.read(cx).snapshot.clone();
        let track = self
            .selected
            .as_ref()
            .and_then(|bus| {
                snapshot
                    .players
                    .iter()
                    .find(|player| &player.bus_name == bus)
            })
            .or_else(|| snapshot.player())
            .cloned();
        let Some(track) = track else {
            return panel(&theme)
                .w_full()
                .child(caption(&theme, "Nothing is playing"))
                .into_any_element();
        };
        let bus = track.bus_name.clone();
        let small = TextSize::Small.pixels(theme.font_size);
        let length = track.length_micros.unwrap_or(0);
        let position = track.position_micros.unwrap_or(0);
        let progress = if length > 0 {
            position as f32 / length as f32
        } else {
            0.0
        };
        let players: Vec<MediaTrack> = snapshot
            .players
            .iter()
            .filter(|player| player.has_media)
            .cloned()
            .collect();

        panel(&theme)
            .w_full()
            .when(players.len() > 1, |panel| {
                panel.child(
                    div()
                        .flex()
                        .flex_wrap()
                        .gap(Spacing::XSmall.rems())
                        .children(players.iter().enumerate().map(|(index, player)| {
                            let player_bus = player.bus_name.clone();
                            Button::new(ElementId::named_usize("media-player", index))
                                .label(player.player_name.clone())
                                .variant(ButtonVariant::Ghost)
                                .selected(player.bus_name == track.bus_name)
                                .on_click(cx.listener(move |applet, _, _, cx| {
                                    applet.selected = Some(player_bus.clone());
                                    cx.notify();
                                }))
                        })),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::Large.rems())
                    .child(cover(&track, &theme))
                    .child(
                        div()
                            .flex()
                            .flex_col()
                            .flex_1()
                            .min_w_0()
                            .gap(Spacing::XXSmall.rems())
                            .child(
                                div()
                                    .truncate()
                                    .font_weight(FontWeight::MEDIUM)
                                    .text_size(TextSize::Large.pixels(theme.font_size))
                                    .child(track.title.clone()),
                            )
                            .child(
                                div()
                                    .truncate()
                                    .text_size(small)
                                    .text_color(theme.text_muted())
                                    .child(track.artist.clone()),
                            )
                            .when(!track.album.is_empty(), |column| {
                                column.child(
                                    div()
                                        .truncate()
                                        .text_size(small)
                                        .text_color(theme.text_muted())
                                        .child(track.album.clone()),
                                )
                            }),
                    ),
            )
            .when(length > 0, |panel| {
                let seek_bus = bus.clone();
                panel.child(
                    div()
                        .flex()
                        .flex_col()
                        .gap(Spacing::XXSmall.rems())
                        .child(Slider::new("media-position").value(progress).on_release(
                            move |fraction, _, _| {
                                let bus = seek_bus.clone();
                                let seconds = fraction as f64 * length as f64 / 1_000_000.0;
                                services::spawn_tokio(async move {
                                    MprisService::seek_to(&bus, seconds).await;
                                });
                            },
                        ))
                        .child(
                            div()
                                .flex()
                                .justify_between()
                                .text_size(small)
                                .text_color(theme.text_muted())
                                .child(format_time(position))
                                .child(format_time(length)),
                        ),
                )
            })
            .child(
                div()
                    .flex()
                    .items_center()
                    .justify_center()
                    .gap(Spacing::Large.rems())
                    .child(
                        control("media-previous", "skip-back.svg", bus.clone(), previous)
                            .disabled(!track.can_go_previous),
                    )
                    .child(
                        IconButton::new(
                            "media-play-pause",
                            if track.is_playing {
                                "pause.svg"
                            } else {
                                "play.svg"
                            },
                        )
                        .variant(ButtonVariant::Primary)
                        .shape_circle(true)
                        .button_size(size::PLAYBACK)
                        .icon_size(IconSize::XLarge.value())
                        .on_click({
                            let bus = bus.clone();
                            move |_, _, _| play_pause(bus.clone())
                        }),
                    )
                    .child(
                        control("media-next", "skip-forward.svg", bus, next)
                            .disabled(!track.can_go_next),
                    ),
            )
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn positions_are_minutes_and_seconds() {
        assert_eq!(format_time(0), "0:00");
        assert_eq!(format_time(83_000_000), "1:23");
        assert_eq!(format_time(-5), "0:00");
    }
}
