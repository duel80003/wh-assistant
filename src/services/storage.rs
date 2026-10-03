use std::path::{Path, PathBuf};
use std::sync::RwLock;
use sha2::{Digest, Sha256};
use anyhow::{Context, Result};
use chrono::Local;

static CUSTOM_IMAGES_DIR: RwLock<Option<PathBuf>> = RwLock::new(None);

pub struct StorageService;

impl StorageService {
    /// Configure or clear a custom (cloud) directory for managed images
    pub fn set_custom_images_dir(dir: Option<PathBuf>) {
        if let Ok(mut lock) = CUSTOM_IMAGES_DIR.write() {
            *lock = dir;
        }
    }

    /// Get current custom images directory if set
    pub fn get_custom_images_dir() -> Option<PathBuf> {
        CUSTOM_IMAGES_DIR.read().ok().and_then(|lock| lock.clone())
    }

    /// Check if cloud/custom images storage is currently active
    #[allow(dead_code)]
    pub fn is_custom_images_dir_active() -> bool {
        Self::get_custom_images_dir().is_some()
    }

    /// Get the default root application data directory (e.g. ~/Library/Application Support/whassistant on macOS)
    pub fn get_app_data_dir() -> PathBuf {
        let base = dirs::data_local_dir().unwrap_or_else(|| PathBuf::from("."));
        base.join("whassistant")
    }

    /// Get SQLite database file path: `<app_data_dir>/whassistant.db`
    pub fn get_db_path() -> PathBuf {
        let dir = Self::get_app_data_dir();
        std::fs::create_dir_all(&dir).ok();
        dir.join("whassistant.db")
    }

