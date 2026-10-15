use crate::compositor::{Compositor, WorkspaceInfo};
use arc_swap::ArcSwap;
use niri_ipc::{Request, Response, socket::Socket};
use std::panic::catch_unwind;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Default)]
pub struct Niri;

impl Niri {
    pub fn new() -> Self {
        Self
    }

    pub fn parse_workspace(workspaces: &[niri_ipc::Workspace]) -> Option<WorkspaceInfo> {
        let ws = workspaces
            .iter()
            .find(|w| w.is_focused)
            .or_else(|| workspaces.iter().find(|w| w.is_active))
            .or_else(|| workspaces.first())?;

        let name = match &ws.name {
            Some(n) => n.clone(),
            None => ws.idx.to_string(),
        };

        let (is_special, special_name) = match ws.name.as_deref() {
            Some(n) if n.starts_with("special:") => (
                true,
                Some(n.strip_prefix("special:").unwrap_or(n).to_string()),
            ),
            _ => (false, None),
        };

        Some(WorkspaceInfo {
            id: ws.id as i64,
            num: ws.idx as i32,
            name,
            is_special,
            special_name,
        })
    }
}

impl Compositor for Niri {
    fn logout(&self) -> futures::future::BoxFuture<'_, anyhow::Result<()>> {
        Box::pin(async {
            tokio::task::spawn_blocking(|| logout_with_socket(Socket::connect()?)).await?
        })
    }

    fn get_refresh_rate(&self) -> f64 {
        let res = catch_unwind(|| {
            if let Ok(mut socket) = Socket::connect()
                && let Ok(Ok(Response::Outputs(outputs))) = socket.send(Request::Outputs)
            {
                for (_name, output) in outputs {
                    if let Some(mode_idx) = output.current_mode
                        && let Some(mode) = output.modes.get(mode_idx)
                    {
                        let rate = (mode.refresh_rate as f64) / 1000.0;
                        if rate > 0.0 {
                            return rate;
                        }
                    }
                }
            }
            60.0
        });
        res.unwrap_or(60.0)
    }

    fn get_workspace(&self) -> Option<WorkspaceInfo> {
        let res = catch_unwind(|| {
            let Ok(mut socket) = Socket::connect() else {
                return None;
            };
            let Ok(Ok(Response::Workspaces(workspaces))) = socket.send(Request::Workspaces) else {
                return None;
            };
            Self::parse_workspace(&workspaces)
        });
        res.ok().flatten()
    }

    fn get_workspaces(&self) -> Vec<WorkspaceInfo> {
        let res = catch_unwind(|| {
            let Ok(mut socket) = Socket::connect() else {
                return vec![];
            };
            let Ok(Ok(Response::Workspaces(workspaces))) = socket.send(Request::Workspaces) else {
                return vec![];
            };
            let mut list: Vec<WorkspaceInfo> = workspaces
                .iter()
                .map(|ws| {
                    let name = match &ws.name {
                        Some(n) => n.clone(),
                        None => ws.idx.to_string(),
                    };
                    let (is_special, special_name) = match ws.name.as_deref() {
                        Some(n) if n.starts_with("special:") => (
                            true,
                            Some(n.strip_prefix("special:").unwrap_or(n).to_string()),
                        ),
                        _ => (false, None),
                    };
                    WorkspaceInfo {
                        id: ws.id as i64,
                        num: ws.idx as i32,
                        name,
                        is_special,
                        special_name,
                    }
                })
                .collect();
            list.sort_by_key(|w| w.num);
            list
        });
        res.unwrap_or_default()
    }

    fn switch_workspace(&self, id: i64) {
        let _ = catch_unwind(|| {
            let Ok(mut socket) = Socket::connect() else {
                return;
            };
            let _ = socket.send(Request::Action(niri_ipc::Action::FocusWorkspace {
                reference: niri_ipc::WorkspaceReferenceArg::Id(id as u64),
            }));
        });
    }

    fn request_layer_focus(&self) {}
}

