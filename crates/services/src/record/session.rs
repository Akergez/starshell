use chrono::Local;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;

use super::options::{RecordAudio, RecordBackend, RecordOptions, RecordTarget};

pub async fn recorder_command(
    socket: &Path,
    name: &str,
    data: Option<bool>,
    timeout: Duration,
) -> Result<(), String> {
    let operation = async {
        let mut stream = UnixStream::connect(socket)
            .await
            .map_err(|error| format!("No se pudo conectar con la grabación: {error}"))?;
        let mut request = serde_json::json!({ "id": 1, "name": name });
        if let Some(data) = data {
            request["data"] = data.into();
        }
        let mut bytes = serde_json::to_vec(&request).map_err(|error| error.to_string())?;
        bytes.push(b'\n');
        stream
            .write_all(&bytes)
            .await
            .map_err(|error| format!("No se pudo controlar la grabación: {error}"))?;
        let mut reader = BufReader::new(stream).take(65536);
        let mut response = String::new();
        reader
            .read_line(&mut response)
            .await
            .map_err(|error| format!("No se pudo leer la respuesta de grabación: {error}"))?;
        let response: serde_json::Value = serde_json::from_str(&response)
            .map_err(|error| format!("Respuesta de grabación inválida: {error}"))?;
        if response.get("id").and_then(serde_json::Value::as_u64) != Some(1) {
            return Err("La respuesta de grabación no corresponde al comando".into());
        }
        if response.get("result").and_then(serde_json::Value::as_str) == Some("ok") {
            Ok(())
        } else {
            Err(response
                .get("data")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("El grabador rechazó el comando")
                .to_string())
        }
    };
    tokio::time::timeout(timeout, operation)
        .await
        .map_err(|_| "El grabador no respondió a tiempo".to_string())?
}

pub fn recorder_signal(pid: u32, signal: i32) -> Result<(), String> {
    if unsafe { libc::kill(pid as libc::pid_t, signal) } == 0 {
        Ok(())
    } else {
        Err(format!(
            "No se pudo controlar la grabación: {}",
            std::io::Error::last_os_error()
        ))
    }
}

pub struct ActiveSession {
    pub backend: RecordBackend,
    pub output_path: PathBuf,
    pub ipc_socket_path: Option<PathBuf>,
    pub child_pid: u32,
    pub child: Option<tokio::process::Child>,
    pub accumulated_active_duration: Duration,
    pub current_segment_start: Option<Instant>,
}

impl ActiveSession {
    pub fn current_duration(&self) -> Duration {
        let mut total = self.accumulated_active_duration;
        if let Some(start) = self.current_segment_start {
            total += start.elapsed();
        }
        total
    }
}

pub async fn spawn_gpu_screen_recorder(
    options: &RecordOptions,
    output_path: &Path,
) -> Result<ActiveSession, String> {
    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let socket_path = PathBuf::from(format!(
        "{}/starshell_gsr_{}_{}.sock",
        runtime_dir,
        std::process::id(),
        Local::now().timestamp_millis()
    ));
    let _ = std::fs::remove_file(&socket_path);

    let mut cmd = tokio::process::Command::new("gpu-screen-recorder");

    let target_str = match &options.target {
        RecordTarget::Screen => "screen".to_string(),
        RecordTarget::Monitor(m) => m.clone(),
        RecordTarget::Portal => "portal".to_string(),
        RecordTarget::Region { .. } => "region".to_string(),
    };
    cmd.arg("-w").arg(&target_str);

    if let RecordTarget::Region {
        x,
        y,
        width,
        height,
    } = &options.target
    {
        cmd.arg("-region").arg(format!("{width}x{height}+{x}+{y}"));
    }

    cmd.arg("-f").arg(options.fps.to_string());
    cmd.arg("-c").arg(&options.container);
    cmd.arg("-cursor")
        .arg(if options.include_cursor { "yes" } else { "no" });
    cmd.arg("-q").arg(&options.quality);

    if options.resolution != "native" && options.resolution.contains('x') {
        cmd.arg("-s").arg(&options.resolution);
    }

    match &options.audio {
        RecordAudio::None => {}
        RecordAudio::Desktop => {
            cmd.arg("-a").arg("default_output");
        }
        RecordAudio::Microphone => {
            cmd.arg("-a").arg("default_input");
        }
        RecordAudio::Both => {
            cmd.arg("-a").arg("default_output|default_input");
        }
        RecordAudio::Custom(src) => {
            cmd.arg("-a").arg(src);
        }
    }

    cmd.arg("-o").arg(output_path);
    cmd.arg("-ipc").arg(&socket_path);

    let log_path = PathBuf::from(format!("{}/starshell_record.log", runtime_dir));
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Fallo al crear archivo de log: {e}"))?;
    let err_file = log_file
        .try_clone()
        .map_err(|e| format!("Fallo al duplicar archivo de log: {e}"))?;

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(log_file));
    cmd.stderr(Stdio::from(err_file));

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Fallo al iniciar gpu-screen-recorder: {e}"))?;

    let child_pid = child
        .id()
        .ok_or_else(|| "No se pudo obtener el PID del proceso de grabación".to_string())?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    if let Ok(Some(status)) = child.try_wait() {
        let err_msg = std::fs::read_to_string(&log_path).unwrap_or_default();
        return Err(format!(
            "gpu-screen-recorder terminó inesperadamente ({status}): {err_msg}"
        ));
    }

    Ok(ActiveSession {
        backend: RecordBackend::GpuScreenRecorder,
        output_path: output_path.to_path_buf(),
        ipc_socket_path: Some(socket_path),
        child_pid,
        child: Some(child),
        accumulated_active_duration: Duration::ZERO,
        current_segment_start: Some(Instant::now()),
    })
}

