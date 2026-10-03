use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
use sqlx::{Pool, Sqlite};
use anyhow::{Context, Result};
use chrono::Local;
use std::str::FromStr;

use crate::models::{OverdueKpi, PaymentTerm, ProcessedFile, Receipt, ReceiptFilter};
use crate::services::storage::StorageService;

#[derive(Clone)]
pub struct Database {
    pub pool: Pool<Sqlite>,
}

impl Database {
    /// Initialize SQLite connection pool and apply schema migrations
    pub async fn init() -> Result<Self> {
        let db_path = StorageService::get_db_path();
        let db_url = format!("sqlite://{}?mode=rwc", db_path.to_string_lossy());

        let connection_options = SqliteConnectOptions::from_str(&db_url)?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
            .foreign_keys(true);

        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .connect_with(connection_options)
            .await
            .with_context(|| format!("Failed to connect to SQLite at {}", db_url))?;

        let db = Self { pool };
        db.migrate().await?;
        db.seed_defaults().await?;
        Ok(db)
    }

    async fn migrate(&self) -> Result<()> {
        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS payment_terms (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                name TEXT NOT NULL,
                duration_code TEXT NOT NULL,
                duration_days INTEGER NOT NULL,
                is_default INTEGER NOT NULL DEFAULT 0,
                description TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS receipts (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                no INTEGER,
                matainer TEXT,
                work_date TEXT NOT NULL,
                due_date TEXT NOT NULL,
                total_amount REAL NOT NULL DEFAULT 0.0,
                currency TEXT NOT NULL DEFAULT 'TWD',
                image_path TEXT NOT NULL,
                status TEXT NOT NULL DEFAULT 'unconfirmed',
                payment_status TEXT NOT NULL DEFAULT 'unpaid',
                payment_term_id INTEGER,
                paid_at TEXT,
                error_message TEXT,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL,
                FOREIGN KEY (payment_term_id) REFERENCES payment_terms(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS processed_files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                file_path TEXT NOT NULL,
                file_hash TEXT NOT NULL UNIQUE,
                file_size INTEGER NOT NULL,
                receipt_id INTEGER,
                status TEXT NOT NULL,
                error_message TEXT,
                processed_at TEXT NOT NULL,
                FOREIGN KEY (receipt_id) REFERENCES receipts(id) ON DELETE SET NULL
            );

            CREATE TABLE IF NOT EXISTS app_settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );

            CREATE UNIQUE INDEX IF NOT EXISTS idx_receipts_no ON receipts(no) WHERE no IS NOT NULL;
            CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
            CREATE INDEX IF NOT EXISTS idx_receipts_payment_status ON receipts(payment_status);
            CREATE INDEX IF NOT EXISTS idx_receipts_work_date ON receipts(work_date);
            CREATE INDEX IF NOT EXISTS idx_receipts_due_date ON receipts(due_date);
            CREATE INDEX IF NOT EXISTS idx_processed_files_hash ON processed_files(file_hash);
            "#,
        )
        .execute(&self.pool)
        .await
        .context("Failed to run schema migrations")?;

        // Automatic upgrade: If receipts table was created in an older version with `no TEXT`,
        // migrate it seamlessly to `no INTEGER`
        let col_type: Option<(String,)> = sqlx::query_as(
            "SELECT type FROM pragma_table_info('receipts') WHERE name = 'no'"
        )
        .fetch_optional(&self.pool)
        .await
        .ok()
        .flatten();

        if let Some((t,)) = col_type {
            if t.to_uppercase() == "TEXT" {
                sqlx::raw_sql(
                    r#"
                    CREATE TABLE IF NOT EXISTS receipts_new (
                        id INTEGER PRIMARY KEY AUTOINCREMENT,
                        no INTEGER,
                        matainer TEXT,
                        work_date TEXT NOT NULL,
                        due_date TEXT NOT NULL,
                        total_amount REAL NOT NULL DEFAULT 0.0,
                        currency TEXT NOT NULL DEFAULT 'TWD',
                        image_path TEXT NOT NULL,
                        status TEXT NOT NULL DEFAULT 'unconfirmed',
                        payment_status TEXT NOT NULL DEFAULT 'unpaid',
                        payment_term_id INTEGER,
                        paid_at TEXT,
                        error_message TEXT,
                        created_at TEXT NOT NULL,
                        updated_at TEXT NOT NULL,
                        FOREIGN KEY (payment_term_id) REFERENCES payment_terms(id) ON DELETE SET NULL
                    );
                    INSERT INTO receipts_new SELECT 
                        id,
                        CAST(no AS INTEGER),
                        matainer, work_date, due_date, total_amount, currency, image_path,
                        status, payment_status, payment_term_id, paid_at, error_message, created_at, updated_at
                    FROM receipts;
                    DROP TABLE receipts;
                    ALTER TABLE receipts_new RENAME TO receipts;
                    CREATE UNIQUE INDEX IF NOT EXISTS idx_receipts_no ON receipts(no) WHERE no IS NOT NULL;
                    CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
                    CREATE INDEX IF NOT EXISTS idx_receipts_payment_status ON receipts(payment_status);
                    CREATE INDEX IF NOT EXISTS idx_receipts_work_date ON receipts(work_date);
                    CREATE INDEX IF NOT EXISTS idx_receipts_due_date ON receipts(due_date);
                    "#
                )
                .execute(&self.pool)
                .await?;
            }
        }

        Ok(())
    }

