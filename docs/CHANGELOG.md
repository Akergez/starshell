# Changelog

All notable changes to Capsule are documented in this file.

## v2.1.0

### Features
- Added expandable Usage and Power orbs anchored to the corners beside Capsule, using the same motion as Capsule.
- Added live CPU, RAM, and GPU usage with dedicated icons and usage indicators.
- Added Power actions for shutting down, rebooting, logging out, and locking the screen.
- Added an Orbits section in Settings Appearance with persistent switches for Power, Usage, Record, and Shelf.
- Integrated orb expansion and contraction into the existing Capsule animation preview.
o
### Improvements
- Added a shared color transition for theme, dark/light mode, and dynamic wallpaper palette changes across Capsule, Orbits, and Settings.
- Limited usage sampling to the expanded panel and stopped requesting animation frames after transitions finish.
- Replaced the clipboard header title with the search bar beside Clear and Close.
- Added English and Spanish labels for orb controls and Power actions.

### Bug Fixes
- Routed logout through the Compositor trait, using the native Hyprland, Niri, and Mango IPC crates and validating compositor acknowledgements.
- Used compositor IPC logout paths instead of forcibly terminating standalone Wayland sessions.
- Prevented duplicate image entries from repeated clipboard writes, equivalent image encodings, and existing general clipboard watchers.
- Fixed Power and Usage orb clicks failing to start or close animations while GPUI was processing a window event.
- Fixed the MangoWM output listener failing to compile because a Wayland read guard was held across an asynchronous wait.
- Fixed excessive idle CPU usage on MangoWM by clearing stale Wayland socket readiness after empty reads and waiting for new output events.
- Kept expanded orbs inside the available viewport and preserved their corner anchors during interrupted transitions.
- Reported failed shutdown, reboot, and logout commands instead of treating unsuccessful exit statuses as successful actions.
- Fixed Settings window clipping on smaller displays and incorrect sizing after screen resolution changes.
- Fixed copying images from Clipboard on Wayland, preserving the image selection and reporting copy failures.
