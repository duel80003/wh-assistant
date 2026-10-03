-- ==============================================================================
-- 專案名稱: WHassistant 工單與單據智慧辨識管理系統
-- 檔案名稱: 001_initial_schema.sql
-- 說明: SQLite 資料庫初始化 DDL、索引與預設種子資料
-- ==============================================================================

-- 啟用外鍵約束
PRAGMA foreign_keys = ON;

-- ------------------------------------------------------------------------------
-- 1. 收款期限規則表 (payment_terms)
-- 說明: 管理不同工單施工後的付款期限規則（如：即期、月結30天、雙月結60天等）
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS payment_terms (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    name TEXT NOT NULL,                         -- 規則名稱 (例如: "月結 30 天")
    duration_code TEXT NOT NULL,                -- 代碼標識 (例如: "30d", "1m")
    duration_days INTEGER NOT NULL,             -- 到期天數 (計算 due_date = work_date + duration_days)
    is_default INTEGER NOT NULL DEFAULT 0,      -- 是否為系統預設條款 (0: 否, 1: 是)
    description TEXT,                           -- 說明備註
    created_at TEXT NOT NULL                    -- 建立時間 (ISO 8601)
);

-- ------------------------------------------------------------------------------
-- 2. 工單與收據核心資料表 (receipts)
-- 說明: 存放經由 Ollama 辨識並經人工審核的工單主資料
-- 重要規則:
--   - 到期日計算基準一律為工單中的「施工日期」(work_date)，絕非 created_at
--   - payment_status 為 'unpaid' 的單據受絕對保護，絕不自動刪除
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS receipts (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    no INTEGER,                                 -- 工單號碼 (純數字唯一鍵，無前綴 NO.)
    matainer TEXT,                              -- 施工人員 / 保養技師
    work_date TEXT NOT NULL,                    -- 施工日期 (標準化 YYYY-MM-DD，逾期計算基準點)
    due_date TEXT NOT NULL,                     -- 預計收款到期日 (YYYY-MM-DD)
    total_amount REAL NOT NULL DEFAULT 0.0,     -- 總金額
    currency TEXT NOT NULL DEFAULT 'TWD',       -- 幣別 (預設新台幣 TWD)
    image_path TEXT NOT NULL,                   -- 圖片相對儲存檔名 (儲存於 receipts/images/)
    status TEXT NOT NULL DEFAULT 'unconfirmed', -- 辨識狀態: 'unconfirmed' (待審核), 'confirmed' (已確認), 'failed' (辨識失敗), 'processing' (處理中)
    payment_status TEXT NOT NULL DEFAULT 'unpaid', -- 收款狀態: 'unpaid' (未收), 'paid' (已收)
    payment_term_id INTEGER,                    -- 關聯之收款期限規則 ID
    paid_at TEXT,                               -- 實際收款日期 (YYYY-MM-DD 或 ISO 8601)
    error_message TEXT,                         -- 辨識異常或錯誤訊息
    created_at TEXT NOT NULL,                   -- 系統建檔時間 (ISO 8601)
    updated_at TEXT NOT NULL,                   -- 最後更新時間 (ISO 8601)
    FOREIGN KEY (payment_term_id) REFERENCES payment_terms(id) ON DELETE SET NULL
);

-- ------------------------------------------------------------------------------
-- 3. 檔案防重複紀錄內部表 (processed_files)
-- 說明: 監控資料夾投入或手動上傳時，防止相同圖片重複辨識與建立工單
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS processed_files (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    file_path TEXT NOT NULL,                    -- 來源檔案原始路徑
    file_hash TEXT NOT NULL UNIQUE,             -- 檔案 SHA-256 唯一雜湊碼
    file_size INTEGER NOT NULL,                 -- 檔案大小 (bytes)
    receipt_id INTEGER,                         -- 關聯建立之 receipts.id
    status TEXT NOT NULL,                       -- 處理狀態 ('completed', 'failed', 'ignored')
    error_message TEXT,                         -- 錯誤訊息
    processed_at TEXT NOT NULL,                 -- 處理時間 (ISO 8601)
    FOREIGN KEY (receipt_id) REFERENCES receipts(id) ON DELETE SET NULL
);

