use crate::theme::Theme;
use crate::theme::templates::AppTheme;
use gpui::{Hsla, hsla_to_rgba};
use std::fs;
use std::io;
use std::path::Path;
use std::process::Command;

pub struct QtApps;

fn to_qt_hex(hex: &str) -> String {
    let clean = hex.trim_start_matches('#');
    if clean.len() == 6 {
        format!("#ff{clean}")
    } else if clean.len() == 8 {
        format!("#{clean}")
    } else {
        "#ffffffff".to_string()
    }
}

fn make_color_list(
    bg: &str,
    bg_alt: &str,
    fg: &str,
    fg_muted: &str,
    accent: &str,
    disabled: bool,
) -> String {
    let fg_eff = if disabled { fg_muted } else { fg };
    let accent_eff = if disabled { fg_muted } else { accent };

    let items = vec![
        fg_eff,      // 0: WindowText
        bg,          // 1: Button
        bg_alt,      // 2: Light
        bg_alt,      // 3: Midlight
        bg,          // 4: Dark
        bg_alt,      // 5: Mid
        fg_eff,      // 6: Text
        fg_eff,      // 7: BrightText
        fg_eff,      // 8: ButtonText
        bg,          // 9: Base
        bg,          // 10: Window
        "#ff000000", // 11: Shadow
        accent_eff,  // 12: Highlight
        bg,          // 13: HighlightedText
        accent_eff,  // 14: Link
        accent_eff,  // 15: LinkVisited
        bg_alt,      // 16: AlternateBase
        "#ff000000", // 17: NoRole
        bg,          // 18: ToolTipBase
        fg_eff,      // 19: ToolTipText
        fg_muted,    // 20: PlaceholderText
        accent_eff,  // 21: Accent
    ];

    items
        .into_iter()
        .map(to_qt_hex)
        .collect::<Vec<_>>()
        .join(", ")
}

impl AppTheme for QtApps {
    fn apply_current_theme(&self, theme: &Theme) {
        let bg = &theme.background_color.hex;
        let bg_alt = &theme.background_color_alt.hex;
        let fg = &theme.foreground_color.hex;
        let fg_muted = &theme.foreground_color_muted.hex;
        let accent = &theme.accent_color.hex;

        // 1. Write qt5ct and qt6ct custom.conf
        let active_list = make_color_list(bg, bg_alt, fg, fg_muted, accent, false);
        let inactive_list = make_color_list(bg, bg_alt, fg, fg_muted, accent, false);
        let disabled_list = make_color_list(bg, bg_alt, fg, fg_muted, accent, true);

        let qt_scheme = format!(
            "[ColorScheme]\nactive_colors={active_list}\ndisabled_colors={disabled_list}\ninactive_colors={inactive_list}\n"
        );

        if let Some(config_dir) = dirs::config_dir() {
            for version in ["qt5ct", "qt6ct"] {
                let colors_dir = config_dir.join(version).join("colors");
                let _ = fs::create_dir_all(&colors_dir);
                let _ = fs::write(colors_dir.join("custom.conf"), &qt_scheme);

                let conf_path = config_dir.join(version).join(format!("{version}.conf"));
                let custom_conf_file = colors_dir.join("custom.conf");
                let custom_conf_str = custom_conf_file.to_string_lossy();

                let keys = [
                    ("color_scheme_path", custom_conf_str.as_ref()),
                    ("custom_palette", "true"),
                    ("style", "Fusion"),
                ];
                if let Err(error) = set_ini_keys(&conf_path, "Appearance", &keys) {
                    eprintln!("Failed to update {}: {error}", conf_path.display());
                }
            }
        }

        if let Err(error) = write_kde_color_scheme(theme) {
            eprintln!("Failed to write the KDE color scheme: {error}");
        }

        self.reload_apps();
    }