fn logout_with_socket(mut socket: Socket) -> anyhow::Result<()> {
    match socket.send(Request::Action(niri_ipc::Action::Quit {
        skip_confirmation: true,
    }))? {
        Ok(Response::Handled) => Ok(()),
        Ok(response) => anyhow::bail!("Unexpected Niri logout response: {response:?}"),
        Err(error) => anyhow::bail!("Niri rejected logout: {error}"),
    }
}

pub async fn run_events_listener(
    current_workspace: Arc<ArcSwap<WorkspaceInfo>>,
    tx: broadcast::Sender<WorkspaceInfo>,
) {
    tokio::task::spawn_blocking(move || {
        loop {
            let res = catch_unwind(std::panic::AssertUnwindSafe(|| {
                let Ok(mut socket) = Socket::connect() else {
                    return;
                };

                let Ok(Ok(Response::Handled)) = socket.send(Request::EventStream) else {
                    return;
                };

                let mut read_event = socket.read_events();
                while let Ok(event) = read_event() {
                    let should_update = match event {
                        niri_ipc::Event::WorkspaceActivated { focused, .. } => focused,
                        niri_ipc::Event::WorkspacesChanged { .. } => true,
                        _ => false,
                    };

                    if should_update && let Some(ws) = Niri::new().get_workspace() {
                        let prev = current_workspace.load();
                        if **prev != ws {
                            current_workspace.store(Arc::new(ws.clone()));
                            let _ = tx.send(ws);
                        }
                    }
                }
            }));

            let _ = res;
            std::thread::sleep(Duration::from_secs(2));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn logout_ipc_sends_quit_and_validates_the_reply() {
        use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

        for (name, reply, success) in [
            ("handled", Ok(Response::Handled), true),
            ("rejected", Err("logout denied".to_owned()), false),
            ("unexpected", Ok(Response::Workspaces(Vec::new())), false),
        ] {
            let path = std::env::temp_dir().join(format!(
                "starshell-niri-logout-{}-{name}.sock",
                std::process::id()
            ));
            let listener = tokio::net::UnixListener::bind(&path).expect("mock socket");
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("connection");
                let mut stream = BufReader::new(stream);
                let mut request = String::new();
                stream.read_line(&mut request).await.expect("request");
                let request: Request = serde_json::from_str(&request).expect("typed request");
                assert!(matches!(
                    request,
                    Request::Action(niri_ipc::Action::Quit {
                        skip_confirmation: true
                    })
                ));
                let mut reply = serde_json::to_vec(&reply).expect("reply");
                reply.push(b'\n');
                stream.get_mut().write_all(&reply).await.expect("response");
            });
            let client_path = path.clone();
            let result = tokio::task::spawn_blocking(move || {
                logout_with_socket(Socket::connect_to(client_path)?)
            })
            .await
            .expect("client task");
            assert_eq!(result.is_ok(), success, "{name}: {result:?}");
            server.await.expect("mock server");
            std::fs::remove_file(path).expect("fixture cleanup");
        }
    }

    #[test]
    fn test_parse_niri_workspace() {
        let workspaces = vec![
            niri_ipc::Workspace {
                id: 1,
                idx: 1,
                name: None,
                output: None,
                is_urgent: false,
                is_active: false,
                is_focused: false,
                active_window_id: None,
            },
            niri_ipc::Workspace {
                id: 2,
                idx: 2,
                name: Some("code".to_string()),
                output: None,
                is_urgent: false,
                is_active: true,
                is_focused: true,
                active_window_id: None,
            },
        ];

        let ws = Niri::parse_workspace(&workspaces);
        assert_eq!(
            ws,
            Some(WorkspaceInfo {
                id: 2,
                num: 2,
                name: "code".to_string(),
                is_special: false,
                special_name: None,
            })
        );
    }
}
