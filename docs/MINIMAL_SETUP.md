# Minimal Setup

A small desktop built from three parts:

- **niri** manages windows.
- **Starshell** provides the bar, control center, applets, launcher, notifications, lock screen and polkit agent.
- **KDE applications** cover everyday work: Dolphin for files, Kate for text, Okular for documents, Gwenview for images and Ark for archives.

Plasma itself is not installed. The KDE applications run on their own and take their colors from Starshell.

The commands below are for Fedora. Package names differ on other distributions, but the configuration is the same.

## 1. Packages

Install without weak dependencies. Several KDE packages recommend `plasma-workspace`, which in turn pulls in the whole Plasma desktop (on Fedora Asahi it also pulls in Anaconda and the KDE initial setup).

```bash
sudo dnf install --setopt=install_weak_deps=False \
  niri greetd tuigreet \
  pop-launcher wl-clipboard cliphist swaybg brightnessctl \
  pipewire wireplumber pipewire-utils pulseaudio-utils NetworkManager \
  xdg-desktop-portal-gnome xdg-desktop-portal-gtk xdg-desktop-portal-wlr xdg-desktop-portal-kde gnome-keyring \
  dolphin kate okular gwenview ark \
  kio-extras ffmpegthumbs kdegraphics-thumbnailers \
  plasma-integration plasma-breeze-qt6 breeze-icon-theme
```

| Package | Why |
| :--- | :--- |
| `plasma-integration` | The `kde` Qt platform theme. Without it KDE applications start with the default Qt palette and keep black text in places on a dark scheme. It also gives them KDE file dialogs, fonts and icons. |
| `plasma-breeze-qt6` | The Breeze widget style. |
| `xdg-desktop-portal-kde` | KDE file chooser for every application that uses portals (browsers, Flatpak). |
| `kio-extras`, `ffmpegthumbs`, `kdegraphics-thumbnailers` | Thumbnails for images, videos and PDFs in Dolphin. |
| `pop-launcher` | The Starshell launcher backend. |
| `cliphist`, `wl-clipboard` | Clipboard history. |
| `swaybg` | Wallpapers. |
| `brightnessctl` | Built-in display brightness. Add `ddcutil` for external monitors. |

## 2. Build and install Starshell

```bash
sudo dnf install --setopt=install_weak_deps=False \
  git cargo rust gcc clang pkgconf-pkg-config \
  pam-devel openssl-devel dbus-devel alsa-lib-devel \
  wayland-devel libxkbcommon-devel libxcb-devel fontconfig-devel freetype-devel \
  vulkan-loader mesa-libGL

git clone https://github.com/Akergez/starshell.git
cd starshell
cargo build --release
sudo install -Dm755 target/release/starshell target/release/starshell-lock target/release/starshell-launcher -t /usr/local/bin
```

Starshell is three programs, each in its own process:

- `starshell`: bar, applets, control center, notifications and polkit agent.
- `starshell-lock`: locks on idle, before sleep and on request. A problem in the bar never blocks unlocking.
- `starshell-launcher`: the application launcher.

`/usr/local/bin` is in the `PATH` niri starts with, so `spawn-at-startup "starshell"` finds it. A binary left in `target/release` or `~/.cargo/bin` is not found.

## 3. Login

greetd starts the session and tuigreet asks for the user and password. Out of the box greetd runs `agreety`, a plain text login that looks like a bare TTY, so point it at tuigreet:

```bash
sudo sed -i 's|^command = .*|command = "tuigreet --time --remember --remember-session --asterisks --sessions /usr/share/wayland-sessions"|' /etc/greetd/config.toml
grep '^command' /etc/greetd/config.toml
```

The second command must print the `tuigreet` line. The resulting `/etc/greetd/config.toml`:

```toml
[terminal]
vt = 1

[default_session]
command = "tuigreet --time --remember --remember-session --asterisks --sessions /usr/share/wayland-sessions"
user = "greetd"
```