pub async fn spawn_wl_screenrec(
    options: &RecordOptions,
    output_path: &Path,
) -> Result<ActiveSession, String> {
    let mut cmd = tokio::process::Command::new("wl-screenrec");
    cmd.arg("-f").arg(output_path);

    match &options.target {
        RecordTarget::Screen => {}
        RecordTarget::Monitor(m) => {
            cmd.arg("-o").arg(m);
        }
        RecordTarget::Region {
            x,
            y,
            width,
            height,
        } => {
            cmd.arg("-g").arg(format!("{x},{y} {width}x{height}"));
        }
        RecordTarget::Portal => {}
    }

    match &options.audio {
        RecordAudio::None => {}
        _ => {
            cmd.arg("--audio");
        }
    }

    let runtime_dir = std::env::var("XDG_RUNTIME_DIR").unwrap_or_else(|_| "/tmp".to_string());
    let log_path = PathBuf::from(format!("{}/starshell_wl_record.log", runtime_dir));
    let log_file = std::fs::File::create(&log_path)
        .map_err(|e| format!("Fallo al crear archivo de log: {e}"))?;
    let err_file = log_file
        .try_clone()
        .map_err(|e| format!("Fallo al duplicar archivo de log: {e}"))?;

    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::from(log_file));
    cmd.stderr(Stdio::from(err_file));

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Fallo al iniciar wl-screenrec: {e}"))?;

    let child_pid = child
        .id()
        .ok_or_else(|| "No se pudo obtener el PID del proceso de grabación".to_string())?;

    tokio::time::sleep(Duration::from_millis(200)).await;

    if let Ok(Some(status)) = child.try_wait() {
        let err_msg = std::fs::read_to_string(&log_path).unwrap_or_default();
        return Err(format!(
            "wl-screenrec terminó prematuramente ({status}): {err_msg}"
        ));
    }

    Ok(ActiveSession {
        backend: RecordBackend::WlScreenRec,
        output_path: output_path.to_path_buf(),
        ipc_socket_path: None,
        child_pid,
        child: Some(child),
        accumulated_active_duration: Duration::ZERO,
        current_segment_start: Some(Instant::now()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_ipc_rejects_errors_and_times_out_without_a_reply() {
        crate::tokio_handle().block_on(async {
            use tokio::net::UnixListener;
            let root = std::env::temp_dir()
                .join(format!("starshell-record-ipc-test-{}", std::process::id()));
            std::fs::create_dir_all(&root).expect("fixture directory");
            let rejected = root.join("rejected.sock");
            let listener = UnixListener::bind(&rejected).expect("mock recorder");
            let server = tokio::spawn(async move {
                let (stream, _) = listener.accept().await.expect("connection");
                let mut reader = BufReader::new(stream);
                let mut request = String::new();
                reader.read_line(&mut request).await.expect("request");
                let request: serde_json::Value =
                    serde_json::from_str(&request).expect("request JSON");
                assert_eq!(request["name"], "set-paused");
                assert_eq!(request["data"], true);
                reader
                    .get_mut()
                    .write_all(b"{\"id\":1,\"result\":\"error\",\"data\":\"Rejected pause\"}\n")
                    .await
                    .expect("reply");
            });
            assert_eq!(
                recorder_command(&rejected, "set-paused", Some(true), Duration::from_secs(1)).await,
                Err("Rejected pause".into())
            );
            server.await.expect("server");
            let stalled = root.join("stalled.sock");
            let listener = UnixListener::bind(&stalled).expect("silent recorder");
            let server = tokio::spawn(async move {
                let (_stream, _) = listener.accept().await.expect("connection");
                std::future::pending::<()>().await;
            });
            let started = Instant::now();
            assert!(
                recorder_command(&stalled, "stop", None, Duration::from_millis(30))
                    .await
                    .is_err()
            );
            assert!(started.elapsed() < Duration::from_secs(1));
            server.abort();
            std::fs::remove_dir_all(root).expect("remove fixtures");
        });
    }

    #[test]
    fn test_active_session_duration_math() {
        let mut session = ActiveSession {
            backend: RecordBackend::GpuScreenRecorder,
            output_path: PathBuf::from("/tmp/dummy.mp4"),
            ipc_socket_path: None,
            child_pid: 1234,
            child: None,
            accumulated_active_duration: Duration::from_secs(5),
            current_segment_start: None,
        };

        assert_eq!(session.current_duration().as_secs(), 5);

        session.current_segment_start = Some(Instant::now());
        std::thread::sleep(Duration::from_millis(50));
        assert!(session.current_duration() >= Duration::from_millis(5050));

        let start = session
            .current_segment_start
            .take()
            .unwrap_or_else(Instant::now);
        session.accumulated_active_duration += start.elapsed();
        assert!(session.current_duration().as_secs() >= 5);
        assert!(session.current_segment_start.is_none());
    }
}
