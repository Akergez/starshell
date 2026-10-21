
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

Starshell is a fork of [Capsule](https://github.com/moiCR/Capsule) with a more traditional design. Capsule's floating island is replaced by a classic bar whose modules open applets, and it keeps Capsule's services, lock screen, notifications and theming engine.

## Scope
Starshell is an all-in-one desktop shell designed to be lightweight, responsive and GPU-accelerated.

In a typical tiling Wayland setup you end up gluing together a bar, a launcher, a notification daemon, a power menu, a clipboard manager and shell scripts. Starshell replaces them with three programs: a bar on every display whose modules open applets right next to where you click, a control center with quick toggles that expand in place, a launcher driven by pop-launcher, and a lock screen that runs apart from the bar.

The interface follows the [Starshell design specification](starshell-design/README.md): every size is a token that scales with the theme font, the palette is derived from a single accent color, and every interaction is animated unless reduce motion is on.

## Features
### Bar
- **Configurable modules**: `launcher`, `workspaces`, `media`, `clock`, `tray`, `notifications`, `clipboard`, `recorder`, `shelf`, `keyboard-layout`, `system`, `audio`, `displays` and `control-center`, arranged in `[bar.layout]`.
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
- **starshell-launcher**, a separate program backed by **pop-launcher**: applications, open windows, files, calculator and every installed plugin. Tab completes, right click shows actions, Escape clears and then closes.

### Session
- Notification popups, a Polkit authentication agent and a settings window.
- **starshell-lock**, a separate program that locks the screen on idle, before sleep and on request. It runs in its own process, so the bar can crash or restart without affecting the lock.

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
Starshell has no release builds yet, so it is built from source. It is three programs: `starshell` (the bar), `starshell-lock` (the lock screen) and `starshell-launcher` (the launcher).

### 1. Build dependencies
Fedora:
```bash
sudo dnf install --setopt=install_weak_deps=False \
  git cargo rust gcc clang pkgconf-pkg-config \
  pam-devel openssl-devel dbus-devel alsa-lib-devel \
  wayland-devel libxkbcommon-devel libxcb-devel fontconfig-devel freetype-devel \
  vulkan-loader mesa-libGL
```

Arch Linux:
```bash
sudo pacman -S --needed base-devel git rust clang pkgconf \
  pam openssl dbus alsa-lib wayland libxkbcommon libxcb fontconfig freetype2 vulkan-icd-loader
```

Debian and Ubuntu:
```bash
sudo apt install build-essential git clang pkg-config curl \
  libpam0g-dev libssl-dev libdbus-1-dev libasound2-dev \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev libfontconfig1-dev libfreetype6-dev libvulkan-dev
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
. "$HOME/.cargo/env"
```

Starshell needs Rust 1.88 or newer. Debian and Ubuntu ship an older one, which is why the commands above install Rust with rustup instead. Check the version:
```bash
cargo --version
```

### 2. Build and install
```bash
git clone https://github.com/Akergez/starshell.git
cd starshell
cargo build --release
sudo install -Dm755 target/release/starshell target/release/starshell-lock target/release/starshell-launcher -t /usr/local/bin
which starshell starshell-lock starshell-launcher
```

The first build takes several minutes. The last command must print three paths in `/usr/local/bin`. Install there and not to `~/.cargo/bin` or `~/.local/bin`: the compositor starts programs with its own `PATH`, which usually does not include your home directory.

### 3. Start with the compositor
Start all three programs when the session starts, and bind keys for the launcher and the lock screen. For niri, in `~/.config/niri/config.kdl`:
```kdl
spawn-at-startup "starshell"
spawn-at-startup "starshell-lock"
spawn-at-startup "starshell-launcher"

binds {
    Mod+Space { spawn "starshell-launcher" "toggle"; }
    Super+Alt+L { spawn "starshell-lock" "lock"; }
}
```

With another compositor, start the same three programs at login and bind the same two commands. Log out and back in, then check that all three are running:
```bash
pgrep -a starshell
```

It must print three lines. For a complete niri desktop with KDE applications, login and portals, see [Minimal Setup](docs/MINIMAL_SETUP.md).

### Updating
In the `starshell` directory:
```bash
git pull
cargo build --release
sudo install -Dm755 target/release/starshell target/release/starshell-lock target/release/starshell-launcher -t /usr/local/bin
```

Then log out and back in. If the previous version was a single `starshell` program, add the `starshell-lock` and `starshell-launcher` lines from step 3 to the compositor config first.

### Uninstalling
```bash
sudo rm /usr/local/bin/starshell /usr/local/bin/starshell-lock /usr/local/bin/starshell-launcher
```

Remove the lines from the compositor config. Settings and themes stay in `~/.config/starshell`.

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

For a complete desktop with niri, KDE applications, login and portals, see [Minimal Setup](docs/MINIMAL_SETUP.md).

### Nix

The provided flake supports `x86_64-linux` and `aarch64-linux`.

```bash
nix run .
nix build .#starshell
```

For a NixOS or Home Manager configuration, add this input to your flake, pointing at your checkout:

```bash
inputs.starshell.url = "path:/path/to/starshell";
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
- [Capsule](https://github.com/moiCR/Capsule): the project Starshell is forked from
- [zed](https://github.com/zed-industries/zed): for creating gpui and making it open source
- [tide-island](https://github.com/enhaoswen/Tide-island): for the inspiration
- [saneAspect](https://www.youtube.com/@saneAspect): for the inspiration (Your course is way too expensive, bro.)
- [Akira Toriyama](https://en.wikipedia.org/wiki/Akira_Toriyama): for creating Dragon Ball, inspiring millions around the world, and giving me a childhood I'll never forget.
