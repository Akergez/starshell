use gpui::{
    AnyElement, Context, ElementId, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, div,
};
use services::{AppState, PowerProfile, VpnService, VpnState, WifiAccessPoint};
use ui::components::{
    Button, ButtonVariant, IconButton, Input, InputState, ListRow, SegmentedControl, caption,
};
use ui::theme::Theme;
use ui::tokens::{Radius, Spacing, TextSize, size, units};

use super::{ControlCenter, Expanded, PasswordPrompt};
use crate::shell::Shell;
use crate::shell::state::{ShellState, Snapshot};

pub fn render(
    expanded: Expanded,
    center: &mut ControlCenter,
    snapshot: &Snapshot,
    theme: &Theme,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    match expanded {
        Expanded::Wifi => wifi(center, snapshot, theme, cx),
        Expanded::Bluetooth => bluetooth(snapshot, theme),
        Expanded::Vpn => vpn(snapshot, theme),
        Expanded::Performance => performance(snapshot),
    }
}

fn heading(
    theme: &Theme,
    label: impl Into<gpui::SharedString>,
    action: Option<IconButton>,
) -> AnyElement {
    div()
        .flex()
        .items_center()
        .w_full()
        .gap(Spacing::Small.rems())
        .child(caption(theme, label).flex_1())
        .children(action)
        .into_any_element()
}

fn list(id: &'static str, rows: Vec<AnyElement>) -> AnyElement {
    div()
        .id(id)
        .flex()
        .flex_col()
        .w_full()
        .gap(Spacing::XXSmall.rems())
        .max_h(units(size::NAVIGATION_LIST))
        .overflow_y_scroll()
        .children(rows)
        .into_any_element()
}

fn error_line(theme: &Theme, error: Option<&String>) -> Option<AnyElement> {
    error.map(|error| {
        div()
            .w_full()
            .text_size(TextSize::Small.pixels(theme.font_size))
            .text_color(theme.error())
            .child(error.clone())
            .into_any_element()
    })
}

fn signal_icon(signal: u8) -> &'static str {
    if signal >= 66 {
        "wifi.svg"
    } else if signal >= 33 {
        "wifi-high.svg"
    } else {
        "wifi-low.svg"
    }
}

pub fn visible_networks(networks: &[WifiAccessPoint]) -> Vec<WifiAccessPoint> {
    let mut visible: Vec<WifiAccessPoint> = Vec::new();
    for network in networks.iter().filter(|network| !network.ssid.is_empty()) {
        match visible.iter_mut().find(|known| known.ssid == network.ssid) {
            Some(known) => {
                if network.is_connected || (!known.is_connected && network.signal > known.signal) {
                    *known = network.clone();
                }
            }
            None => visible.push(network.clone()),
        }
    }
    visible.sort_by(|left, right| {
        right
            .is_connected
            .cmp(&left.is_connected)
            .then(right.is_saved.cmp(&left.is_saved))
            .then(right.signal.cmp(&left.signal))
    });
    visible
}

