use super::{Color, Theme};
use std::time::{Duration, Instant};

pub struct ThemeAnimator {
    current: Theme,
    start: Theme,
    target: Theme,
    started_at: Option<Instant>,
    duration: Duration,
}

impl ThemeAnimator {
    pub fn new(theme: Theme) -> Self {
        Self {
            current: theme.clone(),
            start: theme.clone(),
            target: theme,
            started_at: None,
            duration: Duration::ZERO,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.current
    }

    pub fn transition_to(&mut self, target: Theme, duration: Duration, now: Instant) -> bool {
        if self.target == target {
            return self.started_at.is_some();
        }
        self.start = self.current.clone();
        self.target = target;
        self.duration = duration;
        self.started_at = Some(now);
        self.advance(now)
    }

    pub fn advance(&mut self, now: Instant) -> bool {
        let Some(started_at) = self.started_at else {
            return false;
        };
        let elapsed = now.saturating_duration_since(started_at);
        if elapsed >= self.duration {
            self.current = self.target.clone();
            self.started_at = None;
            return false;
        }
        let t = elapsed.as_secs_f32() / self.duration.as_secs_f32();
        let progress = t * t * (3.0 - 2.0 * t);
        let mut theme = self.target.clone();
        let mix = |start: &Color, target: &Color| interpolate(start, target, progress);
        theme.background_color = mix(&self.start.background_color, &self.target.background_color);
        theme.background_color_alt = mix(
            &self.start.background_color_alt,
            &self.target.background_color_alt,
        );
        theme.surface_color = mix(&self.start.surface_color, &self.target.surface_color);
        theme.foreground_color = mix(&self.start.foreground_color, &self.target.foreground_color);
        theme.foreground_color_muted = mix(
            &self.start.foreground_color_muted,
            &self.target.foreground_color_muted,
        );
        theme.accent_color = mix(&self.start.accent_color, &self.target.accent_color);
        theme.red_color = mix(&self.start.red_color, &self.target.red_color);
        theme.green_color = mix(&self.start.green_color, &self.target.green_color);
        let mix_optional = |start: &Option<Color>, target: &Option<Color>| match (start, target) {
            (Some(start), Some(target)) => Some(interpolate(start, target, progress)),
            _ => target.clone(),
        };
        theme.warning_color = mix_optional(&self.start.warning_color, &self.target.warning_color);
        theme.hover_color = mix_optional(&self.start.hover_color, &self.target.hover_color);
        theme.border_color = mix_optional(&self.start.border_color, &self.target.border_color);
        theme.selection_color =
            mix_optional(&self.start.selection_color, &self.target.selection_color);
        theme.on_accent_color =
            mix_optional(&self.start.on_accent_color, &self.target.on_accent_color);
        theme.light_mode_mix = Some(
            self.start.light_mix() + (self.target.light_mix() - self.start.light_mix()) * progress,
        );
        self.current = theme;
        true
    }
}

fn interpolate(start: &Color, target: &Color, progress: f32) -> Color {
    if progress == 0.0 {
        return start.clone();
    }
    let start = gpui::hsla_to_rgba(start.to_hsla());
    let target = gpui::hsla_to_rgba(target.to_hsla());
    let channel = |a: f32, b: f32| ((a + (b - a) * progress).clamp(0.0, 1.0) * 255.0).round() as u8;
    Color::new(format!(
        "#{:02X}{:02X}{:02X}{:02X}",
        channel(start.red, target.red),
        channel(start.green, target.green),
        channel(start.blue, target.blue),
        channel(start.alpha, target.alpha)
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_and_palette_change_together_and_stop_at_the_target() {
        let now = Instant::now();
        let start = Theme::default();
        let target = start.clone().with_mode(false);
        let mut animator = ThemeAnimator::new(start.clone());
        assert!(animator.transition_to(target.clone(), Duration::from_millis(180), now));
        assert_eq!(animator.theme().background_color, start.background_color);
        assert_eq!(animator.theme().light_mix(), 0.0);
        assert!(animator.advance(now + Duration::from_millis(90)));
        assert_eq!(animator.theme().light_mix(), 0.5);
        assert!(animator.theme().background().lightness > start.background().lightness);
        assert!(!animator.advance(now + Duration::from_millis(180)));
        assert_eq!(animator.theme(), &target);
        assert!(!animator.advance(now + Duration::from_secs(1)));
    }

    #[test]
    fn rapid_changes_continue_from_the_displayed_palette() {
        let now = Instant::now();
        let mut animator = ThemeAnimator::new(Theme::default());
        animator.transition_to(
            Theme::default().with_mode(false),
            Duration::from_millis(180),
            now,
        );
        animator.advance(now + Duration::from_millis(60));
        let displayed = animator.theme().clone();
        let target = Theme::from_wallpaper([220, 90, 40], true, "Geist".into());
        animator.transition_to(
            target.clone(),
            Duration::from_millis(180),
            now + Duration::from_millis(60),
        );
        assert_eq!(
            animator.theme().background_color,
            displayed.background_color
        );
        assert_eq!(animator.theme().light_mix(), displayed.light_mix());
        assert!(!animator.advance(now + Duration::from_millis(240)));
        assert_eq!(animator.theme(), &target);
    }

    #[test]
    fn zero_duration_applies_immediately() {
        let mut animator = ThemeAnimator::new(Theme::default());
        let target = Theme::default().with_mode(false);
        assert!(!animator.transition_to(target.clone(), Duration::ZERO, Instant::now()));
        assert_eq!(animator.theme(), &target);
        assert!(
            !toml::to_string(animator.theme())
                .expect("serialize theme")
                .contains("light_mode_mix")
        );
    }
}
