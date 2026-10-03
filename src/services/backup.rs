use anyhow::{Context, Result};
use chrono::Local;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::db::Database;
use crate::services::storage::StorageService;

pub struct BackupService;

impl BackupService {
    /// Safely create a live SQLite database snapshot using VACUUM INTO
    /// Target file can be located anywhere (e.g. Google Drive, OneDrive, or local folder)
    pub async fn backup_db_snapshot(db: &Database, target_file: &Path) -> Result<()> {
        if let Some(parent) = target_file.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("無法建立備份目錄 {:?}", parent))?;
        }

        let parent = target_file.parent().unwrap_or_else(|| Path::new("."));
        let temp_filename = format!(
            ".whbackup_{}_{}.tmp",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let temp_file = parent.join(temp_filename);

        if temp_file.exists() {
            let _ = std::fs::remove_file(&temp_file);
        }

        // SQLite VACUUM INTO requires non-existent destination file
        let escaped_path = temp_file.to_string_lossy().replace('\'', "''");
        let sql = format!("VACUUM INTO '{}'", escaped_path);

        sqlx::raw_sql(&sql)
            .execute(&db.pool)
            .await
            .with_context(|| format!("執行備份快照失敗: {}", sql))?;

        // Atomically replace target file
        if target_file.exists() {
            let _ = std::fs::remove_file(target_file);
        }

        std::fs::rename(&temp_file, target_file).or_else(|_| {
            std::fs::copy(&temp_file, target_file).and_then(|_| std::fs::remove_file(&temp_file))
        })?;

        Ok(())
    }

    /// Export a full standalone backup ZIP archive containing the clean DB and all receipt images
    pub async fn export_full_backup_zip(db: &Database, target_zip: &Path) -> Result<(u64, usize)> {
        if let Some(parent) = target_zip.parent() {
            std::fs::create_dir_all(parent).ok();
        }

        // 1. Generate clean DB snapshot in temp file
        let temp_db = StorageService::get_app_data_dir().join(format!(
            ".export_{}_{}.tmp",
            std::process::id(),
            Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        Self::backup_db_snapshot(db, &temp_db).await?;

        // 2. Create ZIP file
        let file = File::create(target_zip)
            .with_context(|| format!("無法建立備份壓縮檔 {:?}", target_zip))?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        // 3. Add database file to zip root
        zip.start_file("whassistant.db", options)?;
        let mut db_file = File::open(&temp_db)?;
        std::io::copy(&mut db_file, &mut zip)?;
        drop(db_file);
        let _ = std::fs::remove_file(&temp_db);

        // 4. Add metadata file
        let meta = serde_json::json!({
            "app_version": env!("CARGO_PKG_VERSION"),
            "exported_at": Local::now().to_rfc3339(),
            "os": std::env::consts::OS,
        });
        zip.start_file("whassistant_meta.json", options)?;
        zip.write_all(meta.to_string().as_bytes())?;

        // 5. Add all receipt images from active and default local directory
        let mut added_files = std::collections::HashSet::new();
        let mut images_count = 0usize;

        for dir in [
            StorageService::get_images_dir(),
            StorageService::get_default_images_dir(),
        ] {
            if dir.is_dir() {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        let path = entry.path();
                        if path.is_file() {
                            let file_name = entry.file_name().to_string_lossy().to_string();
                            if !added_files.contains(&file_name) {
                                if let Ok(mut img_f) = File::open(&path) {
                                    zip.start_file(format!("images/{}", file_name), options)?;
                                    std::io::copy(&mut img_f, &mut zip)?;
                                    added_files.insert(file_name);
                                    images_count += 1;
                                }
                            }
                        }
                    }
                }
            }
        }

        zip.finish()?;
        let file_size = std::fs::metadata(target_zip)?.len();

        Ok((file_size, images_count))
    }

    /// Restore full backup from ZIP archive: restores database and extracts all images
    pub fn restore_full_backup_zip(source_zip: &Path) -> Result<usize> {
        let file =
            File::open(source_zip).with_context(|| format!("無法開啟備份檔 {:?}", source_zip))?;
        let mut archive = ZipArchive::new(file)?;

        let temp_restore = StorageService::get_app_data_dir().join(".restore_staging");
        if temp_restore.exists() {
            let _ = std::fs::remove_dir_all(&temp_restore);
        }
        std::fs::create_dir_all(&temp_restore)?;

        archive
            .extract(&temp_restore)
            .with_context(|| "解壓備份檔案時發生錯誤")?;

        let extracted_db = temp_restore.join("whassistant.db");
        if !extracted_db.exists() {
            let _ = std::fs::remove_dir_all(&temp_restore);
            anyhow::bail!("選取的備份壓縮檔中未包含合法的 whassistant.db 資料庫檔案！");
        }

        // 1. Restore images
        let extracted_images = temp_restore.join("images");
        let target_images = StorageService::get_images_dir();
        let mut restored_images_count = 0usize;
        if extracted_images.is_dir() {
            for entry in std::fs::read_dir(&extracted_images)?.flatten() {
                let path = entry.path();
                if path.is_file() {
                    let dest = target_images.join(entry.file_name());
                    std::fs::copy(&path, &dest)?;
                    restored_images_count += 1;
                }
            }
        }

        // 2. Overwrite database
        let target_db = StorageService::get_db_path();
        let target_wal = target_db.with_extension("db-wal");
        let target_shm = target_db.with_extension("db-shm");
        let _ = std::fs::remove_file(&target_wal);
        let _ = std::fs::remove_file(&target_shm);

        std::fs::copy(&extracted_db, &target_db).with_context(|| "置換目標資料庫失敗")?;

        let _ = std::fs::remove_dir_all(&temp_restore);

        Ok(restored_images_count)
    }

    /// Restore database directly from a .db snapshot file (e.g. from cloud sync folder)
    pub fn restore_db_snapshot_file(source_db: &Path) -> Result<()> {
        if !source_db.is_file() {
            anyhow::bail!("選取的資料庫檔案不存在！");
        }
        let meta = std::fs::metadata(source_db)?;
        if meta.len() < 100 {
            anyhow::bail!("選取的檔案不是有效的資料庫檔案！");
        }

        let target_db = StorageService::get_db_path();
        let target_wal = target_db.with_extension("db-wal");
        let target_shm = target_db.with_extension("db-shm");
        let _ = std::fs::remove_file(&target_wal);
        let _ = std::fs::remove_file(&target_shm);

        std::fs::copy(source_db, &target_db).with_context(|| "置換目標資料庫失敗")?;

        Ok(())
    }

    /// Restore from either a .zip full backup package or a .db database snapshot
    pub fn restore_any_backup(source_path: &Path) -> Result<(usize, bool)> {
        let ext = source_path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase();

        if ext == "db" {
            Self::restore_db_snapshot_file(source_path)?;
            Ok((0, false))
        } else {
            let count = Self::restore_full_backup_zip(source_path)?;
            Ok((count, true))
        }
    }

    /// Automatically trigger cloud directory snapshot if enabled in settings
    pub async fn trigger_auto_cloud_backup(db: &Database) -> Result<Option<PathBuf>> {
        let enabled = db
            .get_setting("auto_backup_enabled")
            .await?
            .unwrap_or_default()
            == "true";
        let target_dir_str = db.get_setting("auto_backup_dir").await?.unwrap_or_default();

        if enabled && !target_dir_str.trim().is_empty() {
            let target_dir = PathBuf::from(target_dir_str.trim());
            if target_dir.is_dir() {
                let target_file = target_dir.join("whassistant_snapshot.db");
                Self::backup_db_snapshot(db, &target_file).await?;
                let now_str = Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                let _ = db.set_setting("last_backup_time", &now_str).await;
                return Ok(Some(target_file));
            }
        }

        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_backup_db_snapshot() {
        let db = Database::init().await.expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!(
            "wh_test_backup_{}",
            Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_file = temp_dir.join("test_snapshot.db");

        let res = BackupService::backup_db_snapshot(&db, &target_file).await;
        assert!(res.is_ok(), "VACUUM INTO failed: {:?}", res);
        assert!(target_file.exists());
        assert!(std::fs::metadata(&target_file).unwrap().len() > 0);

        // Test overwrite snapshot
        let res2 = BackupService::backup_db_snapshot(&db, &target_file).await;
        assert!(res2.is_ok(), "Overwriting snapshot failed: {:?}", res2);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_export_backup_zip() {
        let db = Database::init().await.expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!(
            "wh_test_zip_{}",
            Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_zip = temp_dir.join("test_backup.zip");

        let res = BackupService::export_full_backup_zip(&db, &target_zip).await;
        assert!(res.is_ok(), "export_full_backup_zip failed: {:?}", res);
        assert!(target_zip.exists());

        // Verify zip contains whassistant.db
        let file = File::open(&target_zip).unwrap();
        let mut archive = ZipArchive::new(file).unwrap();
        assert!(archive.by_name("whassistant.db").is_ok());
        assert!(archive.by_name("whassistant_meta.json").is_ok());

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[tokio::test]
    async fn test_restore_any_backup() {
        let db = Database::init().await.expect("Failed to init DB");
        let temp_dir = std::env::temp_dir().join(format!(
            "wh_test_restore_{}",
            Local::now().timestamp_nanos_opt().unwrap_or(0)
        ));
        std::fs::create_dir_all(&temp_dir).unwrap();
        let target_snapshot = temp_dir.join("test_restore.db");

        BackupService::backup_db_snapshot(&db, &target_snapshot)
            .await
            .unwrap();

        let res = BackupService::restore_any_backup(&target_snapshot);
        assert!(res.is_ok(), "restore_any_backup on .db failed: {:?}", res);
        let (count, is_zip) = res.unwrap();
        assert_eq!(count, 0);
        assert!(!is_zip);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}
