use std::sync::Arc;
use std::time::Duration;

use arc_swap::ArcSwap;
use tokio::process::Command;
use tokio::sync::Notify;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnKind {
    WireGuard,
    Vpn,
}

impl VpnKind {
    fn from_type(kind: &str) -> Option<Self> {
        match kind {
            "wireguard" => Some(Self::WireGuard),
            "vpn" => Some(Self::Vpn),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::WireGuard => "WireGuard",
            Self::Vpn => "VPN",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VpnState {
    Off,
    Connecting,
    Connected,
    Disconnecting,
}

impl VpnState {
    fn from_active_state(state: &str) -> Self {
        match state {
            "activated" => Self::Connected,
            "activating" => Self::Connecting,
            "deactivating" => Self::Disconnecting,
            _ => Self::Off,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "off",
            Self::Connecting => "connecting…",
            Self::Connected => "connected",
            Self::Disconnecting => "disconnecting…",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VpnProfile {
    pub name: String,
    pub uuid: String,
    pub kind: VpnKind,
    pub state: VpnState,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct VpnStatus {
    pub profiles: Vec<VpnProfile>,
    pub error: Option<String>,
}

impl VpnStatus {
    pub fn active(&self) -> Option<&VpnProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.state != VpnState::Off)
    }
}

#[derive(Clone)]
pub struct VpnService {
    status: Arc<ArcSwap<VpnStatus>>,
    refresh: Arc<Notify>,
}

impl Default for VpnService {
    fn default() -> Self {
        Self::new()
    }
}

impl VpnService {
    pub fn new() -> Self {
        let _guard = crate::tokio_handle().enter();
        let service = Self {
            status: Arc::new(ArcSwap::from_pointee(VpnStatus::default())),
            refresh: Arc::new(Notify::new()),
        };
        let worker = service.clone();
        crate::spawn_tokio(async move {
            loop {
                worker.reload().await;
                tokio::select! {
                    _ = worker.refresh.notified() => {}
                    _ = tokio::time::sleep(Duration::from_secs(3)) => {}
                }
            }
        });
        service
    }

    pub fn get_status(&self) -> VpnStatus {
        (**self.status.load()).clone()
    }

    pub fn connect(&self, uuid: &str) {
        self.run(
            ["connection", "up", "uuid", uuid],
            uuid,
            VpnState::Connecting,
        );
    }

    pub fn disconnect(&self, uuid: &str) {
        self.run(
            ["connection", "down", "uuid", uuid],
            uuid,
            VpnState::Disconnecting,
        );
    }

    pub fn toggle(&self) {
        let status = self.get_status();
        if let Some(active) = status.active() {
            self.disconnect(&active.uuid);
        } else if let Some(first) = status.profiles.first() {
            self.connect(&first.uuid);
        }
    }

    pub fn edit(uuid: Option<&str>) {
        let mut command = std::process::Command::new("nm-connection-editor");
        if let Some(uuid) = uuid {
            command.args(["--edit", uuid]);
        }
        if let Err(error) = command.spawn() {
            crate::log_error!("VPN", "Could not open the connection editor: {error}");
        }
    }

    fn run(&self, args: [&str; 4], uuid: &str, pending: VpnState) {
        let name = self.mark(uuid, pending);
        let service = self.clone();
        let args = args.map(str::to_string);
        crate::spawn_tokio(async move {
            let output = tokio::time::timeout(
                Duration::from_secs(90),
                Command::new("nmcli")
                    .args(&args)
                    .env("LC_ALL", "C")
                    .output(),
            )
            .await;
            let error = match output {
                Ok(Ok(output)) if output.status.success() => None,
                Ok(Ok(output)) => Some(clean_error(&String::from_utf8_lossy(&output.stderr))),
                Ok(Err(error)) => Some(error.to_string()),
                Err(_) => Some("timed out".to_string()),
            };
            service.reload().await;
            if let Some(error) = error {
                let mut status = service.get_status();
                status.error = Some(format!("{name}: {error}"));
                service.status.store(Arc::new(status));
            }
        });
    }

    fn mark(&self, uuid: &str, state: VpnState) -> String {
        let mut status = self.get_status();
        status.error = None;
        let mut name = uuid.to_string();
        for profile in &mut status.profiles {
            if profile.uuid == uuid {
                profile.state = state;
                name = profile.name.clone();
            }
        }
        self.status.store(Arc::new(status));
        name
    }

    async fn reload(&self) {
        let (saved, active) = tokio::join!(
            nmcli(&["-t", "-f", "NAME,UUID,TYPE", "connection", "show"]),
            nmcli(&["-t", "-f", "UUID,STATE", "connection", "show", "--active"]),
        );
        let Some(saved) = saved else {
            return;
        };
        let mut profiles = parse_profiles(&saved, active.as_deref().unwrap_or_default());
        let previous = self.get_status();
        for profile in &mut profiles {
            let pending = previous
                .profiles
                .iter()
                .find(|old| old.uuid == profile.uuid)
                .map(|old| old.state);
            if profile.state == VpnState::Off && pending == Some(VpnState::Connecting) {
                profile.state = VpnState::Connecting;
            }
        }
        let status = VpnStatus {
            profiles,
            error: previous.error.clone(),
        };
        if status != previous {
            self.status.store(Arc::new(status));
        }
    }
}

async fn nmcli(args: &[&str]) -> Option<String> {
    let output = tokio::time::timeout(
        Duration::from_secs(3),
        Command::new("nmcli").args(args).env("LC_ALL", "C").output(),
    )
    .await
    .ok()?
    .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).into_owned())
}

fn split_terse(line: &str) -> Vec<String> {
    let mut fields = vec![String::new()];
    let mut escaped = false;
    for character in line.chars() {
        match (escaped, character) {
            (false, '\\') => escaped = true,
            (false, ':') => fields.push(String::new()),
            (_, character) => {
                escaped = false;
                if let Some(field) = fields.last_mut() {
                    field.push(character);
                }
            }
        }
    }
    fields
}

fn parse_profiles(saved: &str, active: &str) -> Vec<VpnProfile> {
    let active_states: Vec<(String, VpnState)> = active
        .lines()
        .filter_map(|line| {
            let fields = split_terse(line);
            Some((
                fields.first()?.clone(),
                VpnState::from_active_state(fields.get(1)?),
            ))
        })
        .collect();
    let mut profiles: Vec<VpnProfile> = saved
        .lines()
        .filter_map(|line| {
            let fields = split_terse(line);
            let kind = VpnKind::from_type(fields.get(2)?)?;
            let uuid = fields.get(1)?.clone();
            let state = active_states
                .iter()
                .find(|(active_uuid, _)| *active_uuid == uuid)
                .map(|(_, state)| *state)
                .unwrap_or(VpnState::Off);
            Some(VpnProfile {
                name: fields.first()?.clone(),
                uuid,
                kind,
                state,
            })
        })
        .collect();
    profiles.sort_by_key(|left| left.name.to_lowercase());
    profiles
}

fn clean_error(stderr: &str) -> String {
    let line = stderr
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or("failed");
    line.trim_start_matches("Error:")
        .trim()
        .trim_end_matches('.')
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_vpn_and_wireguard_profiles_are_listed() {
        let saved = "Home:1:802-11-wireless\nwork:2:vpn\nMac\\:book:3:wireguard\nlo:4:loopback\n";
        let active = "3:activated\n1:activated\n";
        let profiles = parse_profiles(saved, active);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].name, "Mac:book");
        assert_eq!(profiles[0].kind, VpnKind::WireGuard);
        assert_eq!(profiles[0].state, VpnState::Connected);
        assert_eq!(profiles[1].kind, VpnKind::Vpn);
        assert_eq!(profiles[1].state, VpnState::Off);
    }

    #[test]
    fn active_profile_is_the_first_not_off() {
        let status = VpnStatus {
            profiles: parse_profiles("a:1:vpn\nb:2:vpn\n", "2:activating\n"),
            error: None,
        };
        assert_eq!(
            status.active().map(|profile| profile.uuid.as_str()),
            Some("2")
        );
    }

    #[test]
    fn errors_are_reduced_to_their_reason() {
        assert_eq!(
            clean_error(
                "Error: Connection activation failed: No valid secrets.\nHint: use journalctl"
            ),
            "Connection activation failed: No valid secrets"
        );
    }
}
