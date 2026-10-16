use chrono::{Datelike, Local, NaiveDate};
use gpui::{
    Context, ElementId, FontWeight, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    Window, div, prelude::FluentBuilder,
};
use ui::components::{IconButton, panel};
use ui::theme::Theme;
use ui::tokens::{Radius, Spacing, TextSize, motion, size, units};

const WEEKDAYS: [&str; 7] = ["Mo", "Tu", "We", "Th", "Fr", "Sa", "Su"];

pub struct CalendarApplet {
    year: i32,
    month: u32,
}

impl CalendarApplet {
    pub fn new(_cx: &mut Context<Self>) -> Self {
        let today = Local::now().date_naive();
        Self {
            year: today.year(),
            month: today.month(),
        }
    }

    fn shift(&mut self, months: i32, cx: &mut Context<Self>) {
        let index = self.year * 12 + self.month as i32 - 1 + months;
        self.year = index.div_euclid(12);
        self.month = index.rem_euclid(12) as u32 + 1;
        cx.notify();
    }
}

pub fn month_grid(year: i32, month: u32) -> Vec<NaiveDate> {
    let Some(first) = NaiveDate::from_ymd_opt(year, month, 1) else {
        return Vec::new();
    };
    let offset = first.weekday().num_days_from_monday() as i64;
    let start = first - chrono::Duration::days(offset);
    (0..42)
        .map(|day| start + chrono::Duration::days(day))
        .collect()
}

impl Render for CalendarApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let today = Local::now().date_naive();
        let title = NaiveDate::from_ymd_opt(self.year, self.month, 1)
            .map(|date| date.format("%B %Y").to_string())
            .unwrap_or_default();
        let small = TextSize::Small.pixels(theme.font_size);
        let cell = |content: gpui::Div| {
            content
                .flex_1()
                .h(units(size::CONTROL))
                .flex()
                .items_center()
                .justify_center()
                .rounded(Radius::Medium.rems())
        };
        let days = month_grid(self.year, self.month);
        let hover = theme.ghost_hover();

        panel(&theme)
            .w_full()
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(Spacing::XSmall.rems())
                    .child(div().flex_1().font_weight(FontWeight::MEDIUM).child(title))
                    .child(
                        IconButton::new("calendar-previous", "chevron-left.svg")
                            .tooltip("Previous month")
                            .on_click(cx.listener(|applet, _, _, cx| applet.shift(-1, cx))),
                    )
                    .child(
                        IconButton::new("calendar-today", "calendar-days.svg")
                            .tooltip("Today")
                            .on_click(cx.listener(|applet, _, _, cx| {
                                let today = Local::now().date_naive();
                                applet.year = today.year();
                                applet.month = today.month();
                                cx.notify();
                            })),
                    )
                    .child(
                        IconButton::new("calendar-next", "chevron-right.svg")
                            .tooltip("Next month")
                            .on_click(cx.listener(|applet, _, _, cx| applet.shift(1, cx))),
                    ),
            )
            .child(
                div()
                    .flex()
                    .text_size(small)
                    .text_color(theme.text_muted())
                    .children(WEEKDAYS.map(|day| cell(div()).child(day))),
            )
            .child(
                div()
                    .flex()
                    .flex_col()
                    .gap(Spacing::XXSmall.rems())
                    .children(days.chunks(7).enumerate().map(|(week, dates)| {
                        div().flex().gap(Spacing::XXSmall.rems()).children(
                            dates.iter().enumerate().map(|(weekday, date)| {
                                let is_today = *date == today;
                                let in_month = date.month() == self.month;
                                cell(div())
                                    .id(ElementId::named_usize("calendar-day", week * 7 + weekday))
                                    .bg(if is_today {
                                        theme.accent()
                                    } else {
                                        gpui::transparent_black()
                                    })
                                    .text_color(if is_today {
                                        theme.on_accent()
                                    } else if in_month {
                                        theme.text()
                                    } else {
                                        theme.text_muted()
                                    })
                                    .when(!is_today, |cell| {
                                        cell.hover(move |style| style.bg(hover)).transitions(
                                            |transitions| transitions.bg(motion::fast_cubic()),
                                        )
                                    })
                                    .child(date.day().to_string())
                            }),
                        )
                    })),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grid_starts_on_monday_and_covers_six_weeks() {
        let grid = month_grid(2026, 10);
        assert_eq!(grid.len(), 42);
        assert_eq!(grid[0], NaiveDate::from_ymd_opt(2026, 9, 28).expect("date"));
        assert_eq!(grid[3], NaiveDate::from_ymd_opt(2026, 10, 1).expect("date"));
    }
}