    fn reload_apps(&self) {
        let _ = Command::new("dbus-send")
            .args([
                "--session",
                "--dest=org.kde.KGlobalSettings",
                "/KGlobalSettings",
                "org.kde.KGlobalSettings.notifyChange",
                "int32:0",
                "int32:0",
            ])
            .status();

        let _ = Command::new("dbus-send")
            .args([
                "--session",
                "--type=signal",
                "/KGlobalSettings",
                "org.kde.KGlobalSettings.notifyChange",
                "int32:0",
                "int32:0",
            ])
            .status();

        let _ = Command::new("qdbus")
            .args([
                "org.kde.KGlobalSettings",
                "/KGlobalSettings",
                "notifyChange",
                "0",
                "0",
            ])
            .status();

        let _ = Command::new("kreadconfig6")
            .args([
                "--file",
                "kdeglobals",
                "--group",
                "General",
                "--key",
                "ColorScheme",
            ])
            .status();
    }
}

const KDE_SCHEME_NAME: &str = "Starshell";

struct ColorSet {
    background: Hsla,
    background_alternate: Hsla,
    foreground: Hsla,
    foreground_inactive: Hsla,
    accent: Hsla,
}

fn kde_rgb(color: Hsla) -> String {
    let rgba = hsla_to_rgba(color);
    let channel = |value: f32| (value.clamp(0.0, 1.0) * 255.0).round() as u8;
    format!(
        "{},{},{}",
        channel(rgba.red),
        channel(rgba.green),
        channel(rgba.blue)
    )
}

