use gpui::{AnyElement, Context, IntoElement};
use services::{AppState, PowerProfile, VpnState};
use ui::components::Tile;

use super::{ControlCenter, Expanded};
use crate::shell::state::{ShellState, Snapshot};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TileKind {
    Wifi,
    Bluetooth,
    Vpn,
    Performance,
    Microphone,
    Camera,
}

pub const ROWS: [[TileKind; 2]; 3] = [
    [TileKind::Wifi, TileKind::Bluetooth],
    [TileKind::Vpn, TileKind::Performance],
    [TileKind::Microphone, TileKind::Camera],
];

impl TileKind {
    pub fn panel(self) -> Option<Expanded> {
        match self {
            Self::Wifi => Some(Expanded::Wifi),
            Self::Bluetooth => Some(Expanded::Bluetooth),
            Self::Vpn => Some(Expanded::Vpn),
            Self::Performance => Some(Expanded::Performance),
            Self::Microphone | Self::Camera => None,
        }
    }
}

pub struct TileState {
    pub icon: &'static str,
    pub title: &'static str,
    pub status: String,
    pub active: bool,
}

pub fn next_profile(profile: &PowerProfile) -> PowerProfile {
    match profile {
        PowerProfile::PowerSaver => PowerProfile::Balanced,
        PowerProfile::Balanced => PowerProfile::Performance,
        PowerProfile::Performance => PowerProfile::PowerSaver,
    }
}

pub fn state(kind: TileKind, snapshot: &Snapshot) -> TileState {
    match kind {
        TileKind::Wifi => {
            let network = &snapshot.network;
            TileState {
                icon: crate::bar::network_icon(network),
                title: "Wi-Fi",
                status: if !network.wifi_enabled {
                    "Off".to_string()
                } else if network.wifi_ssid.is_empty() {
                    "Not connected".to_string()
                } else {
                    network.wifi_ssid.clone()
                },
                active: network.wifi_enabled,
            }
        }
        TileKind::Bluetooth => {
            let network = &snapshot.network;
            let connected: Vec<_> = network
                .bluetooth_device_list
                .iter()
                .filter(|device| device.is_connected)
                .collect();
            TileState {
                icon: if !network.bluetooth_enabled {
                    "bluetooth-off.svg"
                } else if connected.is_empty() {
                    "bluetooth.svg"
                } else {
                    "bluetooth-connected.svg"
                },
                title: "Bluetooth",
                status: if !network.bluetooth_enabled {
                    "Off".to_string()
                } else {
                    match connected.as_slice() {
                        [] => "No devices".to_string(),
                        [device] => device.name.clone(),
                        devices => format!("{} devices", devices.len()),
                    }
                },
                active: network.bluetooth_enabled,
            }
        }
        TileKind::Vpn => {
            let vpn = &snapshot.vpn;
            let active = vpn.active();
            TileState {
                icon: if active.is_some_and(|profile| profile.state == VpnState::Connected) {
                    "lock.svg"
                } else {
                    "lock-open.svg"
                },
                title: "VPN",
                status: match active {
                    _ if vpn.profiles.is_empty() => "No profiles".to_string(),
                    Some(profile) if profile.state == VpnState::Connected => profile.name.clone(),
                    Some(profile) if profile.state == VpnState::Connecting => {
                        "Connecting…".to_string()
                    }
                    _ => "Off".to_string(),
                },
                active: active.is_some_and(|profile| profile.state == VpnState::Connected),
            }
        }
        TileKind::Performance => TileState {
            icon: "gauge.svg",
            title: "Performance",
            status: snapshot
                .power_profile
                .as_ref()
                .map(|profile| profile.display_name().to_string())
                .unwrap_or_else(|| "Unavailable".to_string()),
            active: snapshot
                .power_profile
                .as_ref()
                .is_some_and(|profile| *profile != PowerProfile::Balanced),
        },
        TileKind::Microphone => {
            let audio = &snapshot.audio;
            let available = !audio.audio_sources.is_empty();
            TileState {
                icon: if audio.is_input_muted {
                    "mic-off.svg"
                } else {
                    "mic.svg"
                },
                title: "Microphone",
                status: if !available {
                    "Unavailable"
                } else if audio.is_input_muted {
                    "Muted"
                } else {
                    "On"
                }
                .to_string(),
                active: available && !audio.is_input_muted,
            }
        }
        TileKind::Camera => TileState {
            icon: "camera.svg",
            title: "Camera",
            status: if snapshot.capture.camera_in_use {
                "In use"
            } else {
                "Idle"
            }
            .to_string(),
            active: snapshot.capture.camera_in_use,
        },
    }
}

