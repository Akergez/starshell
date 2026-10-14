# Changelog

All notable changes to Capsule are documented in this file.

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
