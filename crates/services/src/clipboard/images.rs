use super::ClipboardItem;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

struct CachedFingerprint {
    modified: Option<SystemTime>,
    size: u64,
    fingerprint: [u8; 32],
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, path: Option<PathBuf>, is_image: bool) -> ClipboardItem {
        ClipboardItem {
            id: id.into(),
            preview: "same preview".into(),
            is_image,
            image_path: path,
        }
    }

    #[test]
    fn equivalent_encodings_keep_the_newest_image_and_preserve_distinct_images_and_text() {
        let directory =
            std::env::temp_dir().join(format!("capsule-dedup-images-{}", std::process::id()));
        std::fs::create_dir_all(&directory).expect("fixture directory");
        let png = directory.join("newest.png");
        let bmp = directory.join("older.bmp");
        let different = directory.join("different.png");
        let resized = directory.join("resized.png");
        let source = image::DynamicImage::new_rgb8(2, 2);
        source.save(&png).expect("PNG");
        source.save(&bmp).expect("BMP");
        image::RgbImage::from_pixel(2, 2, image::Rgb([1, 0, 0]))
            .save(&different)
            .expect("different pixels");
        image::DynamicImage::new_rgb8(1, 4)
            .save(&resized)
            .expect("different dimensions");
        let mut items = vec![
            item("newest", Some(png), true),
            item("older", Some(bmp), true),
            item("different", Some(different), true),
            item("resized", Some(resized), true),
            item("text-one", None, false),
            item("text-two", None, false),
            item("unavailable-image", None, true),
        ];
        let mut cache = HistoryImages::default();
        cache.retain_unique(&mut items);
        assert_eq!(
            items
                .iter()
                .map(|item| item.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "newest",
                "different",
                "resized",
                "text-one",
                "text-two",
                "unavailable-image"
            ]
        );
        cache.retain_unique(&mut items);
        assert_eq!(items.len(), 6);
        std::fs::remove_dir_all(directory).expect("fixture cleanup");
    }

    #[test]
    fn changed_files_are_rechecked_and_removed_entries_release_cached_fingerprints() {
        let path =
            std::env::temp_dir().join(format!("capsule-dedup-change-{}.png", std::process::id()));
        image::DynamicImage::new_rgb8(1, 1)
            .save(&path)
            .expect("initial fixture");
        let mut cache = HistoryImages::default();
        let first = cache.fingerprint(&path).expect("initial fingerprint");
        image::DynamicImage::new_rgb8(4, 3)
            .save(&path)
            .expect("changed fixture");
        assert_ne!(
            cache.fingerprint(&path).expect("changed fingerprint"),
            first
        );
        cache.retain_unique(&mut Vec::new());
        assert!(cache.fingerprints.is_empty());
        std::fs::remove_file(path).expect("fixture cleanup");
    }
}

#[derive(Default)]
pub(super) struct HistoryImages {
    fingerprints: HashMap<PathBuf, CachedFingerprint>,
}

impl HistoryImages {
    pub(super) fn retain_unique(&mut self, items: &mut Vec<ClipboardItem>) {
        let paths: HashSet<_> = items
            .iter()
            .filter(|item| item.is_image)
            .filter_map(|item| item.image_path.as_deref())
            .collect();
        self.fingerprints
            .retain(|path, _| paths.contains(path.as_path()));
        let mut seen = HashSet::new();
        items.retain(|item| {
            !item.is_image
                || item
                    .image_path
                    .as_deref()
                    .and_then(|path| self.fingerprint(path))
                    .is_none_or(|fingerprint| seen.insert(fingerprint))
        });
    }

    fn fingerprint(&mut self, path: &Path) -> Option<[u8; 32]> {
        let metadata = std::fs::metadata(path).ok()?;
        let modified = metadata.modified().ok();
        let size = metadata.len();
        if let Some(cached) = self.fingerprints.get(path)
            && cached.modified == modified
            && cached.size == size
        {
            return Some(cached.fingerprint);
        }
        let bytes = std::fs::read(path).ok()?;
        let image = image::load_from_memory(&bytes).ok()?;
        let mut hash = Sha256::new();
        hash.update(image.width().to_le_bytes());
        hash.update(image.height().to_le_bytes());
        let pixels = image::DynamicImage::ImageRgba16(image.into_rgba16());
        hash.update(pixels.as_bytes());
        let fingerprint = hash.finalize().into();
        self.fingerprints.insert(
            path.to_path_buf(),
            CachedFingerprint {
                modified,
                size,
                fingerprint,
            },
        );
        Some(fingerprint)
    }
}
