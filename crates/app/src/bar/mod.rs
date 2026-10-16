mod icons;

use gpui::{
    AnyElement, App, ClickEvent, Context, DisplayId, ElementId, Entity, FontWeight, Hsla,
    InteractiveElement, IntoElement, MouseButton, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, Subscription, Window, div, img, prelude::FluentBuilder,
    svg,
};
use services::{AppState, BarConfig, BarModule, BarPosition, RecordStatus};
use ui::theme::{Theme, mix};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};

use crate::applets::AppletKind;
use crate::shell::state::{ShellState, Snapshot};
use crate::shell::{Placement, Shell, Surface};

pub use icons::{battery_icon, network_icon, volume_icon};

pub struct Bar {
    display: Option<DisplayId>,
    state: Entity<ShellState>,
    _observe: Subscription,
}

impl Bar {
    pub fn new(display: Option<DisplayId>, cx: &mut Context<Self>) -> Self {
        let state = ShellState::global(cx);
        let observe = cx.observe(&state, |_, _, cx| cx.notify());
        Self {
            display,
            state,
            _observe: observe,
        }
    }
}

struct Chip {
    id: ElementId,
    icon: Option<SharedString>,
    icon_color: Option<Hsla>,
    image: Option<std::path::PathBuf>,
    label: Option<SharedString>,
    selected: bool,
    max_width: Option<f32>,
}

impl gpui::prelude::FluentBuilder for Chip {}

impl Chip {
    fn new(id: impl Into<ElementId>) -> Self {
        Self {
            id: id.into(),
            icon: None,
            icon_color: None,
            image: None,
            label: None,
            selected: false,
            max_width: None,
        }
    }

    fn icon(mut self, icon: impl Into<SharedString>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    fn icon_color(mut self, color: Hsla) -> Self {
        self.icon_color = Some(color);
        self
    }

    fn image(mut self, path: Option<std::path::PathBuf>) -> Self {
        self.image = path;
        self
    }

    fn label(mut self, label: impl Into<SharedString>) -> Self {
        self.label = Some(label.into());
        self
    }

    fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    fn max_width(mut self, width: f32) -> Self {
        self.max_width = Some(width);
        self
    }

    fn render(self, theme: &Theme) -> gpui::Stateful<gpui::Div> {
        let base = if self.selected {
            theme.element_selected()
        } else {
            gpui::transparent_black()
        };
        let hover = if self.selected {
            mix(base, theme.text(), 0.06)
        } else {
            theme.ghost_hover()
        };
        let active = mix(hover, theme.text(), 0.08);
        let foreground = if self.selected {
            theme.accent()
        } else {
            theme.text()
        };
        div()
            .id(self.id)
            .flex()
            .flex_shrink_0()
            .items_center()
            .gap(Spacing::XSmall.rems())
            .h(units(size::BAR_CHIP))
            .min_w(units(size::BAR_CHIP))
            .justify_center()
            .px(Spacing::Small.rems())
            .rounded(Radius::Medium.rems())
            .bg(base)
            .text_color(foreground)
            .text_size(TextSize::Small.pixels(theme.font_size))
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .active(move |style| style.bg(active))
            .transitions(|transitions| {
                transitions
                    .bg(motion::fast_cubic())
                    .text_color(motion::fast_cubic())
            })
            .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
            .when_some(self.max_width, |chip, width| chip.max_w(units(width)))
            .when_some(self.image, |chip, path| {
                chip.child(img(path).flex_shrink_0().size(IconSize::Medium.rems()))
            })
            .when_some(self.icon, |chip, icon| {
                chip.child(
                    svg()
                        .path(icon)
                        .flex_shrink_0()
                        .size(IconSize::Small.rems())
                        .text_color(self.icon_color.unwrap_or(foreground)),
                )
            })
            .when_some(self.label, |chip, label| {
                chip.child(div().min_w_0().truncate().child(label))
            })
    }
}

impl Bar {
    fn toggle(&self, surface: Surface) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let display = self.display;
        move |event, _, cx| {
            let x = event.position().x;
            Shell::toggle(surface, Placement::at(display, x), cx);
        }
    }

    fn applet_chip(
        &self,
        kind: AppletKind,
        chip: Chip,
        theme: &Theme,
        current: Option<Surface>,
    ) -> AnyElement {
        chip.selected(current == Some(Surface::Applet(kind)))
            .render(theme)
            .on_click(self.toggle(Surface::Applet(kind)))
            .into_any_element()
    }

