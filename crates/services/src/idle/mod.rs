mod wayland;

use crate::config::ConfigService;
use crate::ipc::{IpcCommand, push_ipc_command};
use crate::mpris::MprisService;
use std::sync::OnceLock;
use tokio::sync::{broadcast, watch};

#[derive(Clone, Debug)]
pub enum IdleEvent {
    IdleTimeoutTriggered,
}

#[derive(Clone)]
pub struct IdleService {
    tx: broadcast::Sender<IdleEvent>,
    config: ConfigService,
    mpris: MprisService,
}

impl IdleService {
    pub fn new(config: ConfigService, mpris: MprisService) -> Self {
        let _guard = crate::tokio_handle().enter();
        let (tx, _) = broadcast::channel(16);
        let service = Self { tx, config, mpris };

        let svc_dbus = service.clone();
        crate::spawn_tokio(async move {
            svc_dbus.start_dbus_lock_listener().await;
        });

        let svc_input = service.clone();
        crate::spawn_tokio(async move {
            svc_input.track_idleness().await;
        });

        service
    }

    pub fn subscribe(&self) -> broadcast::Receiver<IdleEvent> {
        self.tx.subscribe()
    }

    fn lock_for_idleness(&self) {
        crate::log_info!("IDLE", "Idle timeout reached, locking the session");
        push_ipc_command(IpcCommand::Lock);
        let _ = self.tx.send(IdleEvent::IdleTimeoutTriggered);
    }

    async fn track_idleness(&self) {
        loop {
            match wayland::watch(self.config.clone(), || self.lock_for_idleness()).await {
                Ok(()) => {}
                Err(wayland::IdleError::Unsupported) => {
                    crate::log_warn!(
                        "IDLE",
                        "The compositor lacks ext-idle-notify; falling back to input heuristics"
                    );
                    self.start_physical_input_tracker().await;
                    return;
                }
                Err(wayland::IdleError::Failed(error)) => {
                    crate::log_warn!("IDLE", "Idle notifications stopped: {error}");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }

    async fn start_physical_input_tracker(&self) {
        let (tx_activity, mut rx_activity) = tokio::sync::mpsc::unbounded_channel::<()>();

        crate::spawn_tokio(start_hyprland_socket2_listener(tx_activity));

        let mut last_activity = std::time::Instant::now();
        let mut last_cursor_pos = String::new();

        let hyprland = std::env::var_os("HYPRLAND_INSTANCE_SIGNATURE").is_some();
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;

            let timeout_secs = self.config.get().lockscreen.idle_timeout;
            if timeout_secs == 0 {
                while rx_activity.try_recv().is_ok() {}
                last_activity = std::time::Instant::now();
                continue;
            }
            let mut has_activity = false;

            while rx_activity.try_recv().is_ok() {
                has_activity = true;
            }

            if hyprland
                && let Ok(output) = tokio::process::Command::new("hyprctl")
                    .arg("cursorpos")
                    .output()
                    .await
                && output.status.success()
            {
                let pos_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                if !pos_str.is_empty() {
                    if last_cursor_pos.is_empty() {
                        last_cursor_pos = pos_str;
                    } else if last_cursor_pos != pos_str {
                        last_cursor_pos = pos_str;
                        has_activity = true;
                    }
                }
            }

            let is_video_playing = {
                let players = self.mpris.get_all_players();
                players.iter().any(|p| {
                    if !p.is_playing {
                        return false;
                    }
                    let bus = p.bus_name.to_lowercase();
                    let name = p.player_name.to_lowercase();
                    !bus.contains("spotify") && !name.contains("spotify")
                })
            };

            if has_activity || is_video_playing {
                last_activity = std::time::Instant::now();
            } else if timeout_secs > 0 && last_activity.elapsed().as_secs() >= timeout_secs {
                self.lock_for_idleness();
                last_activity = std::time::Instant::now();
            }
        }
    }

    async fn start_dbus_lock_listener(&self) {
        let Some(conn) = crate::dbus_util::get_shared_system_conn().await else {
            return;
        };

        let tx_sleep = self.tx.clone();
        let conn_sleep = conn.clone();
        crate::spawn_tokio(async move {
            lock_before_sleep(conn_sleep, tx_sleep).await;
        });

        let tx_lock = self.tx.clone();
        crate::spawn_tokio(async move {
            let Some(session) = current_session_path(&conn).await else {
                crate::log_warn!("IDLE", "No logind session found; lock requests are ignored");
                return;
            };
            if let Ok(proxy) = zbus::Proxy::new(
                &conn,
                LOGIND,
                session.as_str(),
                "org.freedesktop.login1.Session",
            )
            .await
                && let Ok(mut stream) = proxy.receive_signal("Lock").await
            {
                use futures::StreamExt;
                while stream.next().await.is_some() {
                    push_ipc_command(IpcCommand::Lock);
                    let _ = tx_lock.send(IdleEvent::IdleTimeoutTriggered);
                }
            }
        });
    }
}

const LOGIND: &str = "org.freedesktop.login1";
const LOCK_BEFORE_SLEEP_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(3);

static LOCK_GENERATION: OnceLock<watch::Sender<u64>> = OnceLock::new();

fn lock_generation() -> &'static watch::Sender<u64> {
    LOCK_GENERATION.get_or_init(|| watch::channel(0).0)
}

/// Called by the lock screen once its surfaces are up, so a pending suspend can proceed.
pub fn report_session_locked() {
    lock_generation().send_modify(|generation| *generation += 1);
}

async fn logind_manager(conn: &zbus::Connection) -> Option<zbus::Proxy<'static>> {
    zbus::Proxy::new(
        conn,
        LOGIND,
        "/org/freedesktop/login1",
        "org.freedesktop.login1.Manager",
    )
    .await
    .ok()
}

