use gpui::App;
use services::IpcCommand;

use super::{Placement, Shell, Surface, launch_apps};
use crate::applets::AppletKind;
use crate::panel::SettingsPanel;

pub fn start(cx: &mut App) {
    cx.spawn(async move |cx| {
        loop {
            services::wait_for_ipc_command().await;
            cx.update(|cx| {
                while let Some(command) = services::pop_ipc_command() {
                    handle(command, cx);
                }
            });
        }
    })
    .detach();
}

fn surface_for(command: &IpcCommand) -> Option<(Surface, bool)> {
    let applet = |kind| Surface::Applet(kind);
    Some(match command {
        IpcCommand::ToggleLauncher => (Surface::Launcher, true),
        IpcCommand::ShowLauncher => (Surface::Launcher, false),
        IpcCommand::ToggleDashboard => (applet(AppletKind::ControlCenter), true),
        IpcCommand::ShowDashboard => (applet(AppletKind::ControlCenter), false),
        IpcCommand::ToggleNotification => (applet(AppletKind::Notifications), true),
        IpcCommand::ShowNotification => (applet(AppletKind::Notifications), false),
        IpcCommand::ToggleSelectTheme => (applet(AppletKind::Themes), true),
        IpcCommand::ShowThemes => (applet(AppletKind::Themes), false),
        IpcCommand::ToggleSelectWallpaper => (applet(AppletKind::Wallpapers), true),
        IpcCommand::ShowWallpapers => (applet(AppletKind::Wallpapers), false),
        IpcCommand::ToggleClipboard => (applet(AppletKind::Clipboard), true),
        IpcCommand::ShowClipboard => (applet(AppletKind::Clipboard), false),
        IpcCommand::ToggleEmoji => (applet(AppletKind::Emoji), true),
        IpcCommand::ShowEmoji => (applet(AppletKind::Emoji), false),
        IpcCommand::ToggleShelf => (applet(AppletKind::Shelf), true),
        IpcCommand::ShowShelf => (applet(AppletKind::Shelf), false),
        IpcCommand::ToggleRecord => (applet(AppletKind::Record), true),
        IpcCommand::ShowRecord => (applet(AppletKind::Record), false),
        IpcCommand::ToggleAudio => (applet(AppletKind::Audio), true),
        IpcCommand::ToggleDisplays => (applet(AppletKind::Displays), true),
        IpcCommand::ToggleMedia => (applet(AppletKind::Media), true),
        IpcCommand::ToggleCalendar => (applet(AppletKind::Calendar), true),
        IpcCommand::ToggleSystem => (applet(AppletKind::System), true),
        _ => return None,
    })
}

pub fn handle(command: IpcCommand, cx: &mut App) {
    if let Some((surface, toggle)) = surface_for(&command) {
        if toggle {
            Shell::toggle(surface, Placement::default(), cx);
        } else {
            Shell::open(surface, Placement::default(), cx);
        }
        return;
    }
    match command {
        IpcCommand::Terminal
        | IpcCommand::Browser
        | IpcCommand::Editor
        | IpcCommand::FileManager => launch_apps::launch(command, cx),
        IpcCommand::Hide | IpcCommand::Default => Shell::close(cx),
        IpcCommand::ToggleSettings => {
            Shell::close(cx);
            SettingsPanel::toggle(cx);
        }
        IpcCommand::ShowSettings => {
            Shell::close(cx);
            SettingsPanel::open(cx);
        }
        IpcCommand::Lock => {
            Shell::close(cx);
            if let Err(error) = services::send_ipc_command("starshell-lock", &IpcCommand::Lock) {
                services::log_error!("Lock", "starshell-lock is not running: {error}");
            }
        }
        IpcCommand::Quit => cx.quit(),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dashboard_commands_open_the_control_center() {
        assert_eq!(
            surface_for(&IpcCommand::ToggleDashboard),
            Some((Surface::Applet(AppletKind::ControlCenter), true))
        );
        assert_eq!(
            surface_for(&IpcCommand::ShowLauncher),
            Some((Surface::Launcher, false))
        );
        assert_eq!(surface_for(&IpcCommand::Quit), None);
    }
}