    fn module(
        &self,
        module: BarModule,
        snapshot: &Snapshot,
        theme: &Theme,
        current: Option<Surface>,
        cx: &App,
    ) -> Option<AnyElement> {
        Some(match module {
            BarModule::Launcher => Chip::new("bar-launcher")
                .icon("layout-grid.svg")
                .selected(current == Some(Surface::Launcher))
                .render(theme)
                .on_click(self.toggle(Surface::Launcher))
                .into_any_element(),
            BarModule::Workspaces => self.workspaces(snapshot, theme),
            BarModule::Media => {
                let player = snapshot.player()?;
                let title = if player.artist.is_empty() {
                    player.title.clone()
                } else {
                    format!("{} — {}", player.title, player.artist)
                };
                self.applet_chip(
                    AppletKind::Media,
                    Chip::new("bar-media")
                        .icon(if player.is_playing {
                            "music.svg"
                        } else {
                            "pause.svg"
                        })
                        .label(title)
                        .max_width(240.0),
                    theme,
                    current,
                )
            }
            BarModule::Clock => self.applet_chip(
                AppletKind::Calendar,
                Chip::new("bar-clock").label(format!("{}  {}", snapshot.date, snapshot.time)),
                theme,
                current,
            ),
            BarModule::Tray => self.tray(snapshot, theme, cx)?,
            BarModule::Notifications => {
                let chip = Chip::new("bar-notifications").icon(if snapshot.do_not_disturb {
                    "bell-off.svg"
                } else if snapshot.notification_count > 0 {
                    "bell-ring.svg"
                } else {
                    "bell.svg"
                });
                let chip = if snapshot.notification_count > 0 {
                    chip.label(snapshot.notification_count.to_string())
                } else {
                    chip
                };
                self.applet_chip(AppletKind::Notifications, chip, theme, current)
            }
            BarModule::Clipboard => self.applet_chip(
                AppletKind::Clipboard,
                Chip::new("bar-clipboard").icon("clipboard-list.svg"),
                theme,
                current,
            ),
            BarModule::Record => {
                let chip = Chip::new("bar-record");
                let chip = match snapshot.record {
                    RecordStatus::Stopped => chip.icon("video.svg"),
                    status => chip
                        .icon(if status == RecordStatus::Paused {
                            "pause.svg"
                        } else {
                            "video.svg"
                        })
                        .icon_color(theme.error())
                        .label(format!(
                            "{}:{:02}",
                            snapshot.record_seconds / 60,
                            snapshot.record_seconds % 60
                        )),
                };
                self.applet_chip(AppletKind::Record, chip, theme, current)
            }
            BarModule::Shelf => {
                let chip = Chip::new("bar-shelf").icon("folder.svg");
                let chip = if snapshot.shelf_count > 0 {
                    chip.label(snapshot.shelf_count.to_string())
                } else {
                    chip
                };
                self.applet_chip(AppletKind::Shelf, chip, theme, current)
            }
            BarModule::System => self.applet_chip(
                AppletKind::System,
                Chip::new("bar-system").icon("cpu.svg"),
                theme,
                current,
            ),
            BarModule::Audio => {
                let audio = &snapshot.audio;
                self.applet_chip(
                    AppletKind::Audio,
                    Chip::new("bar-audio")
                        .icon(volume_icon(audio.volume, audio.is_muted))
                        .when(audio.is_muted, |chip| chip.icon_color(theme.error())),
                    theme,
                    current,
                )
            }
            BarModule::Displays => {
                if snapshot.displays.is_empty() {
                    return None;
                }
                self.applet_chip(
                    AppletKind::Displays,
                    Chip::new("bar-displays").icon("sun.svg"),
                    theme,
                    current,
                )
            }
            BarModule::ControlCenter => self.status(snapshot, theme, current),
        })
    }

    fn workspaces(&self, snapshot: &Snapshot, theme: &Theme) -> AnyElement {
        let mut workspaces: Vec<_> = snapshot
            .workspaces
            .iter()
            .filter(|workspace| !workspace.is_special)
            .cloned()
            .collect();
        if workspaces.is_empty() {
            workspaces.push(snapshot.current_workspace.clone());
        }
        workspaces.sort_by_key(|workspace| workspace.num);
        let current = snapshot.current_workspace.id;
        div()
            .flex()
            .items_center()
            .gap(Spacing::XXSmall.rems())
            .children(workspaces.into_iter().map(|workspace| {
                let id = workspace.id;
                Chip::new(ElementId::named_usize(
                    "bar-workspace",
                    workspace.num.max(0) as usize,
                ))
                .label(workspace.num.to_string())
                .selected(id == current)
                .render(theme)
                .font_weight(FontWeight::MEDIUM)
                .on_click(move |_, _, cx| {
                    cx.global::<AppState>().compositor.switch_workspace(id);
                })
            }))
            .into_any_element()
    }

