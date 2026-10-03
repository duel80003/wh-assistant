use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use anyhow::{Context, Result};
use chrono::Local;
use notify::{Config, Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

use crate::db::Database;
use crate::models::{ProcessedFile, Receipt};
use crate::services::ollama::OllamaService;
use crate::services::storage::StorageService;
use crate::utils::{calculate_due_date, normalize_work_date};

pub struct WatcherService;

impl WatcherService {
    /// Supported image extensions for receipts
    pub fn is_supported_image(path: &Path) -> bool {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let lower = ext.to_lowercase();
            matches!(lower.as_str(), "png" | "jpg" | "jpeg" | "webp")
        } else {
            false
        }
    }

    /// Process a single image file (shared between drag-and-drop / manual pick and directory watcher)
    pub async fn process_image_file(db: &Database, file_path: &Path, force: bool) -> Result<i64> {
        let (saved_filename, hash, size) = StorageService::copy_image_from_file(file_path)?;

        // Deduplication check (skipped if user explicitly chose to force proceed)
        if !force && db.is_file_hash_processed(&hash).await? {
            StorageService::delete_image_file(&saved_filename).ok();
            anyhow::bail!("此圖片先前已經處理過，已為您自動排重。");
        }

        // Read image bytes and call Ollama FIRST
        let full_path = StorageService::resolve_image_path(&saved_filename);
        let img_bytes = std::fs::read(&full_path).with_context(|| "讀取儲存的圖片檔案失敗")?;

        let ollama_url = db.get_setting("ollama_url").await?
            .unwrap_or_else(|| "http://localhost:11434".to_string());
        let ollama_model = db.get_setting("ollama_model").await?
            .unwrap_or_else(|| "llama3.2-vision".to_string());

        let extract_result = OllamaService::extract_receipt(&ollama_url, &ollama_model, &img_bytes).await;

        let now = Local::now().to_rfc3339();
        match extract_result {
            Ok(extracted) => {
                // Ensure ticket number is extracted
                let receipt_no = match extracted.no {
                    Some(no) => no,
                    None => {
                        StorageService::delete_image_file(&saved_filename).ok();
                        anyhow::bail!("未能自圖片中辨識出工單號碼（NO.），工單號碼不可為空！請確認圖片清晰度後重新上傳。");
                    }
                };

                // Check for uniqueness before inserting
                if db.check_receipt_no_exists(receipt_no, None).await? {
                    StorageService::delete_image_file(&saved_filename).ok();
                    anyhow::bail!("工單號碼 {} 已存在於資料庫中，不可重複建立！", receipt_no);
                }

                let default_term = db.get_default_payment_term().await?.unwrap_or_else(|| {
                    crate::models::PaymentTerm {
                        id: 1,
                        name: "月結 30 天".to_string(),
                        duration_code: "30d".to_string(),
                        duration_days: 30,
                        is_default: true,
                        description: None,
                        created_at: Local::now().to_rfc3339(),
                    }
                });

                let today = Local::now().format("%Y-%m-%d").to_string();
                let final_work_date = extracted
                    .work_date
                    .map(|d| normalize_work_date(&d))
                    .unwrap_or(today);
                let final_due_date = calculate_due_date(&final_work_date, default_term.duration_days);

                let new_receipt = Receipt {
                    id: 0,
                    no: Some(receipt_no),
                    matainer: extracted.matainer,
                    work_date: final_work_date,
                    due_date: final_due_date,
                    total_amount: extracted.total_amount.unwrap_or(0.0),
                    currency: "TWD".to_string(),
                    image_path: saved_filename,
                    status: "unconfirmed".to_string(),
                    payment_status: "unpaid".to_string(),
                    payment_term_id: Some(default_term.id),
                    paid_at: None,
                    error_message: None,
                    created_at: now.clone(),
                    updated_at: now.clone(),
                };

                let receipt_id = db.insert_receipt(&new_receipt).await?;

                // Record into processed_files
                let pf = ProcessedFile {
                    id: 0,
                    file_path: file_path.to_string_lossy().to_string(),
                    file_hash: hash,
                    file_size: size,
                    receipt_id: Some(receipt_id),
                    status: "success".to_string(),
                    error_message: None,
                    processed_at: now,
                };
                db.record_processed_file(&pf).await?;

                Ok(receipt_id)
            }
            Err(e) => {
                StorageService::delete_image_file(&saved_filename).ok();
                anyhow::bail!("Ollama 辨識失敗: {:#}", e);
            }
        }
    }

    /// Run background directory watcher loop
    pub async fn start_background_watcher(
        db: Database,
        shutdown_signal: Arc<AtomicBool>,
    ) {
        loop {
            if shutdown_signal.load(Ordering::Relaxed) {
                break;
            }

            // Check if monitoring is enabled in settings
            let enabled = db.get_setting("monitor_enabled").await.unwrap_or(None).unwrap_or_default() == "true";
            let dir_str = db.get_setting("monitor_dir").await.unwrap_or(None).unwrap_or_default();

            if enabled && !dir_str.is_empty() {
                let watch_path = PathBuf::from(&dir_str);
                if watch_path.is_dir() {
                    // Initial scan of directory
                    Self::scan_directory(&db, &watch_path).await;

                    // Setup file watcher channel
                    let (tx, mut rx) = mpsc::channel::<PathBuf>(100);

                    let watcher_res = RecommendedWatcher::new(
                        move |res: notify::Result<Event>| {
                            if let Ok(event) = res {
                                if matches!(event.kind, EventKind::Create(_) | EventKind::Modify(_)) {
                                    for path in event.paths {
                                        if Self::is_supported_image(&path) {
                                            let _ = tx.blocking_send(path);
                                        }
                                    }
                                }
                            }
                        },
                        Config::default(),
                    );

                    if let Ok(mut watcher) = watcher_res {
                        if watcher.watch(&watch_path, RecursiveMode::NonRecursive).is_ok() {
                            // Listen to events for up to 10 seconds before re-checking settings
                            let timeout = tokio::time::sleep(Duration::from_secs(10));
                            tokio::pin!(timeout);

                            loop {
                                tokio::select! {
                                    _ = &mut timeout => {
                                        break;
                                    }
                                    Some(path) = rx.recv() => {
                                        // Wait 500ms for file write to complete
                                        tokio::time::sleep(Duration::from_millis(500)).await;
                                        if path.exists() {
                                            let _ = Self::process_image_file(&db, &path, false).await;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            tokio::time::sleep(Duration::from_secs(5)).await;
        }
    }

    async fn scan_directory(db: &Database, dir: &Path) {
        if let Ok(entries) = std::fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.is_file() && Self::is_supported_image(&path) {
                    let _ = Self::process_image_file(db, &path, false).await;
                }
            }
        }
    }
}