pub fn tile(
    kind: TileKind,
    snapshot: &Snapshot,
    expanded: Option<Expanded>,
    cx: &mut Context<ControlCenter>,
) -> AnyElement {
    let state = state(kind, snapshot);
    let id = gpui::ElementId::Name(format!("tile-{kind:?}").into());
    let mut tile = Tile::new(id, state.icon, state.title)
        .status(state.status)
        .active(state.active)
        .expanded(kind.panel().is_some() && kind.panel() == expanded);
    if let Some(panel) = kind.panel() {
        tile = tile.on_expand(cx.listener(move |this, _, _, cx| this.toggle_expanded(panel, cx)));
    }
    tile = match kind {
        TileKind::Wifi => tile.on_toggle(cx.listener(|_, _, _, cx| {
            cx.global::<AppState>().network.toggle_wifi();
            ShellState::refresh_now(cx);
        })),
        TileKind::Bluetooth => tile.on_toggle(cx.listener(|_, _, _, cx| {
            cx.global::<AppState>().network.toggle_bluetooth();
            ShellState::refresh_now(cx);
        })),
        TileKind::Vpn if !snapshot.vpn.profiles.is_empty() => {
            tile.on_toggle(cx.listener(|_, _, _, cx| {
                cx.global::<AppState>().vpn.toggle();
                ShellState::refresh_now(cx);
            }))
        }
        TileKind::Performance if snapshot.power_profile.is_some() => {
            tile.on_toggle(cx.listener(|_, _, _, cx| {
                let power = &cx.global::<AppState>().power;
                power.set_active_profile(next_profile(&power.get_active_profile()));
                ShellState::refresh_now(cx);
            }))
        }
        TileKind::Microphone if !snapshot.audio.audio_sources.is_empty() => {
            tile.on_toggle(cx.listener(|_, _, _, cx| {
                let system = cx.global::<AppState>().system.clone();
                services::spawn_tokio(async move {
                    let _ = system.toggle_input_mute().await;
                });
            }))
        }
        _ => tile,
    };
    tile.into_any_element()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot() -> Snapshot {
        Snapshot {
            time: String::new(),
            date: String::new(),
            current_workspace: Default::default(),
            workspaces: Vec::new(),
            network: Default::default(),
            vpn: Default::default(),
            battery: None,
            power_profile: None,
            audio: Default::default(),
            capture: Default::default(),
            displays: Default::default(),
            players: Default::default(),
            notification_count: 0,
            do_not_disturb: false,
            tray: Vec::new(),
            record: services::RecordStatus::Stopped,
            record_seconds: 0,
            shelf_count: 0,
            keyboard_layout: None,
        }
    }

    #[test]
    fn statuses_follow_the_specification_table() {
        let mut snapshot = snapshot();
        assert_eq!(state(TileKind::Wifi, &snapshot).status, "Off");
        assert_eq!(state(TileKind::Bluetooth, &snapshot).status, "Off");
        assert_eq!(state(TileKind::Vpn, &snapshot).status, "No profiles");
        assert_eq!(
            state(TileKind::Performance, &snapshot).status,
            "Unavailable"
        );
        assert_eq!(state(TileKind::Microphone, &snapshot).status, "Unavailable");
        assert_eq!(state(TileKind::Camera, &snapshot).status, "Idle");

        snapshot.network.wifi_enabled = true;
        assert_eq!(state(TileKind::Wifi, &snapshot).status, "Not connected");
        snapshot.network.wifi_ssid = "Home".into();
        assert_eq!(state(TileKind::Wifi, &snapshot).status, "Home");
        assert!(state(TileKind::Wifi, &snapshot).active);

        snapshot.power_profile = Some(PowerProfile::Balanced);
        assert!(!state(TileKind::Performance, &snapshot).active);
        snapshot.power_profile = Some(PowerProfile::PowerSaver);
        assert_eq!(state(TileKind::Performance, &snapshot).status, "Saver");
        assert!(state(TileKind::Performance, &snapshot).active);
    }

    #[test]
    fn bluetooth_status_counts_connected_devices() {
        let mut snapshot = snapshot();
        snapshot.network.bluetooth_enabled = true;
        assert_eq!(state(TileKind::Bluetooth, &snapshot).status, "No devices");
        let device = |name: &str| services::BluetoothDeviceItem {
            mac: name.into(),
            name: name.into(),
            is_connected: true,
            is_paired: true,
        };
        snapshot.network.bluetooth_device_list = vec![device("Buds")];
        assert_eq!(state(TileKind::Bluetooth, &snapshot).status, "Buds");
        snapshot.network.bluetooth_device_list.push(device("Mouse"));
        assert_eq!(state(TileKind::Bluetooth, &snapshot).status, "2 devices");
    }

    #[test]
    fn performance_cycles_through_every_profile() {
        assert_eq!(
            next_profile(&PowerProfile::PowerSaver),
            PowerProfile::Balanced
        );
        assert_eq!(
            next_profile(&PowerProfile::Balanced),
            PowerProfile::Performance
        );
        assert_eq!(
            next_profile(&PowerProfile::Performance),
            PowerProfile::PowerSaver
        );
    }
}