fn kde_color_groups(theme: &Theme) -> Vec<(String, Vec<(&'static str, String)>)> {
    let regular = |background: Hsla, background_alternate: Hsla| ColorSet {
        background,
        background_alternate,
        foreground: theme.text(),
        foreground_inactive: theme.text_muted(),
        accent: theme.accent(),
    };
    let sets = [
        ("Window", regular(theme.background(), theme.surface())),
        ("View", regular(theme.background(), theme.surface())),
        ("Button", regular(theme.elevated(), theme.surface())),
        ("Tooltip", regular(theme.surface(), theme.elevated())),
        ("Complementary", regular(theme.surface(), theme.elevated())),
        ("Header", regular(theme.background(), theme.surface())),
        (
            "Header][Inactive",
            regular(theme.background(), theme.surface()),
        ),
        (
            "Selection",
            ColorSet {
                background: theme.accent(),
                background_alternate: theme.element_selected(),
                foreground: theme.on_accent(),
                foreground_inactive: theme.on_accent(),
                accent: theme.on_accent(),
            },
        ),
    ];

    let mut groups: Vec<_> = sets
        .into_iter()
        .map(|(name, set)| {
            let keys = vec![
                ("BackgroundNormal", kde_rgb(set.background)),
                ("BackgroundAlternate", kde_rgb(set.background_alternate)),
                ("ForegroundNormal", kde_rgb(set.foreground)),
                ("ForegroundInactive", kde_rgb(set.foreground_inactive)),
                ("ForegroundActive", kde_rgb(set.accent)),
                ("ForegroundLink", kde_rgb(set.accent)),
                ("ForegroundVisited", kde_rgb(set.accent)),
                ("ForegroundNegative", kde_rgb(theme.error())),
                ("ForegroundNeutral", kde_rgb(theme.warning())),
                ("ForegroundPositive", kde_rgb(theme.success())),
                ("DecorationFocus", kde_rgb(theme.accent())),
                ("DecorationHover", kde_rgb(theme.accent())),
            ];
            (format!("Colors:{name}"), keys)
        })
        .collect();

    // Without these, KDE falls back to effects that dim the text of unfocused views.
    groups.push((
        "ColorEffects:Inactive".to_string(),
        vec![
            ("Enable", "false".to_string()),
            ("ChangeSelectionColor", "false".to_string()),
        ],
    ));
    groups.push((
        "ColorEffects:Disabled".to_string(),
        vec![
            ("ColorEffect", "0".to_string()),
            ("ContrastAmount", "0.65".to_string()),
            ("ContrastEffect", "1".to_string()),
            ("IntensityAmount", "0.1".to_string()),
            ("IntensityEffect", "2".to_string()),
        ],
    ));
    groups.push((
        "WM".to_string(),
        vec![
            ("activeBackground", kde_rgb(theme.background())),
            ("activeForeground", kde_rgb(theme.text())),
            ("inactiveBackground", kde_rgb(theme.surface())),
            ("inactiveForeground", kde_rgb(theme.text_muted())),
        ],
    ));
    groups
}

fn write_kde_color_scheme(theme: &Theme) -> io::Result<()> {
    let groups = kde_color_groups(theme);

    if let Some(data_dir) = dirs::data_dir() {
        let color_schemes_dir = data_dir.join("color-schemes");
        fs::create_dir_all(&color_schemes_dir)?;
        let mut scheme =
            format!("[General]\nName={KDE_SCHEME_NAME}\nColorScheme={KDE_SCHEME_NAME}\n");
        for (group, keys) in &groups {
            scheme.push_str(&format!("\n[{group}]\n"));
            for (key, value) in keys {
                scheme.push_str(&format!("{key}={value}\n"));
            }
        }
        fs::write(
            color_schemes_dir.join(format!("{KDE_SCHEME_NAME}.colors")),
            scheme,
        )?;
    }

    let Some(config_dir) = dirs::config_dir() else {
        return Ok(());
    };

    let kdeglobals = config_dir.join("kdeglobals");
    set_ini_keys(&kdeglobals, "General", &[("ColorScheme", KDE_SCHEME_NAME)])?;
    for (group, keys) in &groups {
        let keys: Vec<_> = keys
            .iter()
            .map(|(key, value)| (*key, value.as_str()))
            .collect();
        set_ini_keys(&kdeglobals, group, &keys)?;
    }

    Ok(())
}

fn set_ini_keys(path: &Path, group: &str, keys: &[(&str, &str)]) -> io::Result<()> {
    let original = match fs::read_to_string(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error),
    };
    let header = format!("[{group}]");
    let mut lines: Vec<String> = original.lines().map(str::to_string).collect();
    let mut missing: Vec<&(&str, &str)> = keys.iter().collect();

    let group_start = lines.iter().position(|line| line.trim() == header);
    match group_start {
        Some(start) => {
            let mut end = lines[start + 1..]
                .iter()
                .position(|line| line.trim_start().starts_with('['))
                .map_or(lines.len(), |offset| start + 1 + offset);
            for line in &mut lines[start + 1..end] {
                let Some((key, _)) = line.split_once('=') else {
                    continue;
                };
                if let Some(index) = missing.iter().position(|(name, _)| *name == key.trim()) {
                    let (name, value) = missing.remove(index);
                    *line = format!("{name}={value}");
                }
            }
            while end > start + 1 && lines[end - 1].trim().is_empty() {
                end -= 1;
            }
            for (offset, (name, value)) in missing.into_iter().enumerate() {
                lines.insert(end + offset, format!("{name}={value}"));
            }
        }
        None => {
            if lines.last().is_some_and(|line| !line.trim().is_empty()) {
                lines.push(String::new());
            }
            lines.push(header);
            lines.extend(
                missing
                    .into_iter()
                    .map(|(name, value)| format!("{name}={value}")),
            );
        }
    }

    let updated = lines.join("\n") + "\n";
    if updated == original {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, updated)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn set_ini_keys_updates_only_the_given_keys() -> io::Result<()> {
        let directory = std::env::temp_dir().join(format!("starshell-ini-{}", std::process::id()));
        fs::create_dir_all(&directory)?;
        let path = directory.join("kdeglobals");
        fs::write(
            &path,
            "[General]\nColorScheme=Breeze\nfont=Geist,10\n\n[Icons]\nTheme=breeze-dark\n",
        )?;

        set_ini_keys(
            &path,
            "General",
            &[("ColorScheme", "Starshell"), ("Name", "x")],
        )?;
        set_ini_keys(&path, "UiSettings", &[("ColorScheme", "Starshell")])?;

        let content = fs::read_to_string(&path)?;
        fs::remove_dir_all(&directory)?;
        assert_eq!(
            content,
            "[General]\nColorScheme=Starshell\nfont=Geist,10\nName=x\n\n[Icons]\nTheme=breeze-dark\n\n[UiSettings]\nColorScheme=Starshell\n"
        );
        Ok(())
    }
}
