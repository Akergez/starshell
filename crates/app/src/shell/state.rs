use std::sync::Arc;
use std::time::Duration;

use chrono::Local;
use gpui::{App, AppContext, Context, Entity, Global, Task};
use services::{
    AppState, BatteryStatus, CaptureStatus, Display, MediaTrack, NetworkStatus, NotificationStore,
    PowerProfile, RecordStatus, SniItem, SystemStatus, VpnStatus, WorkspaceInfo,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Snapshot {
    pub time: String,
    pub date: String,
    pub current_workspace: WorkspaceInfo,
    pub workspaces: Vec<WorkspaceInfo>,
    pub network: NetworkStatus,
    pub vpn: VpnStatus,
    pub battery: Option<BatteryStatus>,
    pub power_profile: Option<PowerProfile>,
    pub audio: SystemStatus,
    pub capture: CaptureStatus,
    pub displays: Arc<Vec<Display>>,
    pub players: Arc<Vec<MediaTrack>>,
    pub notification_count: usize,
    pub do_not_disturb: bool,
    pub tray: Vec<SniItem>,
    pub record: RecordStatus,
    pub record_seconds: u64,
    pub shelf_count: usize,
}

impl Snapshot {
    fn read(cx: &App) -> Self {
        let state = cx.global::<AppState>();
        let config = state.config.get();
        let now = Local::now();
        let notifications = NotificationStore::global();
        let record = state.record.get_status();
        Self {
            time: now.format(config.ui.clock_format()).to_string(),
            date: now.format("%a %e %b").to_string(),
            current_workspace: state.compositor.get_workspace(),
            workspaces: state.compositor.get_workspaces(),
            network: state.network.get_status(),
            vpn: state.vpn.get_status(),
            battery: state.power.get_battery(),
            power_profile: state
                .power
                .profiles_available()
                .then(|| state.power.get_active_profile()),
            audio: state.system.get_status(),
            capture: state.system.get_capture_status(),
            displays: state.displays.get_displays(),
            players: state.mpris.get_all_players(),
            notification_count: notifications.get_all_notifications().len(),
            do_not_disturb: notifications.is_dnd_enabled(),
            tray: state.sni_host.get_items(),
            record_seconds: if record == RecordStatus::Stopped {
                0
            } else {
                state.record.get_duration_secs()
            },
            record,
            shelf_count: state.shelf.count(),
        }
    }

    pub fn player(&self) -> Option<&MediaTrack> {
        self.players
            .iter()
            .find(|player| player.is_playing)
            .or_else(|| self.players.iter().find(|player| player.has_media))
    }
}

pub struct ShellState {
    pub snapshot: Snapshot,
    _poll: Task<()>,
    _events: Task<()>,
}

struct GlobalShellState(Entity<ShellState>);

impl Global for GlobalShellState {}

impl ShellState {
    pub fn init(cx: &mut App) -> Entity<Self> {
        let state = cx.new(|cx| {
            let poll = cx.spawn(async move |this, cx| {
                loop {
                    cx.background_executor()
                        .timer(Duration::from_millis(500))
                        .await;
                    if this
                        .update(cx, |state: &mut ShellState, cx| state.refresh(cx))
                        .is_err()
                    {
                        break;
                    }
                }
            });
            let events = Self::listen_for_events(cx);
            Self {
                snapshot: Snapshot::read(cx),
                _poll: poll,
                _events: events,
            }
        });
        cx.set_global(GlobalShellState(state.clone()));
        state
    }

    pub fn global(cx: &App) -> Entity<Self> {
        cx.global::<GlobalShellState>().0.clone()
    }

    pub fn snapshot(cx: &App) -> Snapshot {
        Self::global(cx).read(cx).snapshot.clone()
    }

    pub fn refresh_now(cx: &mut App) {
        Self::global(cx).update(cx, |state, cx| state.refresh(cx));
    }

    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        let snapshot = Snapshot::read(cx);
        if snapshot != self.snapshot {
            self.snapshot = snapshot;
            cx.notify();
        }
    }

    fn listen_for_events(cx: &mut Context<Self>) -> Task<()> {
        let state = cx.global::<AppState>();
        let audio_changed = state.system.audio_changed();
        let mut workspace_changes = state.compositor.on_change_workspace();
        let mut notifications = NotificationStore::global().subscribe();
        let (sender, mut receiver) = tokio::sync::mpsc::unbounded_channel::<()>();
        let _guard = services::tokio_handle().enter();
        services::spawn_tokio(async move {
            loop {
                tokio::select! {
                    _ = audio_changed.notified() => {}
                    change = workspace_changes.recv() => {
                        if matches!(change, Err(tokio::sync::broadcast::error::RecvError::Closed)) {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                    changed = notifications.changed() => {
                        if changed.is_err() {
                            tokio::time::sleep(Duration::from_secs(1)).await;
                        }
                    }
                }
                if sender.send(()).is_err() {
                    break;
                }
            }
        });
        cx.spawn(async move |this, cx| {
            while receiver.recv().await.is_some() {
                while receiver.try_recv().is_ok() {}
                if this.update(cx, |state, cx| state.refresh(cx)).is_err() {
                    break;
                }
            }
        })
    }
}
