use std::{
    hash::{Hash, Hasher},
    path::PathBuf,
};
use ui::theme::theme_manager::{ThemeItem, ThemeManager};

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct WallpaperEntry {
    pub name: String,
    pub path: PathBuf,
    pub thumbnail: Option<PathBuf>,
}
pub(super) struct Catalog {
    pub themes: Vec<ThemeItem>,
    pub wallpapers: Vec<WallpaperEntry>,
}
impl Catalog {
    pub fn load() -> Self {
        let mut wallpapers = Vec::new();
        for directory in wallpaper_directories() {
            collect_images(&directory, 2, &mut wallpapers);
        }
        wallpapers.sort_by_key(|item| item.name.to_lowercase());
        wallpapers.dedup_by(|a, b| a.path == b.path);
        Self {
            themes: ThemeManager::read_presets(),
            wallpapers,
        }
    }
}

fn wallpaper_directories() -> Vec<PathBuf> {
    let mut directories = Vec::new();
    if let Some(home) = dirs::home_dir() {
        directories.push(home.join("Wallpapers"));
    }
    if let Some(pictures) = dirs::picture_dir() {
        directories.push(pictures.join("Wallpapers"));
    }
    if let Some(data) = dirs::data_dir() {
        directories.push(data.join("wallpapers"));
    }
    directories
}

fn is_image(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "webp"
            )
        })
}

fn collect_images(directory: &std::path::Path, depth: usize, output: &mut Vec<WallpaperEntry>) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            if depth > 0 {
                collect_images(&path, depth - 1, output);
            }
            continue;
        }
        if !is_image(&path) {
            continue;
        }
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        let thumbnail = thumbnail(&path);
        output.push(WallpaperEntry {
            name,
            path,
            thumbnail,
        });
    }
}

fn thumbnail(path: &PathBuf) -> Option<PathBuf> {
    let metadata = std::fs::metadata(path).ok()?;
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    path.hash(&mut hash);
    metadata.modified().ok()?.hash(&mut hash);
    metadata.len().hash(&mut hash);
    let directory = dirs::cache_dir()?.join("starshell/wallpapers");
    std::fs::create_dir_all(&directory).ok()?;
    let output = directory.join(format!("{:x}.png", hash.finish()));
    if !output.exists() {
        image::ImageReader::open(path)
            .ok()?
            .with_guessed_format()
            .ok()?
            .decode()
            .ok()?
            .thumbnail(280, 160)
            .save(&output)
            .ok()?;
    }
    Some(output)
}
