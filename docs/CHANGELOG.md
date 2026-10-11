# Changelog

All notable changes to Capsule are documented in this file.

## v2.1.0

### Features
- Added expandable Usage and Power orbs anchored to the corners beside Capsule, using the same motion as Capsule.
- Added live CPU, RAM, and GPU usage with dedicated icons and usage indicators.
- Added Power actions for shutting down, rebooting, logging out, and locking the screen.
- Added an Orbits section in Settings Appearance with persistent switches for Power, Usage, Record, and Shelf.
- Integrated orb expansion and contraction into the existing Capsule animation preview.

### Improvements
- Added a shared color transition for theme, dark/light mode, and dynamic wallpaper palette changes across Capsule, Orbits, and Settings.
- Kept Power and Usage visible when enabled, while Record and Shelf retain their contextual visibility.
- Limited usage sampling to the expanded panel and stopped requesting animation frames after transitions finish.
- Replaced the clipboard header title with the search bar beside Clear and Close.
- Compacted the Usage panel and centered metric icons.
- Replaced expanded orb titles with circular controls that transition to a Close icon.
- Added English and Spanish labels for orb controls and Power actions.

### Bug Fixes
- Routed logout through the Compositor trait, using the native Hyprland, Niri, and Mango IPC crates and validating compositor acknowledgements.
- Used compositor IPC logout paths instead of forcibly terminating standalone Wayland sessions.
- Prevented duplicate image entries from repeated clipboard writes, equivalent image encodings, and existing general clipboard watchers.
- Fixed Power and Usage orb clicks failing to start or close animations while GPUI was processing a window event.
- Fixed the MangoWM output listener failing to compile because a Wayland read guard was held across an asynchronous wait.
- Kept expanded orbs inside the available viewport and preserved their corner anchors during interrupted transitions.
- Reported failed shutdown, reboot, and logout commands instead of treating unsuccessful exit statuses as successful actions.

## v2.0.0

### Features
- Completely rewrote Capsule with a new architecture and redesigned its core systems and user experience.
- Redesigned the Settings experience with integrated navigation, search, and reusable setting controls.
- Added new localized strings for the updated Settings interface in English and Spanish.
- Added a terminal-based (TUI) installer to guide users through the installation process without requiring a graphical interface.
- Added a notification module with support for displaying, dismissing, and replying to notifications.
- Restructured the application runtime around Tokio and added event-loop tests.

### Improvements
- Improved Settings module state handling and window integration.
- Refined styling and layout primitives throughout the rewritten Capsule.
- Improved CPU performance by reducing unnecessary work during normal operation.
- Improved animation performance to make transitions smoother and more responsive.
- Refined the user interface across all modules for a more consistent and polished experience.
- Reduced CPU usage during normal operation.
- Rebuilt the satellite system as part of the Capsule rewrite.
- Migrated `gpui-capsule` from a fork of `gpui` to `gpui-ce` as part of the rewrite.

### Bug Fixes
- Fixed clipboard integration when using `cliphist`. Previously, `cliphist` could return an error, causing Capsule to fall back to text-only clipboard handling; the integration now uses the expected clipboard history behavior.
- Fixed a clipboard issue that prevented items from being copied.
- Fixed the lock screen continuously rechecking authentication.