    fn tray(&self, snapshot: &Snapshot, theme: &Theme, _cx: &App) -> Option<AnyElement> {
        if snapshot.tray.is_empty() {
            return None;
        }
        let display = self.display;
        Some(
            div()
                .flex()
                .items_center()
                .gap(Spacing::XXSmall.rems())
                .children(snapshot.tray.iter().enumerate().map(|(index, item)| {
                    let image = item
                        .icon_file_path
                        .clone()
                        .map(std::path::PathBuf::from)
                        .or_else(|| services::icons::resolve_icon_name(&item.icon_name));
                    let chip = Chip::new(ElementId::named_usize("bar-tray", index));
                    let chip = if image.is_some() {
                        chip.image(image)
                    } else {
                        chip.icon("package.svg")
                    };
                    let has_menu = !item.menu_items.is_empty();
                    chip.render(theme)
                        .on_click(move |_, _, cx| {
                            cx.global::<AppState>().sni_host.activate_item(index);
                        })
                        .on_mouse_down(MouseButton::Right, move |event, _, cx| {
                            cx.stop_propagation();
                            if has_menu {
                                Shell::toggle(
                                    Surface::Applet(AppletKind::TrayMenu(index)),
                                    Placement::at(display, event.position.x),
                                    cx,
                                );
                            }
                        })
                }))
                .into_any_element(),
        )
    }

    fn status(&self, snapshot: &Snapshot, theme: &Theme, current: Option<Surface>) -> AnyElement {
        let network = &snapshot.network;
        let mut icons: Vec<(SharedString, Hsla)> =
            vec![(network_icon(network).into(), theme.text())];
        if network.bluetooth_enabled {
            icons.push(("bluetooth.svg".into(), theme.text()));
        }
        if snapshot.vpn.active().is_some() {
            icons.push(("lock.svg".into(), theme.accent()));
        }
        if snapshot.capture.microphone_in_use {
            icons.push(("mic.svg".into(), theme.warning()));
        }
        if snapshot.capture.camera_in_use {
            icons.push(("camera.svg".into(), theme.warning()));
        }
        let battery = snapshot.battery;
        let selected = current == Some(Surface::Applet(AppletKind::ControlCenter));
        let mut chip = Chip::new("bar-control-center")
            .selected(selected)
            .render(theme)
            .children(icons.into_iter().map(|(icon, color)| {
                svg()
                    .path(icon)
                    .flex_shrink_0()
                    .size(IconSize::Small.rems())
                    .text_color(if selected { theme.accent() } else { color })
            }));
        if let Some(battery) = battery {
            let color = if battery.is_critical() {
                theme.error()
            } else if battery.is_charging {
                theme.success()
            } else if battery.is_low() {
                theme.warning()
            } else if selected {
                theme.accent()
            } else {
                theme.text()
            };
            chip = chip
                .child(
                    svg()
                        .path(battery_icon(&battery))
                        .flex_shrink_0()
                        .size(IconSize::Small.rems())
                        .text_color(color),
                )
                .child(format!("{}%", battery.percentage));
        }
        chip.on_click(self.toggle(Surface::Applet(AppletKind::ControlCenter)))
            .into_any_element()
    }

    fn section(
        &self,
        modules: &[BarModule],
        snapshot: &Snapshot,
        theme: &Theme,
        current: Option<Surface>,
        cx: &App,
    ) -> gpui::Div {
        div()
            .flex()
            .items_center()
            .gap(Spacing::XSmall.rems())
            .children(
                modules
                    .iter()
                    .filter_map(|module| self.module(*module, snapshot, theme, current, cx)),
            )
    }
}

impl Render for Bar {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>();
        let config = cx.global::<AppState>().config.get();
        let bar: &BarConfig = &config.bar;
        let snapshot = self.state.read(cx).snapshot.clone();
        let current = Shell::current(cx);
        let bottom = bar.position == BarPosition::Bottom;
        let start = self.section(
            &BarConfig::modules(&bar.start),
            &snapshot,
            theme,
            current,
            cx,
        );
        let center = self.section(
            &BarConfig::modules(&bar.center),
            &snapshot,
            theme,
            current,
            cx,
        );
        let end = self.section(&BarConfig::modules(&bar.end), &snapshot, theme, current, cx);

        div()
            .id("bar")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_between()
            .px(Spacing::Small.rems())
            .bg(theme.surface())
            .when(!bottom, |bar| bar.border_b_1())
            .when(bottom, |bar| bar.border_t_1())
            .border_color(theme.border_variant())
            .font_family(theme.font_family())
            .text_size(TextSize::Small.pixels(theme.font_size))
            .text_color(theme.text())
            .on_mouse_down(MouseButton::Left, |_, _, cx| Shell::close(cx))
            .child(start)
            .child(
                div()
                    .absolute()
                    .inset_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(center),
            )
            .child(end)
    }
}
