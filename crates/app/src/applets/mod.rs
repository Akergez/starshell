pub mod common;
pub mod control_center;

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
            Self::TrayMenu(_) => 240.0,
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
        _ => common::placeholder(surface, cx),
    }
}
