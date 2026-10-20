use gpui::{
    ColorExt, Context, FontWeight, IntoElement, ParentElement, Styled, div, prelude::*, px,
};
use ui::theme::Theme;
use ui::tokens::{Spacing, TextSize, units};

use crate::state::LockState;
use crate::view::LockScreen;

pub fn render_lyrics(
    theme: &Theme,
    active_idx: usize,
    anim_progress: f32,
    cx: &mut Context<LockScreen>,
) -> impl IntoElement {
    let (active_track, lyrics) = {
        let state = cx.global::<LockState>();
        let track = state.mpris.get_current_track();
        if track.has_media {
            let lyrics_opt = state
                .lyrics
                .get_cached_lyrics(&track.title, &track.artist)
                .flatten();
            (track, lyrics_opt)
        } else {
            (track, None)
        }
    };

    let lyrics = match lyrics {
        Some(l) if !l.synced_lines.is_empty() => l,
        _ => return div().into_any_element(),
    };

    if !active_track.has_media {
        return div().into_any_element();
    }

    let p = 1.0 - anim_progress;
    let eased_t = 1.0 - p * p * p;

    let start_idx = active_idx.saturating_sub(1);
    let end_idx = (start_idx + 3).min(lyrics.synced_lines.len());

    let mut lines_col = div()
        .flex()
        .flex_col()
        .items_center()
        .gap(Spacing::Small.rems())
        .w_full();

    for (i, line) in lyrics.synced_lines[start_idx..end_idx].iter().enumerate() {
        let actual_idx = start_idx + i;
        let is_active = actual_idx == active_idx;
        let is_past = actual_idx < active_idx;

        if line.text.is_empty() {
            continue;
        }

        let opacity = if is_active {
            0.45 + 0.55 * eased_t
        } else if is_past {
            (0.55 - 0.25 * eased_t).max(0.3)
        } else {
            0.4
        };

        let fraction = if is_active {
            TextSize::Default.fraction()
                + (TextSize::Medium.fraction() - TextSize::Default.fraction()) * eased_t
        } else {
            TextSize::Small.fraction()
        };
        let font_size = (theme.font_size * fraction).max(ui::tokens::MINIMUM_TEXT_SIZE);

        let y_offset = if is_active {
            Spacing::XSmall.value() * (1.0 - eased_t)
        } else {
            0.0
        };

        lines_col = lines_col.child(
            div()
                .w_full()
                .text_center()
                .font_family(theme.font_family())
                .font_weight(if is_active {
                    FontWeight::BOLD
                } else {
                    FontWeight::MEDIUM
                })
                .text_size(px(font_size))
                .text_color(if is_active {
                    theme.accent()
                } else if is_past {
                    theme.text_muted()
                } else {
                    theme.text()
                })
                .opacity(opacity)
                .relative()
                .top(units(y_offset))
                .truncate()
                .child(line.text.clone()),
        );
    }

    div()
        .id("lockscreen-lyrics")
        .flex()
        .flex_col()
        .items_center()
        .w_full()
        .px(Spacing::XXLarge.rems())
        .py(Spacing::Large.rems())
        .rounded_full()
        .bg(theme.elevated().opacity(0.35))
        .border_1()
        .border_color(theme.elevated().opacity(0.25))
        .overflow_hidden()
        .child(lines_col)
        .into_any_element()
}
