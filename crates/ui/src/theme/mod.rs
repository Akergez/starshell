pub mod animator;
pub mod templates;
pub mod theme_manager;
use gpui::{Hsla, Pixels, Rgba, SharedString, hsla_to_rgba, px, rgb_to_hsla};
use serde::{Deserialize, Serialize};
pub use templates::{
    AppTheme, GtkApps, QtApps, TemplateEngine, TemplatePlugin, TemplatePluginManager,
};

pub const DEFAULT_ACCENT: &str = "#70B8FF";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Theme {
    pub mode: ThemeMode,
    #[serde(skip)]
    pub light_mode_mix: Option<f32>,
    #[serde(default = "default_font_family")]
    pub font_family: String,
    #[serde(default = "default_font_size")]
    pub font_size: f32,
    pub background_color: Color,
    pub background_color_alt: Color,
    pub surface_color: Color,
    pub foreground_color: Color,
    pub foreground_color_muted: Color,
    pub accent_color: Color,
    pub red_color: Color,
    pub green_color: Color,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warning_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hover_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub border_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selection_color: Option<Color>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub on_accent_color: Option<Color>,
}

fn default_font_family() -> String {
    "Geist".to_string()
}

fn default_font_size() -> f32 {
    crate::tokens::DEFAULT_FONT_SIZE
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ThemeMode {
    Light,
    Dark,
}

struct Tone {
    hue_shift: f32,
    saturation: f32,
    lightness: f32,
}

const fn tone(hue_shift: f32, saturation: f32, lightness: f32) -> Tone {
    Tone {
        hue_shift,
        saturation,
        lightness,
    }
}

struct Tones {
    background: Tone,
    surface: Tone,
    elevated: Tone,
    hover: Tone,
    border: Tone,
    text: Tone,
    muted: Tone,
    accent: Tone,
    on_accent: Tone,
    selection: Tone,
    warning: Tone,
    error: Tone,
    success: Tone,
}

const DARK_TONES: Tones = Tones {
    background: tone(0.0, 0.19, 0.066),
    surface: tone(0.0, 0.18, 0.103),
    elevated: tone(0.0, 0.18, 0.145),
    hover: tone(0.0, 0.22, 0.19),
    border: tone(0.0, 0.18, 0.255),
    text: tone(0.0, 0.28, 0.94),
    muted: tone(0.0, 0.16, 0.66),
    accent: tone(0.0, 1.0, 0.72),
    on_accent: tone(0.0, 0.55, 0.105),
    selection: tone(0.0, 0.40, 0.19),
    warning: tone(-175.0, 0.82, 0.70),
    error: tone(140.0, 0.70, 0.74),
    success: tone(-80.0, 0.50, 0.66),
};

const LIGHT_TONES: Tones = Tones {
    background: tone(0.0, 0.19, 0.955),
    surface: tone(0.0, 0.20, 0.99),
    elevated: tone(0.0, 0.18, 0.935),
    hover: tone(0.0, 0.22, 0.885),
    border: tone(0.0, 0.18, 0.82),
    text: tone(0.0, 0.28, 0.12),
    muted: tone(0.0, 0.12, 0.40),
    accent: tone(0.0, 0.90, 0.45),
    on_accent: tone(0.0, 0.40, 0.98),
    selection: tone(0.0, 0.60, 0.885),
    warning: tone(-175.0, 0.80, 0.38),
    error: tone(140.0, 0.65, 0.45),
    success: tone(-80.0, 0.55, 0.33),
};

impl Default for Theme {
    fn default() -> Self {
        Self::from_accent(&Color::from(DEFAULT_ACCENT), true)
    }
}

impl Theme {
    pub fn from_accent(accent: &Color, dark: bool) -> Self {
        let hue = accent.to_hsla().hue.into_positive_degrees();
        let tones = if dark { &DARK_TONES } else { &LIGHT_TONES };
        let color = |tone: &Tone| hsl_color(hue + tone.hue_shift, tone.saturation, tone.lightness);
        Theme {
            mode: if dark {
                ThemeMode::Dark
            } else {
                ThemeMode::Light
            },
            light_mode_mix: None,
            font_family: default_font_family(),
            font_size: default_font_size(),
            background_color: color(&tones.background),
            background_color_alt: color(&tones.surface),
            surface_color: color(&tones.elevated),
            foreground_color: color(&tones.text),
            foreground_color_muted: color(&tones.muted),
            accent_color: color(&tones.accent),
            red_color: color(&tones.error),
            green_color: color(&tones.success),
            warning_color: Some(color(&tones.warning)),
            hover_color: Some(color(&tones.hover)),
            border_color: Some(color(&tones.border)),
            selection_color: Some(color(&tones.selection)),
            on_accent_color: Some(color(&tones.on_accent)),
        }
    }

    pub fn light_mix(&self) -> f32 {
        self.light_mode_mix
            .unwrap_or(if self.mode == ThemeMode::Light {
                1.0
            } else {
                0.0
            })
    }

    pub fn with_mode(self, dark: bool) -> Self {
        let mode = if dark {
            ThemeMode::Dark
        } else {
            ThemeMode::Light
        };
        if self.mode == mode {
            return self;
        }
        let mut theme = Self::from_accent(&self.accent_color, dark);
        theme.accent_color = self.accent_color;
        theme.on_accent_color = None;
        theme.font_family = self.font_family;
        theme.font_size = self.font_size;
        theme
    }

    pub fn from_wallpaper(rgb: [u8; 3], dark: bool, font: String) -> Self {
        let accent = Color::new(format!("#{:02X}{:02X}{:02X}", rgb[0], rgb[1], rgb[2]));
        let mut theme = Self::from_accent(&accent, dark);
        theme.font_family = font;
        theme
    }

    pub fn font_family(&self) -> SharedString {
        if self.font_family.is_empty() {
            SharedString::from(default_font_family())
        } else {
            SharedString::from(self.font_family.clone())
        }
    }

    pub fn font_size(&self) -> Pixels {
        px(self.font_size.max(crate::tokens::MINIMUM_TEXT_SIZE))
    }

    pub fn background(&self) -> Hsla {
        self.background_color.to_hsla()
    }

    pub fn surface(&self) -> Hsla {
        self.background_color_alt.to_hsla()
    }

    pub fn elevated(&self) -> Hsla {
        self.surface_color.to_hsla()
    }

    pub fn text(&self) -> Hsla {
        self.foreground_color.to_hsla()
    }

    pub fn text_muted(&self) -> Hsla {
        self.foreground_color_muted.to_hsla()
    }

    pub fn accent(&self) -> Hsla {
        self.accent_color.to_hsla()
    }

    pub fn error(&self) -> Hsla {
        self.red_color.to_hsla()
    }

    pub fn success(&self) -> Hsla {
        self.green_color.to_hsla()
    }

    pub fn warning(&self) -> Hsla {
        self.warning_color
            .as_ref()
            .map(Color::to_hsla)
            .unwrap_or_else(|| mix(self.error(), gpui::hsla(40.0 / 360.0, 0.85, 0.6, 1.0), 0.7))
    }

    pub fn ghost_hover(&self) -> Hsla {
        self.hover_color
            .as_ref()
            .map(Color::to_hsla)
            .unwrap_or_else(|| mix(self.elevated(), self.text(), 0.08))
    }

    pub fn ghost_active(&self) -> Hsla {
        mix(self.ghost_hover(), self.text(), 0.08)
    }

    pub fn border_variant(&self) -> Hsla {
        self.border_color
            .as_ref()
            .map(Color::to_hsla)
            .unwrap_or_else(|| mix(self.elevated(), self.text(), 0.15))
    }

    pub fn element_selected(&self) -> Hsla {
        self.selection_color
            .as_ref()
            .map(Color::to_hsla)
            .unwrap_or_else(|| mix(self.elevated(), self.accent(), 0.2))
    }

    pub fn on_accent(&self) -> Hsla {
        if let Some(color) = &self.on_accent_color {
            return color.to_hsla();
        }
        let accent = luminance(self.accent());
        let text = self.text();
        let background = self.background();
        let contrast = |color| {
            let value = luminance(color);
            (accent.max(value) + 0.05) / (accent.min(value) + 0.05)
        };
        let mut on_accent = if contrast(background) > contrast(text) {
            background
        } else {
            text
        };
        on_accent.alpha = 1.0;
        on_accent
    }
}

fn luminance(color: Hsla) -> f32 {
    let color = hsla_to_rgba(color);
    let linear = |channel: f32| {
        if channel <= 0.04045 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(color.red) + 0.7152 * linear(color.green) + 0.0722 * linear(color.blue)
}

pub fn mix(from: Hsla, to: Hsla, amount: f32) -> Hsla {
    let from = hsla_to_rgba(from);
    let to = hsla_to_rgba(to);
    let channel = |a: f32, b: f32| a + (b - a) * amount;
    rgb_to_hsla(Rgba::new(
        channel(from.red, to.red),
        channel(from.green, to.green),
        channel(from.blue, to.blue),
        channel(from.alpha, to.alpha),
    ))
}

fn hsl_color(hue: f32, saturation: f32, lightness: f32) -> Color {
    let rgba = hsla_to_rgba(gpui::hsla(
        hue.rem_euclid(360.0) / 360.0,
        saturation,
        lightness,
        1.0,
    ));
    Color::new(format!(
        "#{:02X}{:02X}{:02X}",
        (rgba.red * 255.0).round() as u8,
        (rgba.green * 255.0).round() as u8,
        (rgba.blue * 255.0).round() as u8
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Color {
    pub hex: String,
}

impl Color {
    pub fn new(hex: impl Into<String>) -> Self {
        Self { hex: hex.into() }
    }

    pub fn to_hsla(&self) -> Hsla {
        parse_hex_to_hsla(&self.hex)
    }
}

impl From<&str> for Color {
    fn from(s: &str) -> Self {
        Color::new(s)
    }
}

impl From<String> for Color {
    fn from(s: String) -> Self {
        Color::new(s)
    }
}

pub fn parse_hex_to_hsla(hex: &str) -> Hsla {
    let hex = hex.trim_start_matches('#');
    let (r, g, b, a) = match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
            (r, g, b, 255)
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[3..4].repeat(2), 16).unwrap_or(255);
            (r, g, b, a)
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            (r, g, b, 255)
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).unwrap_or(0);
            let g = u8::from_str_radix(&hex[2..4], 16).unwrap_or(0);
            let b = u8::from_str_radix(&hex[4..6], 16).unwrap_or(0);
            let a = u8::from_str_radix(&hex[6..8], 16).unwrap_or(255);
            (r, g, b, a)
        }
        _ => (0, 0, 0, 255),
    };

    rgb_to_hsla(Rgba::new(
        r as f32 / 255.0,
        g as f32 / 255.0,
        b as f32 / 255.0,
        a as f32 / 255.0,
    ))
}

impl gpui::Global for Theme {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_mode_is_independent_of_the_selected_palette() {
        let theme = Theme {
            accent_color: Color::from("#B23388"),
            ..Theme::default()
        };
        let light = theme.clone().with_mode(false);
        assert_eq!(light.mode, ThemeMode::Light);
        assert_eq!(light.accent_color, theme.accent_color);
        assert_ne!(light.background_color, theme.background_color);
        let dark = light.with_mode(true);
        assert_eq!(dark.mode, ThemeMode::Dark);
        assert_eq!(dark.accent_color, theme.accent_color);
    }

    #[test]
    fn dynamic_palette_changes_with_wallpaper_and_respects_mode() {
        let warm = Theme::from_wallpaper([210, 70, 40], true, "Geist".into());
        let cool = Theme::from_wallpaper([40, 70, 210], true, "Geist".into());
        let light = Theme::from_wallpaper([210, 70, 40], false, "Geist".into());
        assert_ne!(warm.accent_color, cool.accent_color);
        assert_eq!(warm.mode, ThemeMode::Dark);
        assert_eq!(light.mode, ThemeMode::Light);
        assert!(warm.background().lightness < warm.text().lightness);
        assert!(light.background().lightness > light.text().lightness);
    }

    #[test]
    fn light_mode_surfaces_have_distinct_contrast() {
        for theme in [
            Theme::default().with_mode(false),
            Theme::from_wallpaper([100, 180, 120], false, "Geist".into()),
        ] {
            assert_eq!(theme.mode, ThemeMode::Light);
            assert!(theme.surface().lightness > theme.background().lightness);
            assert!(theme.background().lightness > theme.elevated().lightness);
            assert!(theme.elevated().lightness > theme.ghost_hover().lightness);
            assert!(theme.background().lightness - theme.text().lightness >= 0.7);
        }
    }

    #[test]
    fn default_palette_follows_the_accent_formula() {
        let theme = Theme::default();
        assert_eq!(theme.mode, ThemeMode::Dark);
        assert_eq!(theme.font_size, 15.0);
        let hue = theme.accent().hue.into_positive_degrees();
        let background = theme.background();
        assert!((background.hue.into_positive_degrees() - hue).abs() < 2.0);
        assert!((background.lightness - 0.066).abs() < 0.01);
        assert!((theme.elevated().lightness - 0.145).abs() < 0.01);
        assert!((theme.border_variant().lightness - 0.255).abs() < 0.01);
        assert!((theme.text().lightness - 0.94).abs() < 0.01);
        assert!(theme.surface().lightness > theme.background().lightness);
        assert!(theme.elevated().lightness > theme.surface().lightness);
    }

    #[test]
    fn presets_without_derived_roles_still_provide_them() {
        let preset: Theme = toml::from_str(
            "mode = 'Dark'\n[background_color]\nhex = '#121210'\n[background_color_alt]\nhex = '#181818'\n[surface_color]\nhex = '#242424'\n[foreground_color]\nhex = '#FFFFFF'\n[foreground_color_muted]\nhex = '#AAAAAA'\n[accent_color]\nhex = '#007BFF'\n[red_color]\nhex = '#FF3B30'\n[green_color]\nhex = '#34C759'\n",
        )
        .expect("legacy preset");
        assert_eq!(preset.font_size, 15.0);
        assert!(preset.ghost_hover().lightness > preset.elevated().lightness);
        assert!(preset.border_variant().lightness > preset.elevated().lightness);
        assert_ne!(preset.element_selected(), preset.elevated());
    }
}