tuigreet stores the last user and session in `/var/cache/tuigreet`, which must belong to `greetd`. Only one login manager can own `display-manager.service`, so disable the current one (GDM, SDDM, COSMIC Greeter, ...) before enabling greetd:

```bash
sudo install -d -o greetd -g greetd /var/cache/tuigreet
sudo systemctl disable display-manager.service
sudo systemctl enable greetd.service
systemctl status display-manager.service --no-pager | head -3
```

The last command must show `greetd.service`. The new login screen appears after a reboot.

## 4. niri

In `~/.config/niri/config.kdl`, set the Qt platform theme, start Starshell and pass the environment to services started over D-Bus:

```kdl
environment {
    QT_QPA_PLATFORMTHEME "kde"
}

spawn-at-startup "starshell"
spawn-at-startup "starshell-lock"
spawn-at-startup "starshell-launcher"
spawn-at-startup "systemctl" "--user" "import-environment" "WAYLAND_DISPLAY" "DISPLAY" "QT_QPA_PLATFORMTHEME"
spawn-at-startup "systemctl" "--user" "restart" "xdg-desktop-portal.service"
```

Applications launched from Starshell inherit this environment. Programs started before the change (for example, from an already open terminal) pick it up after you log in again.

Bind keys for the launcher and the lock screen:

```kdl
binds {
    Mod+Space { spawn "starshell-launcher" "toggle"; }
    Super+Alt+L { spawn "starshell-lock" "lock"; }
}
```

`starshell-lock` locks the screen after `idle_timeout` seconds of inactivity (`[lockscreen]` in the Starshell config), before the system goes to sleep and when logind asks to lock the session (`loginctl lock-session`). Do not run `swayidle` with a lock command as well, or two lock timers will race.

## 5. Portals

`~/.config/xdg-desktop-portal/niri-portals.conf`:

```ini
[preferred]
default=wlr;gnome;gtk;
org.freedesktop.impl.portal.Access=gtk;
org.freedesktop.impl.portal.Notification=gtk;
org.freedesktop.impl.portal.Secret=gnome-keyring;
org.freedesktop.impl.portal.ScreenCast=wlr;
org.freedesktop.impl.portal.Screenshot=wlr;
org.freedesktop.impl.portal.Inhibit=none;
org.freedesktop.impl.portal.FileChooser=kde;
org.freedesktop.impl.portal.Settings=gnome;
```

Restart the portal after editing: `systemctl --user restart xdg-desktop-portal.service`.

## 6. KDE applications

### Style and colors

`~/.config/kdeglobals` needs the Breeze style:

```ini
[KDE]
widgetStyle=Breeze
```

Starshell writes the color scheme itself: it generates `~/.local/share/color-schemes/Starshell.colors` and updates only the color keys in `kdeglobals` whenever the theme changes. Other settings in the file, such as fonts and icons, are kept.

Do not set `widgetStyle=qt6ct-style` unless the `qt6ct` platform theme is in use, or Qt falls back to the bare Windows style.

### "Open with" menu

Outside Plasma there is no applications menu file, so the "Open with" list in Dolphin stays empty. Create `~/.config/menus/applications.menu`:

```xml
<!DOCTYPE Menu PUBLIC "-//freedesktop//DTD Menu 1.0//EN"
 "http://www.freedesktop.org/standards/menu-spec/1.0/menu.dtd">
<Menu>
  <Name>Applications</Name>
  <DefaultAppDirs/>
  <DefaultDirectoryDirs/>
  <Include>
    <All/>
  </Include>
</Menu>
```

Then rebuild the KDE service cache:

```bash
kbuildsycoca6 --noincremental
```

### Default applications

In `~/.config/mimeapps.list`, under `[Default Applications]`:

```ini
inode/directory=org.kde.dolphin.desktop
text/plain=org.kde.kate.desktop
application/pdf=okularApplication_pdf.desktop
```

## 7. Notifications

Starshell is the notification daemon. Uninstall or disable other daemons such as `mako` or `dunst`: whichever starts first owns `org.freedesktop.Notifications`, and Starshell popups will not appear while another daemon holds it.
