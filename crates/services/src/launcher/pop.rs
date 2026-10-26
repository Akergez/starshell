use std::path::PathBuf;
use std::process::Stdio;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Lines};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::mpsc;

use super::applications::ApplicationWatcher;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum PopRequest {
    Search(String),
    Activate(u32),
    ActivateContext { id: u32, context: u32 },
    Complete(u32),
    Context(u32),
    Interrupt,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum IconSource {
    Name(String),
    Mime(String),
}

impl IconSource {
    pub fn resolve(&self) -> Option<PathBuf> {
        match self {
            Self::Name(name) => crate::icons::resolve_icon_name(name),
            Self::Mime(mime) => crate::icons::resolve_mime_icon(mime),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SearchResult {
    pub id: u32,
    pub name: String,
    pub description: String,
    #[serde(default)]
    pub icon: Option<IconSource>,
    #[serde(default)]
    pub category_icon: Option<IconSource>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct ContextOption {
    pub id: u32,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub enum PopResponse {
    Close,
    Context {
        id: u32,
        options: Vec<ContextOption>,
    },
    DesktopEntry {
        path: PathBuf,
        #[serde(default)]
        action_name: Option<String>,
    },
    Update(Vec<SearchResult>),
    Fill(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PopEvent {
    Response(PopResponse),
    Unavailable(String),
    Reloaded,
}

struct Connection {
    requests: mpsc::UnboundedSender<PopRequest>,
}

#[derive(Clone, Default)]
pub struct PopLauncher {
    connection: Arc<Mutex<Option<Connection>>>,
}

impl PopLauncher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn connect(&self) -> mpsc::UnboundedReceiver<PopEvent> {
        let (events, receiver) = mpsc::unbounded_channel();
        let (requests, request_receiver) = mpsc::unbounded_channel();
        if let Ok(mut connection) = self.connection.lock() {
            *connection = Some(Connection { requests });
        }
        crate::spawn_tokio(run(request_receiver, events));
        receiver
    }

    pub fn send(&self, request: PopRequest) {
        if let Ok(connection) = self.connection.lock()
            && let Some(connection) = connection.as_ref()
        {
            let _ = connection.requests.send(request);
        }
    }

    pub fn disconnect(&self) {
        if let Ok(mut connection) = self.connection.lock() {
            connection.take();
        }
    }
}

pub fn description_line(description: &str) -> String {
    description
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join(" · ")
}

struct Backend {
    child: Child,
    stdin: ChildStdin,
    lines: Lines<BufReader<ChildStdout>>,
}

impl Backend {
    fn spawn() -> Result<Self, String> {
        let mut child = Command::new("pop-launcher")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| format!("pop-launcher is not available: {error}"))?;
        let (Some(stdin), Some(stdout)) = (child.stdin.take(), child.stdout.take()) else {
            return Err("pop-launcher did not open its pipes".to_string());
        };
        Ok(Self {
            child,
            stdin,
            lines: BufReader::new(stdout).lines(),
        })
    }

    async fn stop(mut self) {
        let _ = self.stdin.write_all(b"\"Exit\"\n").await;
        let _ = self.child.kill().await;
    }
}

fn watch_applications() -> (mpsc::UnboundedReceiver<()>, tokio::task::JoinHandle<()>) {
    let (changes, receiver) = mpsc::unbounded_channel();
    let task = crate::spawn_tokio(async move {
        loop {
            match ApplicationWatcher::new() {
                Ok(mut watcher) => loop {
                    match watcher.changed().await {
                        Ok(()) => {
                            if changes.send(()).is_err() {
                                return;
                            }
                        }
                        Err(error) => {
                            crate::log_warn!("LAUNCHER", "Application watcher stopped: {error}");
                            break;
                        }
                    }
                },
                Err(error) => {
                    crate::log_warn!("LAUNCHER", "Cannot watch applications: {error}");
                }
            }
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            if changes.is_closed() {
                return;
            }
        }
    });
    (receiver, task)
}

async fn run(
    mut requests: mpsc::UnboundedReceiver<PopRequest>,
    events: mpsc::UnboundedSender<PopEvent>,
) {
    let mut backend = match Backend::spawn() {
        Ok(backend) => backend,
        Err(error) => {
            let _ = events.send(PopEvent::Unavailable(error));
            return;
        }
    };
    let (mut application_changes, watcher) = watch_applications();
    loop {
        tokio::select! {
            request = requests.recv() => {
                let Some(request) = request else {
                    break;
                };
                let Ok(mut payload) = serde_json::to_string(&request) else {
                    continue;
                };
                payload.push('\n');
                if backend.stdin.write_all(payload.as_bytes()).await.is_err() || backend.stdin.flush().await.is_err() {
                    let _ = events.send(PopEvent::Unavailable("pop-launcher stopped responding".to_string()));
                    break;
                }
            }
            line = backend.lines.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if let Ok(response) = serde_json::from_str::<PopResponse>(&line)
                            && events.send(PopEvent::Response(response)).is_err()
                        {
                            break;
                        }
                    }
                    _ => {
                        let _ = events.send(PopEvent::Unavailable("pop-launcher exited".to_string()));
                        break;
                    }
                }
            }
            Some(()) = application_changes.recv() => {
                match Backend::spawn() {
                    Ok(fresh) => {
                        std::mem::replace(&mut backend, fresh).stop().await;
                        if events.send(PopEvent::Reloaded).is_err() {
                            break;
                        }
                    }
                    Err(error) => {
                        let _ = events.send(PopEvent::Unavailable(error));
                        break;
                    }
                }
            }
        }
    }
    watcher.abort();
    backend.stop().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_use_the_pop_launcher_wire_format() {
        assert_eq!(
            serde_json::to_string(&PopRequest::Search("/us".into())).expect("encode"),
            r#"{"Search":"/us"}"#
        );
        assert_eq!(
            serde_json::to_string(&PopRequest::ActivateContext { id: 2, context: 1 })
                .expect("encode"),
            r#"{"ActivateContext":{"id":2,"context":1}}"#
        );
        assert_eq!(
            serde_json::to_string(&PopRequest::Interrupt).expect("encode"),
            r#""Interrupt""#
        );
    }

    #[test]
    fn responses_decode_updates_and_fills() {
        let update: PopResponse = serde_json::from_str(
            r#"{"Update":[{"id":0,"name":"usr","description":"/usr","icon":{"Mime":"inode/directory"},"category_icon":null,"window":null}]}"#,
        )
        .expect("decode update");
        let PopResponse::Update(results) = update else {
            panic!("expected update");
        };
        assert_eq!(
            results[0].icon,
            Some(IconSource::Mime("inode/directory".into()))
        );
        let fill: PopResponse = serde_json::from_str(r#"{"Fill":"/usr/"}"#).expect("decode fill");
        assert_eq!(fill, PopResponse::Fill("/usr/".into()));
        let close: PopResponse = serde_json::from_str(r#""Close""#).expect("decode close");
        assert_eq!(close, PopResponse::Close);
    }

    #[test]
    fn multi_line_descriptions_become_one_line() {
        assert_eq!(
            description_line("first\n\n second \nthird"),
            "first · second · third"
        );
    }
}
