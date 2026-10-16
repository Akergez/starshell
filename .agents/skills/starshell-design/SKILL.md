---
name: starshell-design
description: Design and refine Starshell's bar, control center, applets and launcher following the Starshell design specification - token sizes, accent palette roles, expand-in-place panels and animated interactions.
---

# Starshell Design

The specification lives in `starshell-design/README.md`, with reference screenshots next to it. Read it before changing any UI; this skill only summarizes how it maps onto the code.

## Sizes

- Every size is a token from `ui::tokens`: `Spacing`, `Radius`, `TextSize`, `IconSize`, `size::*` and `units(n)` for design units. Never write pixels in markup.
- Design units are relative to a 13-unit reference font; the theme `font_size` (default 15) scales everything. Text never goes below 12 px; `TextSize::pixels` enforces it.
- Platform pixels (window sizes, input regions, exclusive zones) are computed with `units_to_pixels`.

## Color

- Use the palette roles on `Theme`: `background`, `surface` (panel), `elevated` (card, inactive tile), `element_selected`, `ghost_hover`, `ghost_active`, `border_variant`, `accent`, `on_accent`, `text`, `text_muted`, `error`, `warning`, `success`.
- `Theme::from_accent` derives the whole palette from one accent hue.

## Building blocks

- `panel`, `card`, `divider`, `caption`, `section_label` for surfaces.
- `Tile` (body toggles, chevron expands), `ListRow`, `SegmentedControl`, `SliderRow` with `Slider`, `Switch`, `Button` and `IconButton`.
- `reveal` for panels that expand in place (fade and slide by Medium, ease-out quint) and `fade_in` for applets.

## Behavior

- One open applet at a time, placed at the click point, never over the bar; opening another replaces it.
- Details expand in place below what triggered them, at full width, without moving or resizing their row neighbours.
- Hover, press and selection animate over 140 ms with `motion::fast_cubic`; reduce motion makes them instant.
- `render()` stays pure: request focus, input regions and surface changes only from event handlers.