// A delay inhibitor keeps logind from suspending until the screen is locked.
async fn inhibit_sleep(manager: &zbus::Proxy<'_>) -> Option<zbus::zvariant::OwnedFd> {
    match manager
        .call(
            "Inhibit",
            &(
                "sleep",
                "Starshell",
                "Lock the screen before sleep",
                "delay",
            ),
        )
        .await
    {
        Ok(fd) => Some(fd),
        Err(error) => {
            crate::log_warn!("IDLE", "Failed to take the sleep inhibitor: {error}");
            None
        }
    }
}

async fn lock_before_sleep(conn: zbus::Connection, tx: broadcast::Sender<IdleEvent>) {
    use futures::StreamExt;

    let Some(manager) = logind_manager(&conn).await else {
        return;
    };
    let Ok(mut stream) = manager.receive_signal("PrepareForSleep").await else {
        return;
    };
    let mut inhibitor = inhibit_sleep(&manager).await;

    while let Some(message) = stream.next().await {
        let Ok(going_to_sleep) = message.body().deserialize::<bool>() else {
            continue;
        };
        if !going_to_sleep {
            if inhibitor.is_none() {
                inhibitor = inhibit_sleep(&manager).await;
            }
            continue;
        }

        let mut locked = lock_generation().subscribe();
        locked.borrow_and_update();
        push_ipc_command(IpcCommand::Lock);
        let _ = tx.send(IdleEvent::IdleTimeoutTriggered);
        if tokio::time::timeout(LOCK_BEFORE_SLEEP_TIMEOUT, locked.changed())
            .await
            .is_err()
        {
            crate::log_warn!("IDLE", "The screen was not locked in time before sleep");
        }
        inhibitor = None;
    }
}

async fn current_session_path(conn: &zbus::Connection) -> Option<zbus::zvariant::OwnedObjectPath> {
    let manager = logind_manager(conn).await?;
    if let Ok(id) = std::env::var("XDG_SESSION_ID")
        && let Ok(path) = manager.call("GetSession", &(id.as_str(),)).await
    {
        return Some(path);
    }
    manager
        .call("GetSessionByPID", &(std::process::id(),))
        .await
        .ok()
}

async fn start_hyprland_socket2_listener(tx_activity: tokio::sync::mpsc::UnboundedSender<()>) {
    let signature = match std::env::var("HYPRLAND_INSTANCE_SIGNATURE") {
        Ok(s) if !s.is_empty() => s,
        _ => return,
    };

    let socket_path = if let Ok(runtime_dir) = std::env::var("XDG_RUNTIME_DIR") {
        format!("{runtime_dir}/hypr/{signature}/.socket2.sock")
    } else {
        format!("/tmp/hypr/{signature}/.socket2.sock")
    };

    if let Ok(stream) = tokio::net::UnixStream::connect(&socket_path).await {
        use tokio::io::AsyncBufReadExt;
        let mut reader = tokio::io::BufReader::new(stream).lines();
        while let Ok(Some(_line)) = reader.next_line().await {
            let _ = tx_activity.send(());
        }
    }
}
