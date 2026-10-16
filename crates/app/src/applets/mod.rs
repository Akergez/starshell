pub mod appearance;
pub mod audio;
pub mod calendar;
pub mod clipboard;
pub mod common;
pub mod control_center;
pub mod displays;
pub mod emoji;
pub mod media;
pub mod notifications;
pub mod record;
pub mod shelf;
pub mod system;
pub mod tray_menu;

use gpui::{AppContext, Context, Pixels, Window};
use ui::tokens::size;

use crate::shell::Surface;
use crate::shell::overlay::{Content, Overlay};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppletKind {
    ControlCenter,
    Audio,
    Displays,
    Media,
    System,
    Calendar,
    Notifications,
    Clipboard,
    Emoji,
    Shelf,
    Record,
    Themes,
    Wallpapers,
    TrayMenu(usize),
}

impl AppletKind {
    pub fn width(self) -> f32 {
        match self {
            Self::Audio | Self::Displays | Self::Clipboard | Self::Notifications => {
                size::DEVICE_PANEL
            }
            Self::Wallpapers => size::LAUNCHER,
            Self::TrayMenu(_) => size::NAVIGATION_LIST,
            _ => size::PANEL,
        }
    }

    pub fn default_anchor(self, viewport_width: Pixels) -> Option<Pixels> {
        match self {
            Self::Calendar | Self::Themes | Self::Wallpapers | Self::Emoji | Self::Clipboard => {
                Some(viewport_width / 2.0)
            }
            Self::Media => Some(viewport_width / 4.0),
            _ => None,
        }
    }
}

pub fn build(surface: Surface, window: &mut Window, cx: &mut Context<Overlay>) -> Content {
    match surface {
        Surface::Applet(AppletKind::ControlCenter) => {
            let view = cx.new(|cx| control_center::ControlCenter::new(window, cx));
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Launcher => {
            let view = crate::launcher::Launcher::shared(window, cx);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Audio) => Content {
            view: cx.new(audio::AudioApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::Displays) => Content {
            view: cx.new(displays::DisplaysApplet::new).into(),
            focus: None,
        },
        Surface::Polkit => {
            let agent = crate::polkit::PolkitAgent::global(cx);
            Content {
                focus: Some(agent.read(cx).focus_handle()),
                view: agent.into(),
            }
        }
        Surface::Applet(AppletKind::Notifications) => {
            let view = cx.new(notifications::NotificationsApplet::new);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Media) => Content {
            view: cx.new(media::MediaApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::System) => Content {
            view: cx.new(system::SystemApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::Calendar) => Content {
            view: cx.new(calendar::CalendarApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::Clipboard) => {
            let view = cx.new(clipboard::ClipboardApplet::new);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Emoji) => {
            let view = cx.new(emoji::EmojiApplet::new);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Shelf) => {
            let view = cx.new(shelf::ShelfApplet::new);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Record) => {
            let view = cx.new(record::RecordApplet::new);
            Content {
                focus: Some(view.read(cx).focus_handle()),
                view: view.into(),
            }
        }
        Surface::Applet(AppletKind::Themes) => Content {
            view: cx.new(appearance::ThemesApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::Wallpapers) => Content {
            view: cx.new(appearance::WallpapersApplet::new).into(),
            focus: None,
        },
        Surface::Applet(AppletKind::TrayMenu(index)) => Content {
            view: cx
                .new(|cx| tray_menu::TrayMenuApplet::new(index, cx))
                .into(),
            focus: None,
        },
    }
}
