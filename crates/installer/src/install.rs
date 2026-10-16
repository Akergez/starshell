use anyhow::{Context, Result, anyhow};
use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

const DESKTOP_ENTRY_CONTENT: &str = r#"[Desktop Entry]
Name=Starshell
Comment=Desktop shell for niri
Exec=/usr/local/bin/starshell
Icon=starshell
Terminal=false
Type=Application
Path=$HOME
NoDisplay=true
Categories=Utility;System;
"#;

pub fn run_sudo_cmd(args: &[&str], password: Option<&str>) -> Result<()> {
    let mut cmd = Command::new("sudo");
    if password.is_some() {
        cmd.args(["-S", "-p", ""]);
    }
    cmd.args(args);
    cmd.stdin(Stdio::piped());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .with_context(|| format!("Failed to run sudo with args {:?}", args))?;

    if let (Some(pass), Some(mut stdin)) = (password, child.stdin.take()) {
        let _ = writeln!(stdin, "{}", pass);
    }

    let output = child.wait_with_output()?;
    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!("sudo command failed: {}", err.trim()));
    }

    Ok(())
}

pub fn copy_binary_to_system(source_bin: &Path, password: Option<&str>) -> Result<()> {
    let dest_dir = Path::new("/usr/local/bin");
    let dest_file = dest_dir.join("starshell");

    if !dest_dir.exists() {
        run_sudo_cmd(&["mkdir", "-p", "/usr/local/bin"], password)
            .context("Failed to create /usr/local/bin")?;
    }

    run_sudo_cmd(
        &[
            "cp",
            source_bin.to_string_lossy().as_ref(),
            dest_file.to_string_lossy().as_ref(),
        ],
        password,
    )
    .context("Failed to copy binary to /usr/local/bin/starshell")?;

    run_sudo_cmd(
        &["chmod", "755", dest_file.to_string_lossy().as_ref()],
        password,
    )
    .context("Failed to chmod /usr/local/bin/starshell")?;

    Ok(())
}

pub fn create_desktop_entry(password: Option<&str>) -> Result<()> {
    let desktop_dir = Path::new("/usr/share/applications");
    let desktop_file = desktop_dir.join("starshell.desktop");

    if !desktop_dir.exists() {
        let _ = run_sudo_cmd(&["mkdir", "-p", "/usr/share/applications"], password);
    }

    let temp_desktop =
        std::env::temp_dir().join(format!("starshell-desktop-{}.desktop", std::process::id()));
    fs::write(&temp_desktop, DESKTOP_ENTRY_CONTENT)?;

    run_sudo_cmd(
        &[
            "cp",
            temp_desktop.to_string_lossy().as_ref(),
            desktop_file.to_string_lossy().as_ref(),
        ],
        password,
    )?;

    run_sudo_cmd(
        &["chmod", "644", desktop_file.to_string_lossy().as_ref()],
        password,
    )?;

    let _ = fs::remove_file(temp_desktop);
    Ok(())
}

const SESSION_FILES: [(&str, &str, &str); 5] = [
    (
        include_str!("../session/starshell.desktop"),
        "/usr/share/wayland-sessions/starshell.desktop",
        "644",
    ),
    (
        include_str!("../session/starshell-portals.conf"),
        "/usr/share/xdg-desktop-portal/starshell-portals.conf",
        "644",
    ),
    (
        include_str!("../session/starshell-session"),
        "/usr/local/bin/starshell-session",
        "755",
    ),
    (
        include_str!("../session/starshell.service"),
        "/usr/local/lib/systemd/user/starshell.service",
        "644",
    ),
    (
        include_str!("../session/starshell-session.target"),
        "/usr/local/lib/systemd/user/starshell-session.target",
        "644",
    ),
];

pub fn install_session(password: Option<&str>) -> Result<()> {
    for (content, destination, mode) in SESSION_FILES {
        let destination = Path::new(destination);
        if let Some(parent) = destination.parent() {
            run_sudo_cmd(
                &["mkdir", "-p", parent.to_string_lossy().as_ref()],
                password,
            )?;
        }
        let file_name = destination
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let temp = std::env::temp_dir().join(format!(
            "starshell-session-{}-{file_name}",
            std::process::id()
        ));
        fs::write(&temp, content)?;
        let copied = run_sudo_cmd(
            &[
                "install",
                "-m",
                mode,
                temp.to_string_lossy().as_ref(),
                destination.to_string_lossy().as_ref(),
            ],
            password,
        );
        let _ = fs::remove_file(&temp);
        copied.with_context(|| format!("Failed to install {}", destination.display()))?;
    }
    let _ = Command::new("systemctl")
        .args(["--user", "daemon-reload"])
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    Ok(())
}

pub fn launch_starshell() -> Result<()> {
    let home = std::env::var("HOME").unwrap_or_else(|_| "/root".to_string());

    if crate::system::command_exists("gtk-launch")
        && Path::new("/usr/share/applications/starshell.desktop").exists()
    {
        let status = Command::new("gtk-launch")
            .arg("starshell.desktop")
            .current_dir(&home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();

        if status.is_ok() {
            return Ok(());
        }
    }

    Command::new("/usr/local/bin/starshell")
        .current_dir(&home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .context("Failed to spawn /usr/local/bin/starshell")?;

    Ok(())
}
