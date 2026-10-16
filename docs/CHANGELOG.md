# Changelog

All notable changes to Starshell are documented in this file.

## v3.0.0

Capsule becomes Starshell, rebuilt after the Starshell design specification.

### Features
* **Bar**: a bar on every display with launcher, workspaces, media, clock, tray, notifications, clipboard, recorder, shelf, system, audio, displays and control center modules, configured under `[bar]`, with optional auto-hide.
* **Control center**: battery header with power actions, Wi-Fi, Bluetooth, VPN, Performance, Microphone and Camera tiles whose panels expand in place, and output and display sliders.
* **Applets**: audio with per-application volume and output, displays over backlight and DDC/CI, media player, system usage, calendar, notification center, clipboard, emoji, shelf, recorder, themes, wallpapers and tray menus, one at a time next to where they were opened.
* **Launcher**: driven by pop-launcher, with completion, contextual actions and file, calculator and window plugins.
* **VPN**: NetworkManager VPN and WireGuard profiles can be brought up, taken down and edited.

### Improvements
* Every size is a design token that follows the theme font size, and the palette is derived from a single accent color.
* Hover, press and selection are animated and honor reduce motion.
* Saving the config keeps keys written by other tools.

### Breaking Changes
* The binary, config directory, socket and caches are now named `starshell`.
* The dynamic island and its satellites and orbits are gone, along with their settings.
* The interface is English only; the language setting is removed.

## v2.2.0

### Features
* **New Interactive Elements**: Added standardized buttons, icon buttons, status badges, tag chips, and tooltips throughout the desktop shell.
* **Smart Contrast Reading**: Buttons and highlighted chips now automatically adjust their text color to guarantee crystal-clear contrast and readability, no matter what accent color or theme you pick.
* **Full Keyboard Text Selection**: Text input boxes now fully support standard keyboard shortcuts like `Ctrl+A` to select everything, arrow keys for smooth cursor navigation, and word-level editing.

### Improvements
* **Consistent Visual Experience**: Replaced old disparate buttons across notifications, emoji picker, media controls, and security prompts with unified components and smooth hover animations.
* **Sharper Appearance Previews**: Harmonized the miniature layout preview cards in the Appearance settings for a cleaner, pixel-aligned look.
* **Organized & Faster Background Services**: Cleaned up and modularized internal system services (audio, brightness, screen recording, and notifications) into dedicated helpers, improving overall system responsiveness.
* **Freeze Protection for External Tools**: Added safety time limits when communicating with system utilities (such as brightness and sound mixers) so Capsule stays smooth even if an external app lags.
* **Zero Idle Power Consumption**: Guaranteed that desktop animations completely stop running the moment they finish displaying, preventing unnecessary CPU and battery drain.

### Bug Fixes
* Fixed text input boxes not collapsing selection or moving cursors properly when typing over highlighted text.
* Prevented hanging background recording sockets from lingering after stopping a screen capture.
