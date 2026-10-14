use anyhow::{Context, Result};
use tokio::process::Command;

pub async fn lock() -> Result<()> {
    if Command::new("hyprlock").status().await.is_err()
        && Command::new("swaylock").status().await.is_err()
    {
        Command::new("loginctl")
            .arg("lock-session")
            .status()
            .await
            .context("Failed to lock session")?;
    }
    Ok(())
}

pub async fn suspend() -> Result<()> {
    Command::new("systemctl")
        .arg("suspend")
        .status()
        .await
        .context("Failed to suspend system")?;
    Ok(())
}

pub async fn reboot() -> Result<()> {
    let output = Command::new("systemctl")
        .arg("reboot")
        .output()
        .await
        .context("Failed to reboot system")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to reboot system: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

pub async fn logout() -> Result<()> {
    let session = std::env::var("XDG_SESSION_ID").context("No login session is available")?;
    anyhow::ensure!(!session.trim().is_empty(), "No login session is available");
    let output = Command::new("loginctl")
        .args(["terminate-session", &session])
        .output()
        .await
        .context("Failed to log out")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to log out: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

pub async fn poweroff() -> Result<()> {
    let output = Command::new("systemctl")
        .arg("poweroff")
        .output()
        .await
        .context("Failed to poweroff system")?;
    anyhow::ensure!(
        output.status.success(),
        "Failed to poweroff system: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}
