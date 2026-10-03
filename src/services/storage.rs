use std::path::{Path, PathBuf};
use sha2::{Digest, Sha256};
use anyhow::{Context, Result};
use chrono::Local;

pub struct StorageService;

impl StorageService {
    /// Get the root application data directory (e.g. ~/Library/Application Support/whassistant on macOS)
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

    /// Get managed image storage directory: `<app_data_dir>/receipts/images`
    pub fn get_images_dir() -> PathBuf {
        let dir = Self::get_app_data_dir().join("receipts").join("images");
        std::fs::create_dir_all(&dir).ok();
        dir
    }

    /// Resolve a relative image filename into full absolute path
    pub fn resolve_image_path(relative_filename: &str) -> PathBuf {
        Self::get_images_dir().join(relative_filename)
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
        let full_path = Self::resolve_image_path(relative_filename);
        if full_path.exists() {
            std::fs::remove_file(&full_path)
                .with_context(|| format!("Failed to delete image {:?}", full_path))?;
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