    async fn seed_defaults(&self) -> Result<()> {
        let now = Local::now().to_rfc3339();

        // Seed payment terms if empty
        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM payment_terms")
            .fetch_one(&self.pool)
            .await?;

        if count.0 == 0 {
            sqlx::query(
                r#"
                INSERT INTO payment_terms (name, duration_code, duration_days, is_default, description, created_at)
                VALUES 
                    ('即期付款', '0d', 0, 0, '施工完成即刻付款', ?),
                    ('月結 30 天', '30d', 30, 1, '施工次日起 30 天內結清', ?),
                    ('雙月結 60 天', '60d', 60, 0, '施工次日起 60 天內結清', ?),
                    ('季結 90 天', '90d', 90, 0, '施工次日起 90 天內結清', ?)
                "#,
            )
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        // Seed app_settings if not present
        let default_settings = [
            ("ollama_url", "http://localhost:11434"),
            ("ollama_model", "llama3.2-vision"),
            ("retention_days", "365"),
            ("monitor_dir", ""),
            ("monitor_enabled", "false"),
        ];

        for (k, v) in default_settings {
            sqlx::query(
                r#"
                INSERT OR IGNORE INTO app_settings (key, value, updated_at)
                VALUES (?, ?, ?)
                "#,
            )
            .bind(k)
            .bind(v)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        Ok(())
    }

    // --- Receipts Operations ---

    /// Check if a receipt ticket number already exists in the database.
    /// If exclude_id is provided, ignore matching receipt with that ID (useful for updates).
    pub async fn check_receipt_no_exists(&self, no: i64, exclude_id: Option<i64>) -> Result<bool> {
        let count: (i64,) = if let Some(id) = exclude_id {
            sqlx::query_as("SELECT COUNT(*) FROM receipts WHERE no = ? AND id != ?")
                .bind(no)
                .bind(id)
                .fetch_one(&self.pool)
                .await?
        } else {
            sqlx::query_as("SELECT COUNT(*) FROM receipts WHERE no = ?")
                .bind(no)
                .fetch_one(&self.pool)
                .await?
        };
        Ok(count.0 > 0)
    }

    pub async fn insert_receipt(&self, r: &Receipt) -> Result<i64> {
        let receipt_no = match r.no {
            Some(n) => n,
            None => anyhow::bail!("工單號碼（NO.）不可為空！"),
        };
        if self.check_receipt_no_exists(receipt_no, None).await? {
            anyhow::bail!("工單號碼 {} 已存在於資料庫中，不可重複建立！", receipt_no);
        }

        let now = Local::now().to_rfc3339();
        let norm_work_date = crate::utils::normalize_work_date(&r.work_date);
        let norm_due_date = crate::utils::normalize_work_date(&r.due_date);
        let id = sqlx::query(
            r#"
            INSERT INTO receipts (
                no, matainer, work_date, due_date, total_amount, currency, image_path,
                status, payment_status, payment_term_id, paid_at, error_message, created_at, updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(r.no)
        .bind(&r.matainer)
        .bind(&norm_work_date)
        .bind(&norm_due_date)
        .bind(r.total_amount)
        .bind(&r.currency)
        .bind(&r.image_path)
        .bind(&r.status)
        .bind(&r.payment_status)
        .bind(r.payment_term_id)
        .bind(&r.paid_at)
        .bind(&r.error_message)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    pub async fn update_receipt(&self, r: &Receipt) -> Result<()> {
        let receipt_no = match r.no {
            Some(n) => n,
            None => anyhow::bail!("工單號碼（NO.）不可為空！"),
        };
        if self.check_receipt_no_exists(receipt_no, Some(r.id)).await? {
            anyhow::bail!("工單號碼 {} 已存在於其他工單中，不可重複！", receipt_no);
        }

        let now = Local::now().to_rfc3339();
        let norm_work_date = crate::utils::normalize_work_date(&r.work_date);
        let norm_due_date = crate::utils::normalize_work_date(&r.due_date);
        sqlx::query(
            r#"
            UPDATE receipts SET
                no = ?,
                matainer = ?,
                work_date = ?,
                due_date = ?,
                total_amount = ?,
                currency = ?,
                image_path = ?,
                status = ?,
                payment_status = ?,
                payment_term_id = ?,
                paid_at = ?,
                error_message = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(&r.no)
        .bind(&r.matainer)
        .bind(&norm_work_date)
        .bind(&norm_due_date)
        .bind(r.total_amount)
        .bind(&r.currency)
        .bind(&r.image_path)
        .bind(&r.status)
        .bind(&r.payment_status)
        .bind(r.payment_term_id)
        .bind(&r.paid_at)
        .bind(&r.error_message)
        .bind(&now)
        .bind(r.id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn set_payment_status(&self, id: i64, status: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        let paid_at = if status == "paid" {
            Some(now.clone())
        } else {
            None
        };

        sqlx::query(
            r#"
            UPDATE receipts SET
                payment_status = ?,
                paid_at = ?,
                updated_at = ?
            WHERE id = ?
            "#,
        )
        .bind(status)
        .bind(paid_at)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn delete_receipt(&self, id: i64) -> Result<()> {
        // Find image path first to delete file
        let rec: Option<(String,)> = sqlx::query_as("SELECT image_path FROM receipts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;

        if let Some((img,)) = rec {
            StorageService::delete_image_file(&img).ok();
        }

        sqlx::query("DELETE FROM receipts WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn get_receipt_by_id(&self, id: i64) -> Result<Option<Receipt>> {
        let r = sqlx::query_as::<_, Receipt>("SELECT * FROM receipts WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(r)
    }

    pub async fn get_unconfirmed_receipts(&self) -> Result<Vec<Receipt>> {
        let list = sqlx::query_as::<_, Receipt>(
            "SELECT * FROM receipts WHERE status IN ('unconfirmed', 'failed', 'processing') ORDER BY id ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }

    pub async fn get_unconfirmed_count(&self) -> Result<i64> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM receipts WHERE status IN ('unconfirmed', 'failed', 'processing')",
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0)
    }

    pub async fn get_confirmed_receipts(
        &self,
        filter: &ReceiptFilter,
    ) -> Result<(Vec<Receipt>, i64)> {
        let mut conditions = vec!["status = 'confirmed'".to_string()];

        if let Some(ref kw) = filter.keyword {
            let clean = kw.trim();
            if !clean.is_empty() {
                conditions.push(format!(
                    "(CAST(no AS TEXT) LIKE '%{}%' OR matainer LIKE '%{}%')",
                    clean.replace('\'', "''"),
                    clean.replace('\'', "''")
                ));
            }
        }

        if let Some(ref ps) = filter.payment_status {
            if ps != "all" && !ps.is_empty() {
                conditions.push(format!("payment_status = '{}'", ps.replace('\'', "''")));
            }
        }

        if let Some(ref s_date) = filter.start_date {
            if !s_date.is_empty() {
                conditions.push(format!("work_date >= '{}'", s_date.replace('\'', "''")));
            }
        }

        if let Some(ref e_date) = filter.end_date {
            if !e_date.is_empty() {
                conditions.push(format!("work_date <= '{}'", e_date.replace('\'', "''")));
            }
        }

        let where_clause = conditions.join(" AND ");

        let count_query = format!("SELECT COUNT(*) FROM receipts WHERE {}", where_clause);
        let total: (i64,) = sqlx::query_as(&count_query)
            .fetch_one(&self.pool)
            .await?;

        let limit = if filter.page_size > 0 { filter.page_size } else { 10 };
        let offset = if filter.page > 0 { (filter.page - 1) * limit } else { 0 };

        let data_query = format!(
            "SELECT * FROM receipts WHERE {} ORDER BY work_date DESC, id DESC LIMIT {} OFFSET {}",
            where_clause, limit, offset
        );

        let list = sqlx::query_as::<_, Receipt>(&data_query)
            .fetch_all(&self.pool)
            .await?;

        Ok((list, total.0))
    }

    pub async fn get_overdue_receipts(&self) -> Result<Vec<Receipt>> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let list = sqlx::query_as::<_, Receipt>(
            r#"
            SELECT * FROM receipts 
            WHERE payment_status = 'unpaid' 
              AND status = 'confirmed' 
              AND due_date < ? 
            ORDER BY due_date ASC
            "#,
        )
        .bind(&today)
        .fetch_all(&self.pool)
        .await?;

        Ok(list)
    }

    pub async fn get_overdue_kpi(&self) -> Result<OverdueKpi> {
        let today = Local::now().format("%Y-%m-%d").to_string();
        let row: (i64, Option<f64>) = sqlx::query_as(
            r#"
            SELECT COUNT(*), SUM(total_amount) 
            FROM receipts 
            WHERE payment_status = 'unpaid' 
              AND status = 'confirmed' 
              AND due_date < ?
            "#,
        )
        .bind(&today)
        .fetch_one(&self.pool)
        .await?;

        Ok(OverdueKpi {
            total_count: row.0,
            total_amount: row.1.unwrap_or(0.0),
        })
    }

    /// Clean up paid receipts older than retention_days (based on work_date).
    /// Unpaid receipts are NEVER deleted.
    pub async fn cleanup_expired_paid_receipts(&self, retention_days: i64) -> Result<i64> {
        if retention_days <= 0 {
            return Ok(0); // Permanent retention
        }

        let cutoff_date = (Local::now() - chrono::Duration::days(retention_days))
            .format("%Y-%m-%d")
            .to_string();

        // 1. Fetch images to delete
        let expired: Vec<(i64, String)> = sqlx::query_as(
            r#"
            SELECT id, image_path FROM receipts
            WHERE payment_status = 'paid'
              AND status = 'confirmed'
              AND work_date < ?
            "#,
        )
        .bind(&cutoff_date)
        .fetch_all(&self.pool)
        .await?;

        for (_, img) in &expired {
            StorageService::delete_image_file(img).ok();
        }

        // 2. Delete rows
        let result = sqlx::query(
            r#"
            DELETE FROM receipts
            WHERE payment_status = 'paid'
              AND status = 'confirmed'
              AND work_date < ?
            "#,
        )
        .bind(&cutoff_date)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // --- Payment Terms Operations ---

    pub async fn get_payment_terms(&self) -> Result<Vec<PaymentTerm>> {
        let list = sqlx::query_as::<_, PaymentTerm>(
            "SELECT * FROM payment_terms ORDER BY duration_days ASC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(list)
    }

    pub async fn get_default_payment_term(&self) -> Result<Option<PaymentTerm>> {
        let term = sqlx::query_as::<_, PaymentTerm>(
            "SELECT * FROM payment_terms WHERE is_default = 1 LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(term)
    }

    pub async fn insert_payment_term(&self, term: &PaymentTerm) -> Result<i64> {
        let now = Local::now().to_rfc3339();
        if term.is_default {
            sqlx::query("UPDATE payment_terms SET is_default = 0").execute(&self.pool).await?;
        }
        let id = sqlx::query(
            r#"
            INSERT INTO payment_terms (name, duration_code, duration_days, is_default, description, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&term.name)
        .bind(&term.duration_code)
        .bind(term.duration_days)
        .bind(term.is_default)
        .bind(&term.description)
        .bind(&now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();

        Ok(id)
    }

    pub async fn set_default_payment_term(&self, id: i64) -> Result<()> {
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE payment_terms SET is_default = 0").execute(&mut *tx).await?;
        sqlx::query("UPDATE payment_terms SET is_default = 1 WHERE id = ?")
            .bind(id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_payment_term(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM payment_terms WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // --- Processed Files Operations ---

    pub async fn is_file_hash_processed(&self, hash: &str) -> Result<bool> {
        let count: (i64,) = sqlx::query_as(
            "SELECT COUNT(*) FROM processed_files WHERE file_hash = ? AND status = 'success'",
        )
        .bind(hash)
        .fetch_one(&self.pool)
        .await?;
        Ok(count.0 > 0)
    }

    pub async fn record_processed_file(&self, pf: &ProcessedFile) -> Result<()> {
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO processed_files (file_path, file_hash, file_size, receipt_id, status, error_message, processed_at)
            VALUES (?, ?, ?, ?, ?, ?, ?)
            "#,
        )
        .bind(&pf.file_path)
        .bind(&pf.file_hash)
        .bind(pf.file_size)
        .bind(pf.receipt_id)
        .bind(&pf.status)
        .bind(&pf.error_message)
        .bind(&pf.processed_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // --- App Settings Operations ---

    pub async fn get_setting(&self, key: &str) -> Result<Option<String>> {
        let res: Option<(String,)> = sqlx::query_as("SELECT value FROM app_settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        Ok(res.map(|r| r.0))
    }

    pub async fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        let now = Local::now().to_rfc3339();
        sqlx::query(
            r#"
            INSERT OR REPLACE INTO app_settings (key, value, updated_at)
            VALUES (?, ?, ?)
            "#,
        )
        .bind(key)
        .bind(value)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_database_init_and_seed() {
        let db = Database::init().await.expect("Database initialization failed");
        let terms = db.get_payment_terms().await.expect("Failed to get terms");
        assert!(!terms.is_empty(), "Default payment terms should be seeded");
        
        let retention = db.get_setting("retention_days").await.expect("Failed to get retention setting");
        assert_eq!(retention.as_deref(), Some("365"));
    }

    #[tokio::test]
    async fn test_roc_date_conversion_on_insert() {
        let db = Database::init().await.expect("Database initialization failed");
        sqlx::query("DELETE FROM receipts WHERE no = 99990001").execute(&db.pool).await.ok();
        
        let mut test_receipt = Receipt {
            id: 0,
            no: Some(99990001),
            matainer: Some("王工程師".to_string()),
            work_date: "民國113年5月20日".to_string(),
            due_date: "113/06/19".to_string(),
            total_amount: 8888.0,
            currency: "TWD".to_string(),
            image_path: "test.jpg".to_string(),
            status: "unconfirmed".to_string(),
            payment_status: "unpaid".to_string(),
            payment_term_id: None,
            paid_at: None,
            error_message: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        let id = db.insert_receipt(&test_receipt).await.expect("Insert receipt failed");
        assert!(id > 0);

        let retrieved = db.get_receipt_by_id(id).await.expect("Query failed").expect("Receipt not found");
        assert_eq!(retrieved.no, Some(99990001));
        assert_eq!(retrieved.work_date, "2024-05-20", "ROC work_date must be converted to Western YYYY-MM-DD");
        assert_eq!(retrieved.due_date, "2024-06-19", "ROC due_date must be converted to Western YYYY-MM-DD");

        // Test update conversion as well
        test_receipt.id = id;
        test_receipt.work_date = "113-10-10".to_string();
        test_receipt.due_date = "113年11月10日".to_string();
        db.update_receipt(&test_receipt).await.expect("Update failed");

        let updated = db.get_receipt_by_id(id).await.expect("Query failed").expect("Receipt not found");
        assert_eq!(updated.work_date, "2024-10-10", "Updated ROC work_date must be converted to Western YYYY-MM-DD");
        assert_eq!(updated.due_date, "2024-11-10", "Updated ROC due_date must be converted to Western YYYY-MM-DD");

        // Clean up test row
        db.delete_receipt(id).await.ok();
    }

    #[tokio::test]
    async fn test_unique_receipt_no() {
        let db = Database::init().await.expect("Database initialization failed");
        sqlx::query("DELETE FROM receipts WHERE no = 88887777").execute(&db.pool).await.ok();

        let unique_no = 88887777;
        let r1 = Receipt {
            id: 0,
            no: Some(unique_no),
            matainer: Some("測試員A".to_string()),
            work_date: "2024-05-20".to_string(),
            due_date: "2024-06-20".to_string(),
            total_amount: 1000.0,
            currency: "TWD".to_string(),
            image_path: "test1.jpg".to_string(),
            status: "unconfirmed".to_string(),
            payment_status: "unpaid".to_string(),
            payment_term_id: None,
            paid_at: None,
            error_message: None,
            created_at: String::new(),
            updated_at: String::new(),
        };

        // First insert succeeds
        let id1 = db.insert_receipt(&r1).await.expect("First insert should succeed");

        // Second insert with same `no` must fail
        let mut r2 = r1.clone();
        r2.image_path = "test2.jpg".to_string();
        let err = db.insert_receipt(&r2).await;
        assert!(err.is_err(), "Duplicate receipt no must be rejected");

        // Clean up
        db.delete_receipt(id1).await.ok();
    }
}
