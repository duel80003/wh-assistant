use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use chrono::{Local, Timelike};

use crate::db::Database;

pub struct SchedulerService;

impl SchedulerService {
    /// Execute startup lifecycle check and run background 10:00 AM daily scheduler
    pub async fn start_background_scheduler(
        db: Database,
        shutdown_signal: Arc<AtomicBool>,
    ) {
        // 1. Startup cleanup check
        Self::run_retention_cleanup(&db).await;

        let mut last_cleanup_day = Local::now().date_naive();

        // 2. Loop and check periodically
        loop {
            if shutdown_signal.load(Ordering::Relaxed) {
                break;
            }

            tokio::time::sleep(Duration::from_secs(60)).await;

            let now = Local::now();
            let today = now.date_naive();

            // Check if it is 10:00 AM (hour == 10 and minute == 0) and we haven't run today
            if now.hour() == 10 && now.minute() == 0 && today != last_cleanup_day {
                Self::run_retention_cleanup(&db).await;
                last_cleanup_day = today;
            }
        }
    }

    pub async fn run_retention_cleanup(db: &Database) -> i64 {
        let retention_days: i64 = db
            .get_setting("retention_days")
            .await
            .unwrap_or(None)
            .and_then(|v| v.parse().ok())
            .unwrap_or(365);

        if retention_days > 0 {
            match db.cleanup_expired_paid_receipts(retention_days).await {
                Ok(count) => {
                    if count > 0 {
                        println!("[Scheduler] 自動清理已完成：共清除 {} 筆過期且已收費的歷史單據及圖片檔案。", count);
                    }
                    count
                }
                Err(e) => {
                    eprintln!("[Scheduler] 執行歷史單據清理失敗: {:#}", e);
                    0
                }
            }
        } else {
            0
        }
    }
}