-- ------------------------------------------------------------------------------
-- 4. 系統環境設定表 (app_settings)
-- 說明: 存放 Ollama 連線、監控目錄、資料保留天數、自動備份與雲端圖片指向等全域鍵值設定
-- 鍵值說明:
--   - ollama_url: Ollama API 服務端點 (預設 http://localhost:11434)
--   - ollama_model: 視覺辨識模型名稱 (預設 llama3.2-vision)
--   - retention_days: 已收款工單歷史保留天數 (預設 365 天，0 為永久保存)
--   - monitor_dir: 目錄監控資料夾路徑
--   - monitor_enabled: 是否啟用背景目錄監聽 (true/false)
--   - auto_backup_enabled: 是否啟用資料庫定時/自動雲端快照備份 (true/false)
--   - auto_backup_dir: 資料庫快照存放目錄 (Google Drive / OneDrive / 本機路徑)
--   - last_backup_time: 最近一次成功輸出快照的時間戳記
--   - cloud_images_enabled: 是否啟用工單圖片雲端同步目錄指向 (true/false)
--   - cloud_images_dir: 工單圖片存放之雲端同步目錄路徑
-- ------------------------------------------------------------------------------
CREATE TABLE IF NOT EXISTS app_settings (
    key TEXT PRIMARY KEY,                       -- 設定項鍵值
    value TEXT NOT NULL,                        -- 設定值
    updated_at TEXT NOT NULL                    -- 更新時間 (ISO 8601)
);

-- ------------------------------------------------------------------------------
-- 索引配置 (Indexes)
-- 說明: 加速待審核列表、收款狀態、歷史查詢、逾期篩選與雜湊比對效能
-- ------------------------------------------------------------------------------
CREATE UNIQUE INDEX IF NOT EXISTS idx_receipts_no ON receipts(no) WHERE no IS NOT NULL;
CREATE INDEX IF NOT EXISTS idx_receipts_status ON receipts(status);
CREATE INDEX IF NOT EXISTS idx_receipts_payment_status ON receipts(payment_status);
CREATE INDEX IF NOT EXISTS idx_receipts_work_date ON receipts(work_date);
CREATE INDEX IF NOT EXISTS idx_receipts_due_date ON receipts(due_date);
CREATE INDEX IF NOT EXISTS idx_processed_files_hash ON processed_files(file_hash);

-- ------------------------------------------------------------------------------
-- 5. 預設種子資料 (Initial Seed Data)
-- ------------------------------------------------------------------------------

-- 預設收款期限條款
INSERT OR IGNORE INTO payment_terms (id, name, duration_code, duration_days, is_default, description, created_at)
VALUES 
    (1, '即期付款', '0d', 0, 0, '施工完成即刻付款', datetime('now', 'localtime')),
    (2, '月結 30 天', '30d', 30, 1, '施工次日起 30 天內結清 (系統預設)', datetime('now', 'localtime')),
    (3, '雙月結 60 天', '60d', 60, 0, '施工次日起 60 天內結清', datetime('now', 'localtime')),
    (4, '季結 90 天', '90d', 90, 0, '施工次日起 90 天內結清', datetime('now', 'localtime'));

-- 預設系統參數
INSERT OR IGNORE INTO app_settings (key, value, updated_at)
VALUES 
    ('ollama_url', 'http://localhost:11434', datetime('now', 'localtime')),
    ('ollama_model', 'llama3.2-vision', datetime('now', 'localtime')),
    ('retention_days', '365', datetime('now', 'localtime')),
    ('monitor_dir', '', datetime('now', 'localtime')),
    ('monitor_enabled', 'false', datetime('now', 'localtime')),
    ('auto_backup_enabled', 'false', datetime('now', 'localtime')),
    ('auto_backup_dir', '', datetime('now', 'localtime')),
    ('last_backup_time', '', datetime('now', 'localtime')),
    ('cloud_images_enabled', 'false', datetime('now', 'localtime')),
    ('cloud_images_dir', '', datetime('now', 'localtime'));
