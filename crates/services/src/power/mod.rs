use crate::dbus_util::get_shared_system_conn;
use arc_swap::ArcSwap;
use std::sync::Arc;
use tokio::time::Duration;
use zbus::fdo::PropertiesProxy;
use zbus::zvariant::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BatteryStatus {
    pub percentage: u8,
    pub is_charging: bool,
    pub is_full: bool,
    pub seconds_remaining: Option<u64>,
}

impl BatteryStatus {
    pub const LOW_PERCENTAGE: u8 = 20;
    pub const CRITICAL_PERCENTAGE: u8 = 7;

    pub fn is_low(&self) -> bool {
        !self.is_charging && self.percentage <= Self::LOW_PERCENTAGE
    }

    pub fn is_critical(&self) -> bool {
        !self.is_charging && self.percentage <= Self::CRITICAL_PERCENTAGE
    }

    pub fn remaining_label(&self) -> Option<String> {
        let seconds = self.seconds_remaining.filter(|seconds| *seconds > 0)?;
        let hours = seconds / 3600;
        let minutes = (seconds % 3600) / 60;
        let duration = if hours > 0 {
            format!("{hours}h {minutes}m")
        } else {
            format!("{minutes}m")
        };
        Some(if self.is_charging {
            format!("≈ {duration} until full")
        } else {
            format!("≈ {duration} left")
        })
    }
}

fn parse_battery_uevent(content: &str) -> Option<BatteryStatus> {
    let mut values = std::collections::HashMap::new();
    for line in content.lines() {
        if let Some((key, value)) = line.split_once('=') {
            values.insert(key.trim_start_matches("POWER_SUPPLY_"), value.trim());
        }
    }
    if values.get("TYPE").copied() != Some("Battery") || values.get("PRESENT").copied() == Some("0")
    {
        return None;
    }
    let number = |key: &str| values.get(key).and_then(|value| value.parse::<i64>().ok());
    let percentage = number("CAPACITY")?.clamp(0, 100) as u8;
    let status = values.get("STATUS").copied().unwrap_or_default();
    let is_charging = status == "Charging";
    let is_full = status == "Full";
    let rate = number("CURRENT_NOW")
        .map(i64::abs)
        .filter(|rate| *rate > 0)
        .zip(number("CHARGE_NOW").zip(number("CHARGE_FULL")))
        .or_else(|| {
            number("POWER_NOW")
                .map(i64::abs)
                .filter(|rate| *rate > 0)
                .zip(number("ENERGY_NOW").zip(number("ENERGY_FULL")))
        });
    let estimated = rate.map(|(rate, (now, full))| {
        let amount = if is_charging { full - now } else { now };
        (amount.max(0) as f64 / rate as f64 * 3600.0) as u64
    });
    let reported = if is_charging {
        number("TIME_TO_FULL_NOW")
    } else {
        number("TIME_TO_EMPTY_NOW")
    }
    .filter(|seconds| *seconds > 0)
    .map(|seconds| seconds as u64);
    Some(BatteryStatus {
        percentage,
        is_charging,
        is_full,
        seconds_remaining: if is_full {
            None
        } else {
            reported.or(estimated)
        },
    })
}

