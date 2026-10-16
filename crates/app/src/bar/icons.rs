use services::{BatteryStatus, NetworkStatus};

pub fn battery_icon(battery: &BatteryStatus) -> &'static str {
    if battery.is_charging {
        "battery-charging.svg"
    } else if battery.is_critical() {
        "battery-warning.svg"
    } else if battery.percentage <= 25 {
        "battery-low.svg"
    } else if battery.percentage <= 70 {
        "battery-medium.svg"
    } else {
        "battery-full.svg"
    }
}

pub fn network_icon(network: &NetworkStatus) -> &'static str {
    if network.ethernet_connected {
        "ethernet.svg"
    } else if !network.wifi_enabled {
        "wifi-off.svg"
    } else if network.wifi_ssid.is_empty() {
        "wifi-zero.svg"
    } else if network.wifi_signal >= 66 {
        "wifi.svg"
    } else if network.wifi_signal >= 33 {
        "wifi-high.svg"
    } else {
        "wifi-low.svg"
    }
}

pub fn volume_icon(volume: u32, muted: bool) -> &'static str {
    if muted || volume == 0 {
        "volume-x.svg"
    } else if volume < 34 {
        "volume.svg"
    } else if volume < 67 {
        "volume-1.svg"
    } else {
        "volume-2.svg"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icons_follow_device_state() {
        let mut network = NetworkStatus::default();
        assert_eq!(network_icon(&network), "wifi-off.svg");
        network.wifi_enabled = true;
        assert_eq!(network_icon(&network), "wifi-zero.svg");
        network.wifi_ssid = "home".into();
        network.wifi_signal = 80;
        assert_eq!(network_icon(&network), "wifi.svg");
        assert_eq!(volume_icon(50, true), "volume-x.svg");
        assert_eq!(volume_icon(50, false), "volume-1.svg");
        let battery = BatteryStatus {
            percentage: 5,
            is_charging: false,
            is_full: false,
            seconds_remaining: None,
        };
        assert_eq!(battery_icon(&battery), "battery-warning.svg");
    }
}
