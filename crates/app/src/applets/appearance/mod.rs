mod catalog;

use std::path::PathBuf;

use catalog::{Catalog, WallpaperEntry};
use gpui::{
    AnyElement, Context, ElementId, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, Task, Window, div, img, prelude::FluentBuilder, svg,
};
use services::AppState;
use ui::components::{Button, ButtonVariant, ListRow, caption, panel};
use ui::theme::{
    Theme,
    theme_manager::{ThemeItem, ThemeManager},
};
use ui::tokens::{IconSize, Radius, Spacing, TextSize, motion, size, units};

const WALLPAPER_COLUMNS: usize = 3;

fn load_catalog<V: 'static>(
    cx: &mut Context<V>,
    apply: impl Fn(&mut V, Catalog) + 'static,
) -> Task<()> {
    cx.spawn(async move |this, cx| {
        let Ok(catalog) = services::spawn_blocking(Catalog::load).await else {
            return;
        };
        let _ = this.update(cx, |view, cx| {
            apply(view, catalog);
            cx.notify();
        });
    })
}

pub struct ThemesApplet {
    themes: Vec<ThemeItem>,
    loading: bool,
    _load: Task<()>,
}

impl ThemesApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            themes: Vec::new(),
            loading: true,
            _load: load_catalog(cx, |applet: &mut Self, catalog| {
                applet.themes = catalog.themes;
                applet.loading = false;
            }),
        }
    }

    fn apply(theme: ui::theme::Theme, cx: &mut gpui::App) {
        let dark = cx.global::<AppState>().config.get().ui.dark_mode;
        let mut selected = theme;
        selected.font_size = cx.global::<Theme>().font_size;
        let current = selected.clone().with_mode(dark);
        let manager = cx.global_mut::<ThemeManager>();
        manager.selected_theme = selected.clone();
        manager.current_theme = current.clone();
        manager.apply_theme_to_apps();
        if let Err(error) = ThemeManager::save_selected_theme(&selected) {
            eprintln!("Failed to save selected theme: {error}");
        }
        crate::settings::appearance::persist_theme(current.clone(), cx);
        crate::settings::appearance::transition_theme(current, cx);
    }
}

fn swatches(theme: &Theme) -> AnyElement {
    div()
        .flex()
        .flex_shrink_0()
        .rounded(Radius::Small.rems())
        .overflow_hidden()
        .border_1()
        .border_color(theme.border_variant())
        .children(
            [theme.background(), theme.elevated(), theme.accent()].map(|color| {
                div()
                    .size(units(size::BAR_CHIP / 2.0 + Spacing::XSmall.value()))
                    .bg(color)
            }),
        )
        .into_any_element()
}

impl Render for ThemesApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let locked = cx.global::<AppState>().config.get().ui.dynamic_colors;
        let selected = cx.global::<ThemeManager>().selected_theme.clone();
        panel(&theme)
            .w_full()
            .child(div().child("Themes"))
            .when(locked, |panel| {
                panel
                    .child(
                        caption(&theme, "Dynamic colors follow the wallpaper.").whitespace_normal(),
                    )
                    .child(
                        Button::new("themes-unlock")
                            .label("Use a theme instead")
                            .variant(ButtonVariant::Outline)
                            .w_full(true)
                            .on_click(|_, _, cx| {
                                let _ = cx
                                    .global::<AppState>()
                                    .config
                                    .update(|config| config.ui.dynamic_colors = false);
                            }),
                    )
            })
            .when(self.loading, |panel| {
                panel.child(caption(&theme, "Loading…"))
            })
            .when(!locked, |panel| {
                panel.child(
                    div()
                        .id("theme-list")
                        .flex()
                        .flex_col()
                        .gap(Spacing::XXSmall.rems())
                        .max_h(units(size::APPLET_HEIGHT))
                        .overflow_y_scroll()
                        .children(self.themes.iter().enumerate().map(|(index, item)| {
                            let is_selected = item.theme.accent_color == selected.accent_color
                                && item.theme.background_color == selected.background_color;
                            let preset = item.theme.clone();
                            div()
                                .flex()
                                .items_center()
                                .gap(Spacing::Small.rems())
                                .child(swatches(&item.theme))
                                .child(
                                    div().flex_1().min_w_0().child(
                                        ListRow::new(
                                            ElementId::named_usize("theme", index),
                                            item.name.clone(),
                                        )
                                        .status(if item.is_default { "Default" } else { "" })
                                        .selected(is_selected)
                                        .on_click(move |_, _, cx| Self::apply(preset.clone(), cx)),
                                    ),
                                )
                        })),
                )
            })
    }
}

