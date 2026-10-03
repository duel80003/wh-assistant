# WHassistant 資料庫結構與遷移說明 (Database Migrations)

本專案使用原生 **SQLite 3** 作為本地嵌入式資料庫，搭配 **WAL (Write-Ahead Logging)** 高效模式與外鍵約束保證資料完整性。

---

## 📍 一、 SQLite 資料庫存放位置 (Database Location)

應用程式遵從各作業系統標準的本機應用程式資料路徑（透過 Rust `dirs::data_local_dir()` 自動定位）：

| 作業系統 (OS) | 實體資料庫完整路徑 | 圖片儲存目錄 |
| :--- | :--- | :--- |
| **macOS** | `~/Library/Application Support/whassistant/whassistant.db` | `~/Library/Application Support/whassistant/receipts/images/` |
| **Windows** | `C:\Users\<使用者名稱>\AppData\Local\whassistant\whassistant.db` | `C:\Users\<使用者名稱>\AppData\Local\whassistant\receipts\images\` |
| **Linux** | `~/.local/share/whassistant/whassistant.db` | `~/.local/share/whassistant/receipts/images/` |

> [!NOTE]
> - 在 WAL 模式下，同目錄下可能會有暫存日誌檔 `whassistant.db-wal` 與共享記憶體檔 `whassistant.db-shm`，此為 SQLite 正常機制。
> - 跨平台備份或移轉時，請將 `whassistant.db` 與整個 `receipts/` 目錄一併複製。

---

## 🗄️ 二、 目錄結構與 DDL 檔案

```text
migrations/
├── 001_initial_schema.sql  # 初始資料庫結構 (DDL、索引與種子資料)
└── README.md              # 本說明文件 (路徑規範、表結構與使用指南)
```

---

## 📊 三、 資料表關聯結構 (Entity Relationship)

```mermaid
erDiagram
    payment_terms ||--o{ receipts : "約束收款期限"
    receipts ||--o| processed_files : "對應原始檔案"

    payment_terms {
        INTEGER id PK "主鍵"
        TEXT name "規則名稱 (例: 月結 30 天)"
        TEXT duration_code "代碼標籤 (例: 30d)"
        INTEGER duration_days "收款天數 (加計基準)"
        INTEGER is_default "是否預設 (0/1)"
        TEXT description "備註說明"
        TEXT created_at "建立時間"
    }

    receipts {
        INTEGER id PK "主鍵"
        INTEGER no UK "工單號碼 (純數字唯一鍵，無NO.前綴)"
        TEXT matainer "施工人員 / 保養者"
        TEXT work_date "施工日期 (逾期基準點)"
        TEXT due_date "到期日 (work_date + duration_days)"
        REAL total_amount "工單總金額"
        TEXT currency "幣別 (預設 TWD)"
        TEXT image_path "圖片相對檔名"
        TEXT status "審核狀態 (unconfirmed/confirmed/failed)"
        TEXT payment_status "付款狀態 (unpaid/paid - unpaid 永不刪除)"
        INTEGER payment_term_id FK "關聯 payment_terms"
        TEXT paid_at "實收日期"
        TEXT error_message "錯誤紀錄"
        TEXT created_at "建立時間"
        TEXT updated_at "更新時間"
    }

    processed_files {
        INTEGER id PK "主鍵"
        TEXT file_path "來源檔案原始路徑"
        TEXT file_hash UK "SHA-256 唯一雜湊碼 (防重複辨識)"
        INTEGER file_size "檔案大小 (bytes)"
        INTEGER receipt_id FK "關聯 receipts"
        TEXT status "處理狀態 (completed/failed/ignored)"
        TEXT error_message "錯誤訊息"
        TEXT processed_at "處理時間"
    }

    app_settings {
        TEXT key PK "設定鍵名 (例: retention_days, ollama_url)"
        TEXT value "設定值"
        TEXT updated_at "最後更新時間"
    }
```

---

## ⚙️ 四、 自動遷移機制 (Auto-Migration)

本應用程式於啟動時在 `src/db/mod.rs` 中的 `Database::init()` 自動執行下列步驟：
1. **建立資料庫與連線池**：若檔案不存在則自動建立，啟用 WAL 模式。
2. **結構遷移 (`migrate`)**：執行 `CREATE TABLE IF NOT EXISTS` 與 `CREATE INDEX IF NOT EXISTS`。
3. **預設資料寫入 (`seed_defaults`)**：若無任何條款或設定，自動寫入即期/30天/60天/90天收款條款及預設系統設定。

---

## 🛠️ 五、 如何手動檢視或連線資料庫

### 1. 使用終端機 sqlite3 CLI
```bash
# macOS 快速進入 SQLite 命令列
sqlite3 "$HOME/Library/Application Support/whassistant/whassistant.db"

# 查看所有資料表
.tables

# 查看資料表欄位定義
.schema receipts

# 離開
.exit
```

### 2. 使用 GUI 資料庫工具
推薦以下免費且支援 SQLite 的視覺化工具：
- **TablePlus** (macOS / Windows)
- **DBeaver** (全平台)
- **DB Browser for SQLite** (全平台開源)

**連線方式**：開啟工具後選擇 SQLite，直接將上述對應平台的 `whassistant.db` 檔案拖入或開啟即可。