fn wifi(
    center: &mut ControlCenter,
    snapshot: &Snapshot,
    theme: &Theme,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    let network = &snapshot.network;
    if !network.wifi_enabled {
        return caption(theme, "Wi-Fi is off").into_any_element();
    }
    let label = if network.is_scanning_wifi {
        "Scanning…".to_string()
    } else if !network.wifi_ssid.is_empty() {
        format!("Connected to {}", network.wifi_ssid)
    } else {
        "Networks".to_string()
    };
    let refresh = IconButton::new("wifi-rescan", "refresh-cw.svg")
        .tooltip("Scan")
        .disabled(network.is_scanning_wifi)
        .on_click(|_, _, cx| cx.global::<AppState>().network.rescan_wifi());

    let rows = visible_networks(&network.wifi_ap_list)
        .into_iter()
        .enumerate()
        .map(|(index, access_point)| {
            if center
                .password
                .as_ref()
                .is_some_and(|prompt| prompt.ssid == access_point.ssid)
            {
                return password_row(center, theme, cx);
            }
            network_row(
                index,
                access_point,
                network.connecting_wifi_ssid.as_deref(),
                theme,
                cx,
            )
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap(Spacing::Small.rems())
        .child(heading(theme, label, Some(refresh)))
        .child(list("wifi-networks", rows))
        .children(error_line(theme, network.wifi_error.as_ref()))
        .into_any_element()
}

fn network_row(
    index: usize,
    access_point: WifiAccessPoint,
    connecting: Option<&str>,
    theme: &Theme,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    let ssid = access_point.ssid.clone();
    let secured = !access_point.security.is_empty() && access_point.security != "--";
    let status = if connecting == Some(ssid.as_str()) {
        "Connecting…".to_string()
    } else if access_point.is_connected {
        "Connected".to_string()
    } else if access_point.is_saved {
        "Saved".to_string()
    } else if secured {
        access_point.security.clone()
    } else {
        "Open".to_string()
    };
    let mut row = ListRow::new(ElementId::named_usize("wifi-network", index), ssid.clone())
        .icon(signal_icon(access_point.signal))
        .icon_color(if access_point.is_connected {
            theme.accent()
        } else {
            theme.text_muted()
        })
        .status(status)
        .status_accent(access_point.is_connected)
        .selected(access_point.is_connected);
    if access_point.is_connected {
        let disconnect = ssid.clone();
        row = row.action(
            IconButton::new(ElementId::named_usize("wifi-disconnect", index), "x.svg")
                .tooltip("Disconnect")
                .on_click(move |_, _, cx| {
                    cx.global::<AppState>().network.disconnect_wifi(&disconnect);
                }),
        );
    } else {
        let connect = ssid.clone();
        let needs_password = secured && !access_point.is_saved;
        row = row.on_click(cx.listener(move |center, _, _, cx| {
            if needs_password {
                center.password = Some(PasswordPrompt {
                    ssid: connect.clone(),
                    input: InputState::new("").with_masked(true),
                });
                cx.notify();
            } else {
                cx.global::<AppState>().network.connect_wifi(&connect, None);
                ShellState::refresh_now(cx);
            }
        }));
    }
    if access_point.is_saved {
        let forget = ssid;
        row = row.action(
            IconButton::new(ElementId::named_usize("wifi-forget", index), "trash.svg")
                .tooltip("Forget")
                .on_click(move |_, _, cx| cx.global::<AppState>().network.forget_wifi(&forget)),
        );
    }
    row.into_any_element()
}

fn password_row(
    center: &ControlCenter,
    theme: &Theme,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    let Some(prompt) = center.password.as_ref() else {
        return div().into_any_element();
    };
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap(Spacing::Small.rems())
        .p(Spacing::Small.rems())
        .rounded(Radius::Medium.rems())
        .bg(theme.element_selected())
        .child(
            div()
                .truncate()
                .text_size(TextSize::Small.pixels(theme.font_size))
                .child(prompt.ssid.clone()),
        )
        .child(
            Input::from_state("wifi-password", &prompt.input)
                .placeholder("Password")
                .masked(true)
                .focused(true)
                .w_full(),
        )
        .child(
            div()
                .flex()
                .justify_end()
                .gap(Spacing::Small.rems())
                .child(
                    Button::new("wifi-password-cancel")
                        .label("Cancel")
                        .variant(ButtonVariant::Ghost)
                        .on_click(cx.listener(|center, _, _, cx| {
                            center.password = None;
                            cx.notify();
                        })),
                )
                .child(
                    Button::new("wifi-password-connect")
                        .label("Connect")
                        .disabled(prompt.input.is_empty())
                        .on_click(cx.listener(|center, _, _, cx| center.submit_password(cx))),
                ),
        )
        .into_any_element()
}

fn bluetooth(snapshot: &Snapshot, theme: &Theme) -> AnyElement {
    let network = &snapshot.network;
    if !network.bluetooth_enabled {
        return caption(theme, "Bluetooth is off").into_any_element();
    }
    let scan = IconButton::new("bluetooth-scan", "refresh-cw.svg")
        .tooltip("Scan")
        .disabled(network.is_scanning_bluetooth)
        .on_click(|_, _, cx| cx.global::<AppState>().network.start_bluetooth_scan());
    let label = if network.is_scanning_bluetooth {
        "Scanning…"
    } else {
        "Devices"
    };
    let mut devices = network.bluetooth_device_list.clone();
    devices.sort_by(|left, right| {
        right
            .is_connected
            .cmp(&left.is_connected)
            .then(right.is_paired.cmp(&left.is_paired))
            .then(left.name.to_lowercase().cmp(&right.name.to_lowercase()))
    });
    let rows = devices
        .into_iter()
        .enumerate()
        .map(|(index, device)| {
            let connecting =
                network.connecting_bluetooth_mac.as_deref() == Some(device.mac.as_str());
            let status = if connecting {
                "Connecting…"
            } else if device.is_connected {
                "Connected"
            } else if device.is_paired {
                "Paired"
            } else {
                "Available"
            };
            let mut row = ListRow::new(
                ElementId::named_usize("bluetooth-device", index),
                device.name.clone(),
            )
            .icon(if device.is_connected {
                "bluetooth-connected.svg"
            } else {
                "bluetooth.svg"
            })
            .icon_color(if device.is_connected {
                theme.accent()
            } else {
                theme.text_muted()
            })
            .status(status)
            .status_accent(device.is_connected)
            .selected(device.is_connected);
            let mac = device.mac.clone();
            if device.is_connected {
                row = row.action(
                    IconButton::new(
                        ElementId::named_usize("bluetooth-disconnect", index),
                        "x.svg",
                    )
                    .tooltip("Disconnect")
                    .on_click(move |_, _, cx| {
                        cx.global::<AppState>().network.disconnect_bluetooth(&mac);
                    }),
                );
            } else if !connecting {
                row = row.on_click(move |_, _, cx| {
                    cx.global::<AppState>().network.connect_bluetooth(&mac);
                });
            }
            if device.is_paired {
                let mac = device.mac.clone();
                row = row.action(
                    IconButton::new(
                        ElementId::named_usize("bluetooth-forget", index),
                        "trash.svg",
                    )
                    .tooltip("Forget")
                    .on_click(move |_, _, cx| {
                        cx.global::<AppState>().network.forget_bluetooth(&mac);
                    }),
                );
            }
            row.into_any_element()
        })
        .collect();

    div()
        .flex()
        .flex_col()
        .w_full()
        .gap(Spacing::Small.rems())
        .child(heading(theme, label, Some(scan)))
        .child(list("bluetooth-devices", rows))
        .children(error_line(theme, network.bluetooth_error.as_ref()))
        .into_any_element()
}

fn manage_profiles() -> Button {
    Button::new("vpn-manage")
        .label("Manage profiles")
        .icon("sliders-horizontal.svg")
        .variant(ButtonVariant::Outline)
        .w_full(true)
        .on_click(|_, _, cx| {
            Shell::close(cx);
            VpnService::edit(None);
        })
}

fn vpn(snapshot: &Snapshot, theme: &Theme) -> AnyElement {
    let vpn = &snapshot.vpn;
    if vpn.profiles.is_empty() {
        return div()
            .flex()
            .flex_col()
            .w_full()
            .gap(Spacing::Small.rems())
            .child(caption(theme, "No VPN profiles yet"))
            .child(manage_profiles())
            .children(error_line(theme, vpn.error.as_ref()))
            .into_any_element();
    }
    let rows = vpn
        .profiles
        .iter()
        .enumerate()
        .map(|(index, profile)| {
            let up = profile.state == VpnState::Connected;
            let busy = profile.state == VpnState::Disconnecting;
            let uuid = profile.uuid.clone();
            let state = profile.state;
            let edit = profile.uuid.clone();
            ListRow::new(
                ElementId::named_usize("vpn-profile", index),
                profile.name.clone(),
            )
            .icon(if up { "lock.svg" } else { "lock-open.svg" })
            .icon_color(if up {
                theme.accent()
            } else {
                theme.text_muted()
            })
            .status(format!(
                "{} · {}",
                profile.kind.label(),
                profile.state.label()
            ))
            .status_accent(up)
            .selected(up)
            .disabled(busy)
            .on_click(move |_, _, cx| {
                let vpn = &cx.global::<AppState>().vpn;
                if state == VpnState::Off {
                    vpn.connect(&uuid);
                } else {
                    vpn.disconnect(&uuid);
                }
                ShellState::refresh_now(cx);
            })
            .action(
                IconButton::new(ElementId::named_usize("vpn-edit", index), "pencil.svg")
                    .tooltip("Edit profile")
                    .on_click(move |_, _, cx| {
                        Shell::close(cx);
                        VpnService::edit(Some(&edit));
                    }),
            )
            .into_any_element()
        })
        .collect();
    div()
        .flex()
        .flex_col()
        .w_full()
        .gap(Spacing::Small.rems())
        .child(list("vpn-profiles", rows))
        .children(error_line(theme, vpn.error.as_ref()))
        .child(manage_profiles())
        .into_any_element()
}

fn performance(snapshot: &Snapshot) -> AnyElement {
    let profiles = [
        PowerProfile::PowerSaver,
        PowerProfile::Balanced,
        PowerProfile::Performance,
    ];
    let selected = snapshot
        .power_profile
        .as_ref()
        .and_then(|current| profiles.iter().position(|profile| profile == current));
    profiles
        .iter()
        .fold(
            SegmentedControl::new("performance-profiles"),
            |control, profile| control.option(profile.display_name()),
        )
        .selected(selected)
        .disabled(snapshot.power_profile.is_none())
        .on_select(move |index, _, cx| {
            if let Some(profile) = profiles.get(index) {
                cx.global::<AppState>()
                    .power
                    .set_active_profile(profile.clone());
                ShellState::refresh_now(cx);
            }
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn access_point(ssid: &str, signal: u8, connected: bool, saved: bool) -> WifiAccessPoint {
        WifiAccessPoint {
            ssid: ssid.into(),
            signal,
            security: "WPA2".into(),
            is_connected: connected,
            is_saved: saved,
        }
    }

    #[test]
    fn networks_are_deduplicated_and_ordered() {
        let networks = vec![
            access_point("cafe", 40, false, false),
            access_point("home", 30, false, true),
            access_point("cafe", 70, false, false),
            access_point("", 90, false, false),
            access_point("office", 20, true, true),
        ];
        let visible = visible_networks(&networks);
        let names: Vec<_> = visible
            .iter()
            .map(|network| network.ssid.as_str())
            .collect();
        assert_eq!(names, ["office", "home", "cafe"]);
        assert_eq!(visible[2].signal, 70);
    }
}