fn read_system_battery() -> Option<BatteryStatus> {
    let entries = std::fs::read_dir("/sys/class/power_supply").ok()?;
    entries.flatten().find_map(|entry| {
        let content = std::fs::read_to_string(entry.path().join("uevent")).ok()?;
        parse_battery_uevent(&content)
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PowerProfile {
    Performance,
    Balanced,
    PowerSaver,
}

impl PowerProfile {
    pub fn as_str(&self) -> &'static str {
        match self {
            PowerProfile::Performance => "performance",
            PowerProfile::Balanced => "balanced",
            PowerProfile::PowerSaver => "power-saver",
        }
    }

    pub fn parse(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "performance" => PowerProfile::Performance,
            "power-saver" => PowerProfile::PowerSaver,
            _ => PowerProfile::Balanced,
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            PowerProfile::Performance => "Performance",
            PowerProfile::Balanced => "Balanced",
            PowerProfile::PowerSaver => "Saver",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            PowerProfile::Performance => "zap.svg",
            PowerProfile::Balanced => "scale.svg",
            PowerProfile::PowerSaver => "leaf.svg",
        }
    }
}

#[derive(Clone)]
pub struct PowerService {
    profiles_available: Arc<std::sync::atomic::AtomicBool>,
    active_profile: Arc<ArcSwap<PowerProfile>>,
    battery: Arc<ArcSwap<Option<BatteryStatus>>>,
}

impl Default for PowerService {
    fn default() -> Self {
        Self::new()
    }
}

impl PowerService {
    pub fn new() -> Self {
        let _guard = crate::tokio_handle().enter();
        let service = Self {
            profiles_available: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            active_profile: Arc::new(ArcSwap::from_pointee(PowerProfile::Balanced)),
            battery: Arc::new(ArcSwap::from_pointee(read_system_battery())),
        };

        let svc_clone = service.clone();
        crate::spawn_tokio(async move {
            svc_clone.start_polling().await;
        });

        service
    }

    pub fn profiles_available(&self) -> bool {
        self.profiles_available
            .load(std::sync::atomic::Ordering::Relaxed)
    }

    pub fn get_active_profile(&self) -> PowerProfile {
        (**self.active_profile.load()).clone()
    }

    pub fn get_battery(&self) -> Option<BatteryStatus> {
        **self.battery.load()
    }

    pub fn set_active_profile(&self, profile: PowerProfile) {
        self.active_profile.store(Arc::new(profile.clone()));

        crate::spawn_tokio(async move {
            if let Some(conn) = get_shared_system_conn().await
                && let Ok(builder) =
                    PropertiesProxy::builder(&conn).destination("net.hadess.PowerProfiles")
                && let Ok(builder) = builder.path("/net/hadess/PowerProfiles")
                && let Ok(proxy) = builder.build().await
                && let Ok(iface) = "net.hadess.PowerProfiles".try_into()
            {
                let _ = proxy
                    .set(iface, "ActiveProfile", Value::from(profile.as_str()))
                    .await;
            }
        });
    }

    async fn start_polling(&self) {
        loop {
            self.battery.store(Arc::new(read_system_battery()));
            if let Some(conn) = get_shared_system_conn().await
                && let Ok(builder) =
                    PropertiesProxy::builder(&conn).destination("net.hadess.PowerProfiles")
                && let Ok(builder) = builder.path("/net/hadess/PowerProfiles")
                && let Ok(proxy) = builder.build().await
                && let Ok(iface) = "net.hadess.PowerProfiles".try_into()
            {
                let profile = proxy
                    .get(iface, "ActiveProfile")
                    .await
                    .ok()
                    .and_then(|value| String::try_from(value).ok());
                self.profiles_available
                    .store(profile.is_some(), std::sync::atomic::Ordering::Relaxed);
                if let Some(profile) = profile {
                    self.active_profile
                        .store(Arc::new(PowerProfile::parse(&profile)));
                }
            }
            tokio::time::sleep(Duration::from_secs(3)).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISCHARGING: &str = "POWER_SUPPLY_TYPE=Battery\nPOWER_SUPPLY_STATUS=Discharging\nPOWER_SUPPLY_PRESENT=1\nPOWER_SUPPLY_CURRENT_NOW=-1000000\nPOWER_SUPPLY_CHARGE_FULL=4000000\nPOWER_SUPPLY_CHARGE_NOW=3000000\nPOWER_SUPPLY_CAPACITY=75\n";

    #[test]
    fn discharging_battery_estimates_time_left() {
        let battery = parse_battery_uevent(DISCHARGING).expect("battery");
        assert_eq!(battery.percentage, 75);
        assert!(!battery.is_charging);
        assert_eq!(battery.seconds_remaining, Some(3 * 3600));
        assert_eq!(battery.remaining_label().as_deref(), Some("≈ 3h 0m left"));
    }

    #[test]
    fn charging_battery_estimates_time_until_full() {
        let content = DISCHARGING.replace("Discharging", "Charging");
        let battery = parse_battery_uevent(&content).expect("battery");
        assert!(battery.is_charging);
        assert_eq!(battery.seconds_remaining, Some(3600));
        assert_eq!(
            battery.remaining_label().as_deref(),
            Some("≈ 1h 0m until full")
        );
    }

    #[test]
    fn mains_supplies_are_not_batteries() {
        assert!(parse_battery_uevent("POWER_SUPPLY_TYPE=Mains\nPOWER_SUPPLY_ONLINE=1\n").is_none());
    }

    #[test]
    fn thresholds_follow_charging_state() {
        let mut battery = parse_battery_uevent(DISCHARGING).expect("battery");
        battery.percentage = 5;
        assert!(battery.is_low() && battery.is_critical());
        battery.is_charging = true;
        assert!(!battery.is_low() && !battery.is_critical());
    }
}
