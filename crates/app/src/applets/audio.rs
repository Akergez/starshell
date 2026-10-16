use gpui::{
    AnyElement, Context, ElementId, Entity, IntoElement, ParentElement, Render, Styled,
    Subscription, Window, div, prelude::FluentBuilder,
};
use services::{AppState, AppStream, AudioSink, SystemStatus};
use ui::components::{
    Button, ButtonSize, ButtonVariant, IconButton, ListRow, Slider, SliderRow, caption, divider,
    panel, reveal, section_label,
};
use ui::theme::Theme;
use ui::tokens::Spacing;

use super::common;
use crate::shell::state::ShellState;

pub struct AudioApplet {
    state: Entity<ShellState>,
    stream_menu: Option<u32>,
    _observe: Subscription,
}

impl AudioApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            state,
            stream_menu: None,
            _observe: observe,
        }
    }
}

fn run_audio<F>(cx: &gpui::App, action: impl FnOnce(services::SystemService) -> F + Send + 'static)
where
    F: std::future::Future<Output = anyhow::Result<()>> + Send + 'static,
{
    let system = cx.global::<AppState>().system.clone();
    services::spawn_tokio(async move {
        if let Err(error) = action(system).await {
            services::log_warn!("Audio", "{error}");
        }
    });
}

fn device_row(
    id: ElementId,
    icon: &'static str,
    label: String,
    selected: bool,
    theme: &Theme,
    on_select: impl Fn(&mut gpui::App) + 'static,
) -> AnyElement {
    let row = ListRow::new(id.clone(), label)
        .icon(icon)
        .icon_color(if selected {
            theme.accent()
        } else {
            theme.text_muted()
        })
        .selected(selected)
        .on_click(move |_, _, cx| on_select(cx));
    if selected {
        row.action(
            IconButton::new(
                ElementId::NamedChild(std::sync::Arc::new(id), "check".into()),
                "check.svg",
            )
            .icon_color(theme.accent())
            .disabled(true),
        )
        .into_any_element()
    } else {
        row.into_any_element()
    }
}

fn sink_label(sinks: &[AudioSink], name: &str) -> String {
    sinks
        .iter()
        .find(|sink| sink.name == name)
        .map(|sink| sink.description.clone())
        .unwrap_or_else(|| "Default output".to_string())
}

impl AudioApplet {
    fn stream(
        &self,
        index: usize,
        stream: &AppStream,
        audio: &SystemStatus,
        theme: &Theme,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let stream_index = stream.index;
        let icon = stream
            .icon_name
            .as_deref()
            .and_then(services::icons::resolve_icon_name);
        let leading: AnyElement = match icon {
            Some(path) => div()
                .size(ui::tokens::units(ui::tokens::size::CONTROL))
                .flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .child(gpui::img(path).size(ui::tokens::IconSize::XLarge.rems()))
                .into_any_element(),
            None => SliderRow::mute_button(
                ElementId::named_usize("stream-mute", index),
                theme,
                crate::bar::volume_icon(stream.volume, stream.muted),
                stream.muted,
            )
            .on_click(move |_, _, cx| {
                run_audio(cx, move |system| async move {
                    system.toggle_stream_mute(stream_index).await
                })
            })
            .into_any_element(),
        };
        let open = self.stream_menu == Some(stream_index);
        let sinks = audio.audio_sinks.clone();
        div()
            .flex()
            .flex_col()
            .w_full()
            .gap(Spacing::XSmall.rems())
            .child(SliderRow::new(
                leading,
                stream.name.clone(),
                Slider::new(ElementId::named_usize("stream-volume", index))
                    .when(stream.muted, |slider| slider.color(theme.text_muted()))
                    .on_change(move |fraction, _, cx| {
                        let percent = (fraction * 100.0).round() as u32;
                        run_audio(cx, move |system| async move {
                            system.set_stream_volume(stream_index, percent).await
                        });
                        ShellState::refresh_now(cx);
                    }),
                stream.volume.min(100) as f32 / 100.0,
            ))
            .when(sinks.len() > 1, |column| {
                column.child(
                    div()
                        .pl(ui::tokens::units(
                            ui::tokens::size::CONTROL + Spacing::Medium.value(),
                        ))
                        .child(
                            Button::new(ElementId::named_usize("stream-output", index))
                                .label(sink_label(&sinks, &stream.sink))
                                .right_icon(if open {
                                    "chevron-up.svg"
                                } else {
                                    "chevron-down.svg"
                                })
                                .variant(ButtonVariant::Ghost)
                                .size(ButtonSize::Sm)
                                .on_click(cx.listener(move |applet, _, _, cx| {
                                    applet.stream_menu = if applet.stream_menu == Some(stream_index)
                                    {
                                        None
                                    } else {
                                        Some(stream_index)
                                    };
                                    cx.notify();
                                })),
                        ),
                )
            })
            .when(open, |column| {
                column.child(reveal(
                    ElementId::named_usize("stream-outputs", index),
                    div()
                        .flex()
                        .flex_col()
                        .gap(Spacing::XXSmall.rems())
                        .children(sinks.iter().enumerate().map(|(sink_index, sink)| {
                            let name = sink.name.clone();
                            let selected = sink.name == stream.sink;
                            device_row(
                                ElementId::named_usize(
                                    format!("stream-{stream_index}-sink"),
                                    sink_index,
                                ),
                                "volume-2.svg",
                                sink.description.clone(),
                                selected,
                                theme,
                                move |cx| {
                                    let name = name.clone();
                                    run_audio(cx, move |system| async move {
                                        system.move_stream(stream_index, &name).await
                                    });
                                },
                            )
                        })),
                ))
            })
            .into_any_element()
    }
}

impl Render for AudioApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let audio = self.state.read(cx).snapshot.audio.clone();
        let outputs = audio.audio_sinks.iter().enumerate().map(|(index, sink)| {
            let name = sink.name.clone();
            device_row(
                ElementId::named_usize("audio-output", index),
                "volume-2.svg",
                sink.description.clone(),
                sink.is_default,
                &theme,
                move |cx| {
                    let name = name.clone();
                    run_audio(cx, move |system| async move {
                        system.set_default_sink(&name).await
                    });
                },
            )
        });
        let inputs = audio
            .audio_sources
            .iter()
            .enumerate()
            .map(|(index, source)| {
                let name = source.name.clone();
                device_row(
                    ElementId::named_usize("audio-input", index),
                    "mic.svg",
                    source.description.clone(),
                    source.is_default,
                    &theme,
                    move |cx| {
                        let name = name.clone();
                        run_audio(cx, move |system| async move {
                            system.set_default_source(&name).await
                        });
                    },
                )
            });
        let streams: Vec<AnyElement> = audio
            .streams
            .iter()
            .enumerate()
            .map(|(index, stream)| self.stream(index, stream, &audio, &theme, cx))
            .collect();

        panel(&theme)
            .w_full()
            .child(section_label(&theme, "Output"))
            .when(audio.audio_sinks.is_empty(), |panel| {
                panel.child(caption(&theme, "No output devices"))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::XXSmall.rems())
                    .children(outputs),
            )
            .child(common::volume_row(&theme, &audio))
            .child(divider(&theme))
            .child(section_label(&theme, "Applications"))
            .when(streams.is_empty(), |panel| {
                panel.child(caption(&theme, "No applications are playing"))
            })
            .children(streams)
            .child(divider(&theme))
            .child(section_label(&theme, "Input"))
            .when(audio.audio_sources.is_empty(), |panel| {
                panel.child(caption(&theme, "No input devices"))
            })
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::XXSmall.rems())
                    .children(inputs),
            )
            .child(common::input_row(&theme, &audio))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_outputs_show_descriptions_not_device_names() {
        let sinks = vec![AudioSink {
            name: "alsa_output.usb-headset".into(),
            description: "USB Headset".into(),
            is_default: true,
        }];
        assert_eq!(sink_label(&sinks, "alsa_output.usb-headset"), "USB Headset");
        assert_eq!(sink_label(&sinks, "unknown"), "Default output");
    }
}