    /// Default native local image storage directory: `<app_data_dir>/receipts/images`
    pub fn get_default_images_dir() -> PathBuf {
        let dir = Self::get_app_data_dir().join("receipts").join("images");
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    /// Get active managed image storage directory.
    /// If a custom/cloud directory is active, returns that directory; otherwise returns the default local directory.
    pub fn get_images_dir() -> PathBuf {
        if let Some(custom) = Self::get_custom_images_dir() {
            if custom.is_dir() || std::fs::create_dir_all(&custom).is_ok() {
                return custom;
            }
        }
        Self::get_default_images_dir()
    }

    /// Resolve a relative image filename into full absolute path.
    /// Seamless fallback: first checks the active images dir, then checks the default local dir!
    pub fn resolve_image_path(relative_filename: &str) -> PathBuf {
        if relative_filename.is_empty() {
            return Self::get_images_dir().join("");
        }
        let active_path = Self::get_images_dir().join(relative_filename);
        if active_path.exists() {
            return active_path;
        }
        let default_path = Self::get_default_images_dir().join(relative_filename);
        if default_path.exists() {
            return default_path;
        }
        active_path
    }

    /// Copy all existing local images into the custom/cloud directory
    pub fn copy_all_images_to_custom_dir(explicit_target: Option<&std::path::Path>) -> Result<usize> {
        let target_dir = match explicit_target {
            Some(dir) => dir.to_path_buf(),
            None => match Self::get_custom_images_dir() {
                Some(dir) => dir,
                None => anyhow::bail!("尚未設定或啟用雲端圖片目錄！"),
            },
        };
        std::fs::create_dir_all(&target_dir)?;

        let source_dir = Self::get_default_images_dir();
        let mut count = 0;
        if source_dir.is_dir() {
            for entry in std::fs::read_dir(&source_dir)?.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let dest = target_dir.join(entry.file_name());
                    if !dest.exists() {
                        std::fs::copy(&path, &dest)?;
                        count += 1;
                    }
                }
            }
        }
        Ok(count)
    }

    /// Compute SHA-256 hex string from bytes
    pub fn compute_sha256(bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        format!("{:x}", hasher.finalize())
    }

    /// Save image bytes into the managed storage directory with collision-resistant naming
    pub fn save_image_bytes(bytes: &[u8], original_ext: &str) -> Result<(String, String)> {
        let hash = Self::compute_sha256(bytes);
        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let ext = if original_ext.starts_with('.') {
            original_ext.trim_start_matches('.')
        } else if original_ext.is_empty() {
            "jpg"
        } else {
            original_ext
        };
        let hash_prefix = if hash.len() >= 8 { &hash[..8] } else { &hash };
        let filename = format!("{}_{}.{}", timestamp, hash_prefix, ext);
        let target_path = Self::get_images_dir().join(&filename);

        std::fs::write(&target_path, bytes)
            .with_context(|| format!("Failed to write image file to {:?}", target_path))?;

        Ok((filename, hash))
    }

    /// Copy a file from source path into the managed image directory
    pub fn copy_image_from_file(source_path: &Path) -> Result<(String, String, i64)> {
        let bytes = std::fs::read(source_path)
            .with_context(|| format!("Failed to read source image {:?}", source_path))?;
        let size = bytes.len() as i64;
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg");
        let (filename, hash) = Self::save_image_bytes(&bytes, ext)?;
        Ok((filename, hash, size))
    }

    /// Delete an image file from the managed storage
    pub fn delete_image_file(relative_filename: &str) -> Result<()> {
        if relative_filename.is_empty() {
            return Ok(());
        }
        let active_path = Self::get_images_dir().join(relative_filename);
        if active_path.exists() {
            let _ = std::fs::remove_file(&active_path);
        }
        let default_path = Self::get_default_images_dir().join(relative_filename);
        if default_path.exists() {
            let _ = std::fs::remove_file(&default_path);
        }
        Ok(())
    }

    /// Read image file and convert to base64 Data URL for webview rendering
    pub fn read_image_as_data_url(relative_filename: &str) -> Option<String> {
        if relative_filename.is_empty() {
            return None;
        }
        let full_path = Self::resolve_image_path(relative_filename);
        let bytes = std::fs::read(&full_path).ok()?;
        let ext = full_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("jpg")
            .to_lowercase();
        let mime = match ext.as_str() {
            "png" => "image/png",
            "webp" => "image/webp",
            _ => "image/jpeg",
        };
        use base64::engine::general_purpose::STANDARD as BASE64;
        use base64::Engine;
        let b64 = BASE64.encode(&bytes);
        Some(format!("data:{};base64,{}", mime, b64))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_images_dir_and_fallback() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp_custom = std::env::temp_dir().join(format!("wh_test_custom_img_{}", nanos));
        std::fs::create_dir_all(&temp_custom).unwrap();

        assert!(!StorageService::is_custom_images_dir_active());

        // Set custom directory
        StorageService::set_custom_images_dir(Some(temp_custom.clone()));
        assert!(StorageService::is_custom_images_dir_active());
        assert_eq!(StorageService::get_images_dir(), temp_custom);

        // Test fallback resolution:
        // Case 1: File only exists in default local dir -> resolve should return default path
        let default_dir = StorageService::get_default_images_dir();
        std::fs::create_dir_all(&default_dir).unwrap();
        let test_file_name = format!("test_fallback_{}.jpg", nanos);
        let default_file = default_dir.join(&test_file_name);
        std::fs::write(&default_file, b"default_content").unwrap();

        let resolved = StorageService::resolve_image_path(&test_file_name);
        assert_eq!(resolved, default_file);

        // Case 2: File exists in custom cloud dir -> resolve returns custom path
        let custom_file = temp_custom.join(&test_file_name);
        std::fs::write(&custom_file, b"custom_content").unwrap();
        let resolved_custom = StorageService::resolve_image_path(&test_file_name);
        assert_eq!(resolved_custom, custom_file);

        // Test copy_all_images_to_custom_dir
        let copied = StorageService::copy_all_images_to_custom_dir(Some(&temp_custom)).unwrap();
        let _ = copied;

        // Clean up
        let _ = std::fs::remove_file(&default_file);
        let _ = std::fs::remove_dir_all(&temp_custom);
        StorageService::set_custom_images_dir(None);
        assert!(!StorageService::is_custom_images_dir_active());
    }
}
