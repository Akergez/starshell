use std::time::Duration;

use gpui::{Pixels, Rems, px, rems};

pub const REFERENCE_FONT_SIZE: f32 = 13.0;
pub const DEFAULT_FONT_SIZE: f32 = 15.0;
pub const MINIMUM_TEXT_SIZE: f32 = 12.0;

pub fn units(value: f32) -> Rems {
    rems(value / REFERENCE_FONT_SIZE)
}

pub fn units_to_pixels(value: f32, font_size: f32) -> Pixels {
    px(value * font_size / REFERENCE_FONT_SIZE)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Spacing {
    Hairline,
    XXSmall,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
    XXXLarge,
}

impl Spacing {
    pub const fn value(self) -> f32 {
        match self {
            Self::Hairline => 1.0,
            Self::XXSmall => 2.0,
            Self::XSmall => 4.0,
            Self::Small => 6.0,
            Self::Medium => 8.0,
            Self::Large => 12.0,
            Self::XLarge => 16.0,
            Self::XXLarge => 20.0,
            Self::XXXLarge => 24.0,
        }
    }

    pub fn rems(self) -> Rems {
        units(self.value())
    }

    pub fn pixels(self, font_size: f32) -> Pixels {
        units_to_pixels(self.value(), font_size)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Radius {
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
}

impl Radius {
    pub const fn value(self) -> f32 {
        match self {
            Self::XSmall => 2.0,
            Self::Small => 4.0,
            Self::Medium => 6.0,
            Self::Large => 8.0,
            Self::XLarge => 12.0,
        }
    }

    pub fn rems(self) -> Rems {
        units(self.value())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextSize {
    XSmall,
    Small,
    Default,
    Medium,
    Large,
    XLarge,
}

impl TextSize {
    pub const fn fraction(self) -> f32 {
        match self {
            Self::XSmall => 0.80,
            Self::Small => 0.87,
            Self::Default => 1.0,
            Self::Medium => 1.08,
            Self::Large => 1.23,
            Self::XLarge => 1.38,
        }
    }

    pub fn pixels(self, font_size: f32) -> Pixels {
        px((font_size * self.fraction()).max(MINIMUM_TEXT_SIZE).round())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconSize {
    Indicator,
    XSmall,
    Small,
    Medium,
    Large,
    XLarge,
    XXLarge,
}

impl IconSize {
    pub const fn value(self) -> f32 {
        match self {
            Self::Indicator => 10.0,
            Self::XSmall => 12.0,
            Self::Small => 14.0,
            Self::Medium => 16.0,
            Self::Large => 18.0,
            Self::XLarge => 20.0,
            Self::XXLarge => 48.0,
        }
    }

    pub fn rems(self) -> Rems {
        units(self.value())
    }
}

pub mod size {
    pub const PANEL: f32 = 360.0;
    pub const DEVICE_PANEL: f32 = 400.0;
    pub const LAUNCHER: f32 = 600.0;
    pub const LAUNCHER_OFFSET: f32 = 100.0;
    pub const APPLET_HEIGHT: f32 = 440.0;
    pub const PANEL_MAX_HEIGHT: f32 = 780.0;
    pub const PANEL_MIN_WIDTH: f32 = 200.0;
    pub const PANEL_MIN_HEIGHT: f32 = 160.0;
    pub const VIEWPORT_INSET: f32 = 48.0;
    pub const TILE: f32 = 56.0;
    pub const CONTROL: f32 = 32.0;
    pub const PERCENTAGE_LABEL: f32 = 36.0;
    pub const APPLICATION_ICON: f32 = 28.0;
    pub const NAVIGATION_LIST: f32 = 240.0;
    pub const BAR: f32 = 28.0;
    pub const BAR_CHIP: f32 = 22.0;
    pub const BAR_HIDDEN_STRIP: f32 = 2.0;
    pub const SLIDER_HITBOX: f32 = 16.0;
    pub const SLIDER_TRACK: f32 = 4.0;
    pub const SLIDER_THUMB: f32 = 10.0;
    pub const SWITCH_WIDTH: f32 = 32.0;
    pub const SWITCH_HEIGHT: f32 = 18.0;
    pub const SWITCH_KNOB: f32 = 12.0;
    pub const COVER: f32 = 72.0;
    pub const PLAYBACK: f32 = 40.0;
}

pub mod motion {
    use super::Duration;

    pub const FAST: Duration = Duration::from_millis(140);
    pub const BAR: Duration = Duration::from_millis(180);

    pub fn ease_out_cubic(delta: f32) -> f32 {
        1.0 - (1.0 - delta).powi(3)
    }

    pub fn ease_out_quint(delta: f32) -> f32 {
        1.0 - (1.0 - delta).powi(5)
    }

    pub fn fast_cubic() -> gpui::Motion {
        use gpui::DurationWithEasing;
        FAST.with_easing(ease_out_cubic)
    }

    pub fn fast_quint() -> gpui::Motion {
        use gpui::DurationWithEasing;
        FAST.with_easing(ease_out_quint)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn units_scale_with_the_font() {
        assert_eq!(units_to_pixels(13.0, 15.0), px(15.0));
        assert_eq!(Spacing::Medium.pixels(13.0), px(8.0));
        assert_eq!(units(26.0), rems(2.0));
    }

    #[test]
    fn text_never_drops_below_the_minimum() {
        assert_eq!(TextSize::XSmall.pixels(15.0), px(12.0));
        assert_eq!(TextSize::Small.pixels(15.0), px(13.0));
        assert_eq!(TextSize::XSmall.pixels(10.0), px(12.0));
        assert_eq!(TextSize::Large.pixels(15.0), px(18.0));
    }

    #[test]
    fn easing_ends_at_the_target() {
        assert_eq!(motion::ease_out_cubic(1.0), 1.0);
        assert_eq!(motion::ease_out_quint(0.0), 0.0);
        assert!(motion::ease_out_quint(0.5) > motion::ease_out_cubic(0.5));
    }
}
