
<div align="center">

[![Arch Linux](https://img.shields.io/badge/Arch_Linux-supported-1793d1?logo=archlinux&logoColor=white)](https://archlinux.org/)
[![NixOS](https://img.shields.io/badge/NixOS-supported-5277c3?logo=nixos&logoColor=white)](https://nixos.org/)

[![Niri](https://img.shields.io/badge/Niri-supported-5e81ac?logo=wayland&logoColor=white)](https://github.com/YaLTeR/niri)
[![Hyprland](https://img.shields.io/badge/Hyprland-supported-58e1ff?logo=wayland&logoColor=white)](https://github.com/hyprwm/Hyprland)
[![MangoWM](https://img.shields.io/badge/MangoWM-supported-f2a65a?logo=wayland&logoColor=white)](https://github.com/DreamMaoMao/mangowc)

</div>

<div align="center">

# Starshell

A Wayland desktop shell built with GPUI (the Zed UI framework): a bar, a control center, applets and a launcher.
</div>

## Scope
Starshell is an all-in-one desktop shell designed to be lightweight, responsive and GPU-accelerated.

In a typical tiling Wayland setup you end up gluing together a bar, a launcher, a notification daemon, a power menu, a clipboard manager and shell scripts. Starshell replaces them with one process: a bar on every display whose modules open applets right next to where you click, a control center with quick toggles that expand in place, and a launcher driven by pop-launcher.

The interface follows the [Starshell design specification](starshell-design/README.md): every size is a token that scales with the theme font, the palette is derived from a single accent color, and every interaction is animated unless reduce motion is on.

## Features
### Bar
- **Configurable modules**: `launcher`, `workspaces`, `media`, `clock`, `tray`, `notifications`, `clipboard`, `recorder`, `shelf`, `system`, `audio`, `displays` and `control-center`, arranged in `[bar.layout]`.
- **One applet at a time**: an applet opens at the click point, never covers the bar, and closes on Escape or an outside click.
- **Auto-hide**: optionally keep only a thin strip that reveals the bar on hover.

```toml
[bar]
height = 28            # design units, scaled by the theme font size
edge = "top"           # or "bottom"
autohide = false
autohide_delay_ms = 500

[bar.layout]
left = ["launcher", "workspaces", "media"]
middle = ["clock"]
right = ["recorder", "tray", "notifications", "system", "audio", "displays", "control-center"]
```

### Control Center
- **Battery header** with time left or until full, and Sleep, Reboot and Power off actions.
- **Tiles** for Wi-Fi, Bluetooth, VPN, Performance, Microphone and Camera. The body toggles; the chevron expands networks, devices, VPN profiles or power profiles in place.
- **Sliders** for output volume and every controllable display.

### Applets
- **Audio**: outputs, per-application volume and output, inputs.
- **Displays**: brightness over the backlight and DDC/CI, optionally together.
- **Media, System, Calendar** and a **notification center** with actions, inline replies and do not disturb.
- **Clipboard, Emoji, Shelf, Recorder, Themes and Wallpapers**.

### Launcher
- Backed by **pop-launcher**: applications, open windows, files, calculator and every installed plugin. Tab completes, right click shows actions, Escape clears and then closes.

### Session
- Notification popups, a Polkit authentication agent, a lock screen and a settings window.
- A **Starshell** login session: pick it in your display manager and it starts niri and Starshell as systemd user services, runs XDG autostart entries and restarts the shell if it crashes.

## Theming & Templates
Starshell includes a declarative theming engine that automatically synchronizes your system colors across your terminals, compositor, shell, and file managers in real time without external template dependencies.

### 1. Creating Custom Themes
Themes are stored in `~/.config/starshell/themes/presets/<Name>.toml`. Any `.toml` file placed here is automatically discovered by Starshell and selectable via the quick settings menu:

```toml
mode = "Dark" # "Dark" | "Light"
font_family = "Geist"

[background_color]
hex = "#121210"

[background_color_alt]
hex = "#181818"

[surface_color]
hex = "#242424"

[foreground_color]
hex = "#FFFFFF"

[foreground_color_muted]
hex = "#AAAAAA"

[accent_color]
hex = "#007BFF"

[red_color]
hex = "#FF3B30"

[green_color]
hex = "#34C759"
```

### 2. Template Plugins
Templates live in `~/.config/starshell/templates/` (either as `<app>.toml` or `<app>/template.toml`). When a theme changes, Starshell renders all active templates and updates your target configs.

A template definition supports:
- **`target`**: Destination path (`~` and `$XDG_CONFIG_HOME` are automatically expanded).
- **`replace_section`**: For INI/TOML files where themes must live inside an existing file (e.g. `foot.ini`). Replaces only that section (e.g. `[colors-{{mode}}]`) while preserving non-color keys like `alpha`, `blur`, and custom keybindings.
- **`start_marker` / `end_marker`**: In-place replacement between specific comment markers for arbitrary files.
- **`hook`**: Automatically injects an `include` or `require` line into your main config if it's missing.
- **`reload`**: Shell command executed after writing the template.

#### Example: Terminal with In-Place Section Replacement (`foot.toml`)
```toml
name = "foot"
description = "Foot terminal theme"
enabled = true
target = "~/.config/foot/foot.ini"
replace_section = "[colors-{{mode}}]"

[template]
content = """
[colors-{{mode}}]
background={{background.strip}}
foreground={{foreground.strip}}
regular0={{palette.0.strip}}
regular1={{palette.1.strip}}
...
bright7={{palette.15.strip}}
"""
```

#### Example: Hook Injection (`hyprland.toml`)
```toml
name = "hyprland"
description = "Hyprland borders and colors"
enabled = true
target = "~/.config/hypr/starshell_colors.lua"
reload = "hyprctl reload"

[hook]
file = "~/.config/hypr/hyprland.lua"
contains = "require(\"starshell_colors\")"
inject = "colors = require(\"starshell_colors\")\n"

[template]
content = """
return {
    active_border = "rgb({{accent.strip}})",
    inactive_border = "rgb({{background_alt.strip}})",
    surface = "rgb({{surface.strip}})"
}
"""
```

### 3. Template Engine Syntax & Context
You can use the following variables inside any template:

| Variable | Format / Description | Example |
| :--- | :--- | :--- |
| `{{mode}}` | Current mode (`dark` or `light`) | `dark` |
| `{{font_family}}` | Theme font name | `Geist` |
| `{{<color>}}` | Standard 6-digit hex color with `#` | `#007BFF` |
| `{{<color>.strip}}` | Hex color without `#` (for Foot, Fish, Alacritty) | `007BFF` |
| `{{<color>.rgb}}` | Comma-separated RGB channels | `0, 123, 255` |
| `{{<color>.rgba}}` | CSS rgba string | `rgba(0, 123, 255, 1.0)` |
| `{{<color>.qt}}` | ARGB hex color for Qt | `#ff007bff` |
| `{{palette.0}}` .. `{{palette.15}}` | Full 16-color ANSI palette with modifiers | `{{palette.1.strip}}` |

Conditionals are also supported:
```ini
{% if is_dark %}
# Dark mode configuration
{% else %}
# Light mode configuration
{% endif %}
```

## Installation
```bash
curl -fsSL https://raw.githubusercontent.com/moiCR/Capsule/master/install.sh | bash
```

To install the latest beta pre-release instead:
```bash
curl -fsSL https://raw.githubusercontent.com/moiCR/Capsule/master/install.sh | bash -s -- --beta
```

Or build from source:
```bash
git clone https://github.com/moiCR/Capsule.git
cd Capsule
cargo build --release
```

To add the login session by hand, install the files from `crates/installer/session`:
```bash
sudo install -Dm644 crates/installer/session/starshell.desktop /usr/share/wayland-sessions/starshell.desktop
sudo install -Dm755 crates/installer/session/starshell-session /usr/local/bin/starshell-session
sudo install -Dm644 crates/installer/session/starshell.service /usr/local/lib/systemd/user/starshell.service
sudo install -Dm644 crates/installer/session/starshell-session.target /usr/local/lib/systemd/user/starshell-session.target
```
Inside a plain niri session, start the shell with `spawn-at-startup "systemctl" "--user" "start" "starshell.service"`.

## Requirements
Starshell is a Wayland desktop shell and expects a real Wayland runtime with layer-shell support, not a bare X11 setup.

- A supported Wayland compositor such as Niri, Hyprland, Sway or MangoWM.
- `pop-launcher` for the launcher.
- NetworkManager (`nmcli`) for Wi-Fi and VPN; `nm-connection-editor` to edit VPN profiles.
- `brightnessctl` for the built-in display, and optionally `ddcutil` for external monitors.
- PAM development headers to build (`pam-devel` on Fedora, `libpam0g-dev` on Debian).
- PipeWire and WirePlumber enabled and running.
- The `wpctl`, `pactl`, and `parec` utilities available in `PATH`.
- At least one visible audio device (sink/source) exposed by the system.
- If you are testing inside a VM, you will usually need a virtual audio device or a host with real audio passthrough; otherwise the audio widgets will stay empty.

### Nix

The provided flake supports `x86_64-linux` and `aarch64-linux`.

```bash
nix run github:moiCR/Capsule
nix build github:moiCR/Capsule#starshell
```

For a NixOS or Home Manager configuration, add this input to your flake:

```bash
inputs.starshell.url = "github:moiCR/Capsule";
inputs.starshell.inputs.nixpkgs.follows = "nixpkgs";
```

For NixOS:

```nix
environment.systemPackages = [ inputs.starshell.packages.${pkgs.system}.default ];
```

Or with Home Manager:

```nix
home.packages = [ inputs.starshell.packages.${pkgs.system}.default ];
```

## Special Thanks
- [zed](https://github.com/zed-industries/zed): for creating gpui and making it open source
- [tide-island](https://github.com/enhaoswen/Tide-island): for the inspiration
- [saneAspect](https://www.youtube.com/@saneAspect): for the inspiration (Your course is way too expensive, bro.)
- [Akira Toriyama](https://en.wikipedia.org/wiki/Akira_Toriyama): for creating Dragon Ball, inspiring millions around the world, and giving me a childhood I'll never forget.
