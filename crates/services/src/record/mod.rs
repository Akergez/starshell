pub mod options;
pub mod session;

use arc_swap::ArcSwap;
use chrono::Local;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, broadcast};

pub use options::{RecordAudio, RecordBackend, RecordOptions, RecordStatus, RecordTarget};
use session::{ActiveSession, recorder_command, recorder_signal};

#[derive(Clone)]
pub struct RecordService {
    status: Arc<ArcSwap<RecordStatus>>,
    active_session: Arc<Mutex<Option<ActiveSession>>>,
    status_tx: broadcast::Sender<RecordStatus>,
}

impl Default for RecordService {
    fn default() -> Self {
        Self::new()
    }
}

impl RecordService {
    pub fn new() -> Self {
        let (status_tx, _) = broadcast::channel(16);
        Self {
            status: Arc::new(ArcSwap::from_pointee(RecordStatus::Stopped)),
            active_session: Arc::new(Mutex::new(None)),
            status_tx,
        }
    }

    pub fn get_status(&self) -> RecordStatus {
        **self.status.load()
    }

    pub fn is_recording(&self) -> bool {
        self.get_status() == RecordStatus::Recording
    }

    pub fn is_paused(&self) -> bool {
        self.get_status() == RecordStatus::Paused
    }

    pub fn subscribe_status(&self) -> broadcast::Receiver<RecordStatus> {
        self.status_tx.subscribe()
    }

    pub fn mark_stopped(&self) {
        self.status.store(Arc::new(RecordStatus::Stopped));
        let _ = self.status_tx.send(RecordStatus::Stopped);
    }

    pub fn get_duration_secs(&self) -> u64 {
        if let Ok(guard) = self.active_session.try_lock()
            && let Some(ref session) = *guard
        {
            return session.current_duration().as_secs();
        }
        0
    }

    pub async fn get_duration(&self) -> Duration {
        let guard = self.active_session.lock().await;
        if let Some(ref session) = *guard {
            session.current_duration()
        } else {
            Duration::ZERO
        }
    }

    pub async fn get_output_path(&self) -> Option<PathBuf> {
        let guard = self.active_session.lock().await;
        guard.as_ref().map(|s| s.output_path.clone())
    }

    pub async fn get_active_backend(&self) -> Option<RecordBackend> {
        let guard = self.active_session.lock().await;
        guard.as_ref().map(|s| s.backend)
    }