pub struct WallpapersApplet {
    wallpapers: Vec<WallpaperEntry>,
    current: Option<PathBuf>,
    loading: bool,
    error: Option<String>,
    _load: Task<()>,
}

impl WallpapersApplet {
    pub fn new(cx: &mut Context<Self>) -> Self {
        Self {
            wallpapers: Vec::new(),
            current: cx.global::<AppState>().wallpaper.get_current(),
            loading: true,
            error: None,
            _load: load_catalog(cx, |applet: &mut Self, catalog| {
                applet.wallpapers = catalog.wallpapers;
                applet.loading = false;
            }),
        }
    }

    fn set(&mut self, path: PathBuf, cx: &mut Context<Self>) {
        let wallpaper = cx.global::<AppState>().wallpaper.clone();
        let target = path.clone();
        cx.spawn(async move |this, cx| {
            let applied = services::spawn_blocking(move || wallpaper.set_wallpaper(target))
                .await
                .unwrap_or(false);
            let _ = this.update(cx, |applet, cx| {
                if applied {
                    applet.current = Some(path);
                    applet.error = None;
                } else {
                    applet.error = Some("Could not set the wallpaper".to_string());
                }
                cx.notify();
            });
        })
        .detach();
    }
}

impl Render for WallpapersApplet {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let theme = cx.global::<Theme>().clone();
        let hover = theme.ghost_hover();
        let tiles: Vec<AnyElement> = self
            .wallpapers
            .iter()
            .enumerate()
            .map(|(index, entry)| {
                let current = self.current.as_ref() == Some(&entry.path);
                let path = entry.path.clone();
                let preview = entry
                    .thumbnail
                    .clone()
                    .unwrap_or_else(|| entry.path.clone());
                div()
                    .id(ElementId::named_usize("wallpaper", index))
                    .flex_1()
                    .min_w_0()
                    .h(units(size::COVER))
                    .rounded(Radius::Large.rems())
                    .overflow_hidden()
                    .border_2()
                    .border_color(if current {
                        theme.accent()
                    } else {
                        gpui::transparent_black()
                    })
                    .bg(theme.elevated())
                    .cursor_pointer()
                    .hover(move |style| style.border_color(hover))
                    .transitions(|transitions| transitions.border_color(motion::fast_cubic()))
                    .on_click(cx.listener(move |applet, _, _, cx| applet.set(path.clone(), cx)))
                    .child(img(preview).size_full())
                    .into_any_element()
            })
            .collect();
        let mut rows = Vec::new();
        let mut tiles = tiles.into_iter().peekable();
        while tiles.peek().is_some() {
            let mut row: Vec<AnyElement> = tiles.by_ref().take(WALLPAPER_COLUMNS).collect();
            while row.len() < WALLPAPER_COLUMNS {
                row.push(div().flex_1().into_any_element());
            }
            rows.push(div().flex().gap(Spacing::Small.rems()).children(row));
        }

        panel(&theme)
            .w_full()
            .child(div().child("Wallpapers"))
            .when(self.loading, |panel| {
                panel.child(caption(&theme, "Loading…"))
            })
            .when(!self.loading && rows.is_empty(), |panel| {
                panel.child(
                    div()
                        .flex()
                        .items_center()
                        .gap(Spacing::Small.rems())
                        .child(
                            svg()
                                .path("wallpaper.svg")
                                .size(IconSize::Medium.rems())
                                .text_color(theme.text_muted()),
                        )
                        .child(caption(&theme, "Add images to ~/Wallpapers")),
                )
            })
            .when(!rows.is_empty(), |panel| {
                panel.child(
                    div()
                        .id("wallpaper-grid")
                        .flex()
                        .flex_col()
                        .gap(Spacing::Small.rems())
                        .max_h(units(size::APPLET_HEIGHT))
                        .overflow_y_scroll()
                        .children(rows),
                )
            })
            .when_some(self.error.clone(), |panel, error| {
                panel.child(
                    div()
                        .text_size(TextSize::Small.pixels(theme.font_size))
                        .text_color(theme.error())
                        .child(error),
                )
            })
    }
}