    pub async fn start(&self, options: RecordOptions) -> Result<PathBuf, String> {
        let mut guard = self.active_session.lock().await;
        if guard.is_some() {
            return Err("Ya hay una grabación en curso".to_string());
        }

        let output_path = if let Some(ref p) = options.output_path {
            p.clone()
        } else {
            Self::generate_default_output_path(&options.container)
        };

        if let Some(parent) = output_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let session = match options.backend {
            RecordBackend::GpuScreenRecorder => {
                session::spawn_gpu_screen_recorder(&options, &output_path).await?
            }
            RecordBackend::WlScreenRec => {
                session::spawn_wl_screenrec(&options, &output_path).await?
            }
        };

        let child_pid = session.child_pid;
        *guard = Some(session);
        self.status.store(Arc::new(RecordStatus::Recording));
        let _ = self.status_tx.send(RecordStatus::Recording);

        let active_clone = self.active_session.clone();
        let status_clone = self.status.clone();
        let tx_clone = self.status_tx.clone();

        crate::spawn_tokio(async move {
            loop {
                tokio::time::sleep(Duration::from_millis(500)).await;
                let mut lock = active_clone.lock().await;
                let Some(session) = lock
                    .as_mut()
                    .filter(|session| session.child_pid == child_pid)
                else {
                    break;
                };
                let finished = session
                    .child
                    .as_mut()
                    .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))));
                if finished {
                    if let Some(session) = lock.take()
                        && let Some(socket) = session.ipc_socket_path
                    {
                        let _ = std::fs::remove_file(socket);
                    }
                    status_clone.store(Arc::new(RecordStatus::Stopped));
                    let _ = tx_clone.send(RecordStatus::Stopped);
                    break;
                }
            }
        });

        Ok(output_path)
    }

    pub async fn stop(&self) -> Result<PathBuf, String> {
        let mut guard = self.active_session.lock().await;
        let mut session = guard
            .take()
            .ok_or_else(|| "No hay ninguna grabación en curso".to_string())?;
        let exited = session
            .child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))));
        let stopped_via_ipc = if !exited && let Some(socket) = &session.ipc_socket_path {
            recorder_command(socket, "stop", None, Duration::from_secs(3))
                .await
                .is_ok()
        } else {
            false
        };
        let exited = session
            .child
            .as_mut()
            .is_some_and(|child| matches!(child.try_wait(), Ok(Some(_))));
        if !exited
            && !stopped_via_ipc
            && let Err(error) = recorder_signal(session.child_pid, libc::SIGINT)
        {
            *guard = Some(session);
            return Err(error);
        }
        let outcome = if let Some(child) = session.child.as_mut() {
            match tokio::time::timeout(Duration::from_secs(3), child.wait()).await {
                Ok(Ok(status)) => Ok(status.success()),
                Ok(Err(error)) => Err(format!("No se pudo esperar al grabador: {error}")),
                Err(_) => {
                    Err("El grabador no terminó a tiempo; vuelve a intentar detenerlo".into())
                }
            }
        } else {
            Ok(true)
        };
        let success = match outcome {
            Ok(success) => success,
            Err(error) => {
                *guard = Some(session);
                return Err(error);
            }
        };
        if let Some(socket) = &session.ipc_socket_path {
            let _ = std::fs::remove_file(socket);
        }
        self.status.store(Arc::new(RecordStatus::Stopped));
        let _ = self.status_tx.send(RecordStatus::Stopped);
        if !success {
            return Err("El grabador terminó con un error al guardar el archivo".into());
        }
        Ok(session.output_path)
    }

    pub async fn pause(&self) -> Result<(), String> {
        let mut guard = self.active_session.lock().await;
        let session = guard
            .as_mut()
            .ok_or_else(|| "No hay grabación activa".to_string())?;

        if session.current_segment_start.is_none() {
            return Ok(());
        }

        if let Some(ref socket) = session.ipc_socket_path {
            recorder_command(socket, "set-paused", Some(true), Duration::from_secs(1)).await?;
        } else {
            recorder_signal(session.child_pid, libc::SIGUSR2)?;
        }

        if let Some(start) = session.current_segment_start.take() {
            session.accumulated_active_duration += start.elapsed();
        }
        self.status.store(Arc::new(RecordStatus::Paused));
        let _ = self.status_tx.send(RecordStatus::Paused);
        Ok(())
    }

    pub async fn resume(&self) -> Result<(), String> {
        let mut guard = self.active_session.lock().await;
        let session = guard
            .as_mut()
            .ok_or_else(|| "No hay grabación activa".to_string())?;

        if session.current_segment_start.is_some() {
            return Ok(());
        }

        if let Some(ref socket) = session.ipc_socket_path {
            recorder_command(socket, "set-paused", Some(false), Duration::from_secs(1)).await?;
        } else {
            recorder_signal(session.child_pid, libc::SIGUSR2)?;
        }

        session.current_segment_start = Some(Instant::now());
        self.status.store(Arc::new(RecordStatus::Recording));
        let _ = self.status_tx.send(RecordStatus::Recording);
        Ok(())
    }

    pub async fn toggle_pause(&self) -> Result<(), String> {
        match self.get_status() {
            RecordStatus::Recording => self.pause().await,
            RecordStatus::Paused => self.resume().await,
            RecordStatus::Stopped => Err("No hay grabación activa".to_string()),
        }
    }

    pub fn list_monitors() -> Vec<String> {
        if let Ok(output) = std::process::Command::new("gpu-screen-recorder")
            .arg("--list-monitors")
            .output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let monitors: Vec<String> = text
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.split('|').next().unwrap_or(trimmed).to_string())
                    }
                })
                .collect();
            if !monitors.is_empty() {
                return monitors;
            }
        }

        if let Ok(output) = std::process::Command::new("grim").arg("-l").output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            let list: Vec<String> = text
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect();
            if !list.is_empty() {
                return list;
            }
        }

        vec!["screen".to_string()]
    }

    pub fn list_audio_devices() -> Vec<String> {
        if let Ok(output) = std::process::Command::new("gpu-screen-recorder")
            .arg("--list-audio-devices")
            .output()
            && output.status.success()
        {
            let text = String::from_utf8_lossy(&output.stdout);
            return text
                .lines()
                .filter_map(|line| {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.split('|').next().unwrap_or(trimmed).to_string())
                    }
                })
                .collect();
        }
        vec!["default_output".to_string(), "default_input".to_string()]
    }

    fn generate_default_output_path(container: &str) -> PathBuf {
        let base_dir = dirs::video_dir().unwrap_or_else(|| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("/tmp"))
                .join("Videos")
        });
        let filename = format!(
            "Capsule_Record_{}.{}",
            Local::now().format("%Y-%m-%d_%H-%M-%S"),
            container
        );
        base_dir.join(filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_record_status() {
        let service = RecordService::new();
        assert_eq!(service.get_status(), RecordStatus::Stopped);
        assert!(!service.is_recording());
        assert!(!service.is_paused());
    }

    #[test]
    fn test_generate_default_output_path() {
        let path = RecordService::generate_default_output_path("mp4");
        assert!(path.to_string_lossy().ends_with(".mp4"));
        assert!(path.to_string_lossy().contains("Capsule_Record_"));
    }

    #[tokio::test]
    async fn test_live_recording_lifecycle() {
        if !RecordBackend::GpuScreenRecorder.is_available() {
            return;
        }

        let temp_dir = std::env::temp_dir();
        let output_file = temp_dir.join("test_capsule_live_record.mp4");
        let _ = std::fs::remove_file(&output_file);

        let service = RecordService::new();
        let options = RecordOptions {
            backend: RecordBackend::GpuScreenRecorder,
            target: RecordTarget::Screen,
            audio: RecordAudio::None,
            fps: 30,
            resolution: "native".to_string(),
            quality: "very_high".to_string(),
            container: "mp4".to_string(),
            output_path: Some(output_file.clone()),
            include_cursor: false,
        };

        let start_res = service.start(options).await;
        if start_res.is_err() {
            return;
        }

        assert!(service.is_recording());
        tokio::time::sleep(Duration::from_millis(1500)).await;
        assert!(service.is_recording());

        let stop_res = service.stop().await;
        assert!(stop_res.is_ok());
        assert_eq!(service.get_status(), RecordStatus::Stopped);

        assert!(output_file.exists());
        let meta = std::fs::metadata(&output_file);
        assert!(meta.map(|m| m.len() > 0).unwrap_or(false));
        let _ = std::fs::remove_file(&output_file);
    }

    #[test]
    fn rejected_pause_keeps_recording_state_and_elapsed_time() {
        crate::tokio_handle().block_on(async {
            let service = RecordService::new();
            service.status.store(Arc::new(RecordStatus::Recording));
            *service.active_session.lock().await = Some(ActiveSession {
                backend: RecordBackend::GpuScreenRecorder,
                output_path: PathBuf::from("/tmp/fixture.mp4"),
                ipc_socket_path: Some(PathBuf::from("/tmp/capsule-nonexistent-recorder-ipc.sock")),
                child_pid: 0,
                child: None,
                accumulated_active_duration: Duration::from_secs(5),
                current_segment_start: Some(Instant::now()),
            });
            assert!(service.pause().await.is_err());
            assert_eq!(service.get_status(), RecordStatus::Recording);
            let guard = service.active_session.lock().await;
            assert!(
                guard
                    .as_ref()
                    .expect("session")
                    .current_segment_start
                    .is_some()
            );
            assert!(guard.as_ref().expect("session").current_duration() >= Duration::from_secs(5));
        });
    }

    #[test]
    fn test_record_service_duration_stopped() {
        let service = RecordService::new();
        assert_eq!(service.get_duration_secs(), 0);
    }
}
