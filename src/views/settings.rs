use chrono::Local;
use dioxus::prelude::*;

use crate::db::Database;
use crate::models::PaymentTerm;
use crate::services::backup::BackupService;
use crate::services::ollama::OllamaService;
use crate::services::storage::StorageService;
use crate::services::updater::{UpdateInfo, UpdateStatus, UpdaterService};
use crate::services::watcher::{WatcherEvent, WatcherService};

#[component]
pub fn SettingsView() -> Element {
    let db = use_context::<Database>();
    let mut payment_terms = use_signal(Vec::<PaymentTerm>::new);

    // Update signals
    let mut update_info = use_context::<Signal<UpdateInfo>>();
    let mut update_status = use_context::<Signal<UpdateStatus>>();
    let mut is_checking_update = use_signal(|| false);
    let mut manual_check_notice = use_signal(|| Option::<String>::None);

    // Settings fields
    let mut ollama_url = use_signal(|| "http://localhost:11434".to_string());
    let mut ollama_model = use_signal(|| "llama3.2-vision".to_string());
    let mut monitor_dir = use_signal(String::new);
    let mut monitor_enabled = use_signal(|| false);
    let mut retention_days = use_signal(|| "365".to_string());

    // Backup & Restore fields & signals
    let mut auto_backup_enabled = use_signal(|| false);
    let mut auto_backup_dir = use_signal(String::new);
    let mut last_backup_time = use_signal(|| "尚未執行過備份".to_string());
    let mut backup_notice = use_signal(|| Option::<(String, bool)>::None);
    let mut is_exporting_backup = use_signal(|| false);
    let mut is_restoring_backup = use_signal(|| false);
    let mut show_restore_confirm_modal = use_signal(|| false);
    let mut restore_file_path = use_signal(|| Option::<std::path::PathBuf>::None);
    let mut restore_success = use_signal(|| false);

    // Cloud Images fields & signals
    let mut cloud_images_enabled = use_signal(|| false);
    let mut cloud_images_dir = use_signal(String::new);
    let mut cloud_images_notice = use_signal(|| Option::<(String, bool)>::None);
    let mut is_copying_images = use_signal(|| false);

    // Status & Feedback signals
    let mut test_result = use_signal(|| Option::<(String, bool)>::None); // (text, is_success)
    let mut save_notice = use_signal(|| Option::<String>::None);
    let mut cleanup_notice = use_signal(|| Option::<String>::None);

    // Modal for adding new payment term
    let mut show_add_term_modal = use_signal(|| false);
    let mut new_term_name = use_signal(String::new);
    let mut new_term_code = use_signal(|| "30d".to_string());
    let mut new_term_days = use_signal(|| 30i64);
    let mut new_term_desc = use_signal(String::new);

    // Modal for cleanup confirmation
    let mut show_cleanup_confirm_modal = use_signal(|| false);

    // Reload settings & terms
    let reload = {
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                if let Ok(terms) = db.get_payment_terms().await {
                    payment_terms.set(terms);
                }
                if let Ok(Some(v)) = db.get_setting("ollama_url").await {
                    ollama_url.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("ollama_model").await {
                    ollama_model.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("monitor_dir").await {
                    monitor_dir.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("monitor_enabled").await {
                    monitor_enabled.set(v == "true");
                }
                if let Ok(Some(v)) = db.get_setting("retention_days").await {
                    retention_days.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("auto_backup_enabled").await {
                    auto_backup_enabled.set(v == "true");
                }
                if let Ok(Some(v)) = db.get_setting("auto_backup_dir").await {
                    auto_backup_dir.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("last_backup_time").await {
                    last_backup_time.set(v);
                }
                if let Ok(Some(v)) = db.get_setting("cloud_images_enabled").await {
                    cloud_images_enabled.set(v == "true");
                }
                if let Ok(Some(v)) = db.get_setting("cloud_images_dir").await {
                    cloud_images_dir.set(v);
                }
            });
        }
    };

    use_effect({
        let reload = reload.clone();
        move || {
            reload();
        }
    });

    // Save settings handler
    let handle_save_settings = {
        let db = db.clone();
        move |_| {
            let db = db.clone();
            spawn(async move {
                let _ = db.set_setting("ollama_url", &ollama_url()).await;
                let _ = db.set_setting("ollama_model", &ollama_model()).await;
                let _ = db.set_setting("monitor_dir", &monitor_dir()).await;
                let _ = db
                    .set_setting(
                        "monitor_enabled",
                        if monitor_enabled() { "true" } else { "false" },
                    )
                    .await;
                let _ = db.set_setting("retention_days", &retention_days()).await;
                let _ = db
                    .set_setting(
                        "auto_backup_enabled",
                        if auto_backup_enabled() {
                            "true"
                        } else {
                            "false"
                        },
                    )
                    .await;
                let _ = db.set_setting("auto_backup_dir", &auto_backup_dir()).await;
                let _ = db
                    .set_setting(
                        "cloud_images_enabled",
                        if cloud_images_enabled() {
                            "true"
                        } else {
                            "false"
                        },
                    )
                    .await;
                let _ = db
                    .set_setting("cloud_images_dir", &cloud_images_dir())
                    .await;

                // Sync custom images dir to StorageService
                if cloud_images_enabled() && !cloud_images_dir().trim().is_empty() {
                    StorageService::set_custom_images_dir(Some(cloud_images_dir().trim().into()));
                } else {
                    StorageService::set_custom_images_dir(None);
                }

                WatcherService::emit(WatcherEvent::StatusChanged {
                    enabled: monitor_enabled(),
                    dir: monitor_dir(),
                });

                if auto_backup_enabled() {
                    let _ = BackupService::trigger_auto_cloud_backup(&db).await;
                    if let Ok(Some(v)) = db.get_setting("last_backup_time").await {
                        last_backup_time.set(v);
                    }
                }

                save_notice.set(Some("✓ 系統設定已成功儲存！".to_string()));
            });
        }
    };

    // Pick cloud images directory handler
    let handle_pick_cloud_images_dir = {
        move |_| {
            spawn(async move {
                if let Some(folder) = rfd::AsyncFileDialog::new()
                    .set_title("選擇掃描圖片存放之雲端同步資料夾（例如 Google Drive、OneDrive 或 Dropbox）")
                    .pick_folder()
                    .await
                {
                    cloud_images_dir.set(folder.path().to_string_lossy().to_string());
                }
            });
        }
    };

    // Copy all local images to cloud directory handler
    let handle_copy_images_to_cloud = {
        move |_| {
            let target_str = cloud_images_dir();
            if target_str.trim().is_empty() {
                cloud_images_notice.set(Some(("請先選取雲端圖片目標資料夾！".to_string(), true)));
                return;
            }
            let target_path = std::path::PathBuf::from(target_str.trim());
            is_copying_images.set(true);
            spawn(async move {
                match StorageService::copy_all_images_to_custom_dir(Some(&target_path)) {
                    Ok(count) => {
                        cloud_images_notice.set(Some((
                            format!("✓ 成功將 {} 張本地圖片完整複製同步至雲端目錄！", count),
                            false,
                        )));
                    }
                    Err(e) => {
                        cloud_images_notice.set(Some((
                            format!("複製圖片至雲端目錄失敗: {:#}", e),
                            true,
                        )));
                    }
                }
                is_copying_images.set(false);
            });
        }
    };

    // Pick backup directory handler
    let handle_pick_backup_dir = {
        move |_| {
            spawn(async move {
                if let Some(folder) = rfd::AsyncFileDialog::new()
                    .set_title("選擇自動備份同步資料夾（可選 Google Drive、OneDrive 或本地目錄）")
                    .pick_folder()
                    .await
                {
                    auto_backup_dir.set(folder.path().to_string_lossy().to_string());
                }
            });
        }
    };

    // Manual cloud snapshot handler
    let handle_manual_snapshot = {
        let db = db.clone();
        move |_| {
            let db = db.clone();
            spawn(async move {
                let target_dir = auto_backup_dir();
                if target_dir.trim().is_empty() {
                    backup_notice.set(Some(("請先設定備份目標資料夾！".to_string(), true)));
                    return;
                }
                let target =
                    std::path::PathBuf::from(target_dir.trim()).join("whassistant_snapshot.db");
                match BackupService::backup_db_snapshot(&db, &target).await {
                    Ok(()) => {
                        let now_str = chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string();
                        let _ = db.set_setting("last_backup_time", &now_str).await;
                        last_backup_time.set(now_str);
                        backup_notice.set(Some((
                            "✓ 資料庫已成功安全備份至指定資料夾！".to_string(),
                            false,
                        )));
                    }
                    Err(e) => {
                        backup_notice.set(Some((format!("備份失敗: {:#}", e), true)));
                    }
                }
            });
        }
    };

    // Export full ZIP backup handler
    let handle_export_zip = {
        let db = db.clone();
        move |_| {
            let db = db.clone();
            spawn(async move {
                let default_name = format!(
                    "whassistant_backup_{}.zip",
                    chrono::Local::now().format("%Y%m%d_%H%M%S")
                );
                if let Some(handle) = rfd::AsyncFileDialog::new()
                    .set_title("匯出完整系統備份檔案 (.zip)")
                    .set_file_name(&default_name)
                    .add_filter("ZIP 備份壓縮檔", &["zip"])
                    .save_file()
                    .await
                {
                    is_exporting_backup.set(true);
                    let path = handle.path().to_path_buf();
                    match BackupService::export_full_backup_zip(&db, &path).await {
                        Ok((size, img_count)) => {
                            let size_mb = (size as f64) / 1024.0 / 1024.0;
                            backup_notice.set(Some((
                                format!(
                                    "✓ 完整備份匯出成功！大小：{:.2} MB，共打包 {} 張單據圖檔。",
                                    size_mb, img_count
                                ),
                                false,
                            )));
                        }
                        Err(e) => {
                            backup_notice.set(Some((format!("匯出備份失敗: {:#}", e), true)));
                        }
                    }
                    is_exporting_backup.set(false);
                }
            });
        }
    };

    // Pick restore ZIP file handler
    let handle_pick_restore_file = {
        move |_| {
            spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new()
                    .set_title("選取要還原的 WHassistant 備份檔（.zip 備份包 或 .db 快照檔）")
                    .add_filter("WHassistant 備份檔案 (*.zip, *.db)", &["zip", "db"])
                    .pick_file()
                    .await
                {
                    restore_file_path.set(Some(handle.path().to_path_buf()));
                    restore_success.set(false);
                    show_restore_confirm_modal.set(true);
                }
            });
        }
    };

    // Execute restore handler
    let handle_execute_restore = {
        move |_| {
            if let Some(path) = restore_file_path() {
                is_restoring_backup.set(true);
                spawn(async move {
                    match BackupService::restore_any_backup(&path) {
                        Ok((img_count, is_zip)) => {
                            restore_success.set(true);
                            if is_zip {
                                backup_notice.set(Some((
                                    format!(
                                        "✓ 系統資料庫與 {} 張圖檔已成功還原！請點選重啟軟體。",
                                        img_count
                                    ),
                                    false,
                                )));
                            } else {
                                backup_notice.set(Some((
                                    "✓ 資料庫快照已成功還原！請點選重啟軟體。".to_string(),
                                    false,
                                )));
                            }
                        }
                        Err(e) => {
                            backup_notice.set(Some((format!("還原備份失敗: {:#}", e), true)));
                            show_restore_confirm_modal.set(false);
                        }
                    }
                    is_restoring_backup.set(false);
                });
            }
        }
    };

    // Pick directory handler
    let handle_pick_directory = {
        move |_| {
            spawn(async move {
                if let Some(folder) = rfd::AsyncFileDialog::new().pick_folder().await {
                    monitor_dir.set(folder.path().to_string_lossy().to_string());
                }
            });
        }
    };

    // Test Ollama connection handler
    let handle_test_ollama = {
        move |_| {
            let url = ollama_url();
            spawn(async move {
                test_result.set(None);
                match OllamaService::test_connection(&url).await {
                    Ok(models) => {
                        let count = models.len();
                        test_result.set(Some((
                            format!(
                                "✓ 連線成功！Ollama 正常運行中，本機已安裝 {} 個模型 ({})",
                                count,
                                models.join(", ")
                            ),
                            true,
                        )));
                    }
                    Err(e) => {
                        test_result.set(Some((format!("✕ 連線失敗: {:#}", e), false)));
                    }
                }
            });
        }
    };

    // Add new payment term handler
    let handle_create_term = {
        let db = db.clone();
        let reload = reload.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            let term = PaymentTerm {
                id: 0,
                name: new_term_name().trim().to_string(),
                duration_code: new_term_code().trim().to_string(),
                duration_days: new_term_days(),
                is_default: false,
                description: Some(new_term_desc().trim().to_string()).filter(|s| !s.is_empty()),
                created_at: Local::now().to_rfc3339(),
            };

            spawn(async move {
                if !term.name.is_empty() {
                    let _ = db.insert_payment_term(&term).await;
                    show_add_term_modal.set(false);
                    new_term_name.set(String::new());
                    reload();
                }
            });
        }
    };

    // Execute retention cleanup handler
    let handle_execute_cleanup = {
        let db = db.clone();
        move |_| {
            let db = db.clone();
            spawn(async move {
                let days: i64 = retention_days().parse().unwrap_or(365);
                match db.cleanup_expired_paid_receipts(days).await {
                    Ok(count) => {
                        cleanup_notice.set(Some(format!(
                            "✓ 清理完成！共刪除 {} 筆已收費過期單據與關聯圖片檔案。",
                            count
                        )));
                    }
                    Err(e) => {
                        cleanup_notice.set(Some(format!("✕ 清理失敗: {:#}", e)));
                    }
                }
                show_cleanup_confirm_modal.set(false);
            });
        }
    };

    rsx! {
        div { class: "flex flex-col gap-6 max-w-4xl pb-10",
            // Page Header
            div { class: "flex items-center justify-between",
                div {
                    h1 { class: "text-2xl font-bold text-slate-100 tracking-tight", "系統設定" }
                    p { class: "text-xs text-slate-400 mt-1", "自訂收費期限規則、目錄自動監聽、Ollama 視覺模型與資料生命週期清理政策" }
                }
                button {
                    class: "px-5 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                    onclick: handle_save_settings,
                    "儲存全部設定"
                }
            }

            // Save Notification Banner
            if let Some(msg) = save_notice() {
                div { class: "px-4 py-3 rounded-xl bg-emerald-950/40 border border-emerald-800 text-emerald-300 text-xs flex items-center justify-between",
                    span { "{msg}" }
                    button {
                        class: "text-slate-400 hover:text-white text-sm font-bold ml-4 cursor-pointer",
                        onclick: move |_| save_notice.set(None),
                        "✕"
                    }
                }
            }

            // Section 1: 收費期限規則管理 (Payment Terms)
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                    div {
                        h2 { class: "text-sm font-semibold text-slate-100", "收費期限規則管理 (Payment Terms)" }
                        p { class: "text-xs text-slate-400 mt-0.5", "設定公司約定的收費期限週期方案（如 30d、1m 等），單據核對時自動推算應收截止日" }
                    }
                    button {
                        class: "px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition-colors cursor-pointer",
                        onclick: move |_| show_add_term_modal.set(true),
                        "+ 新增收費方案"
                    }
                }

                table { class: "w-full text-left text-xs border-collapse",
                    thead {
                        tr { class: "text-slate-400 border-b border-slate-800",
                            th { class: "py-2 px-3", "方案名稱" }
                            th { class: "py-2 px-3", "週期代碼" }
                            th { class: "py-2 px-3", "折算天數" }
                            th { class: "py-2 px-3 text-center", "預設方案" }
                            th { class: "py-2 px-3", "說明" }
                            th { class: "py-2 px-3 text-center", "操作" }
                        }
                    }
                    tbody { class: "divide-y divide-slate-800/60 text-slate-300",
                        for term in payment_terms() {
                            {
                                let tid = term.id;
                                let is_def = term.is_default;
                                rsx! {
                                    tr { key: "{term.id}", class: "hover:bg-slate-800/30",
                                        td { class: "py-2.5 px-3 font-medium text-slate-100", "{term.name}" }
                                        td { class: "py-2.5 px-3 font-mono text-indigo-400", "{term.duration_code}" }
                                        td { class: "py-2.5 px-3 font-mono font-semibold", "{term.duration_days} 天" }
                                        td { class: "py-2.5 px-3 text-center",
                                            if is_def {
                                                span { class: "px-2 py-0.5 rounded-full text-[11px] font-bold bg-indigo-500/20 text-indigo-300 border border-indigo-500/30",
                                                    "★ 系統預設"
                                                }
                                            } else {
                                                span { class: "text-slate-600", "-" }
                                            }
                                        }
                                        td { class: "py-2.5 px-3 text-slate-400", "{term.description.as_deref().unwrap_or(\"-\")}" }
                                        td { class: "py-2.5 px-3 text-center",
                                            div { class: "flex items-center justify-center gap-2",
                                                if !is_def {
                                                    button {
                                                        class: "px-2 py-0.5 text-xs text-indigo-400 hover:text-indigo-300 hover:bg-slate-800 rounded transition-colors cursor-pointer",
                                                        onclick: {
                                                            let db = db.clone();
                                                            let reload = reload.clone();
                                                            move |_| {
                                                                let db = db.clone();
                                                                let reload = reload.clone();
                                                                spawn(async move {
                                                                    let _ = db.set_default_payment_term(tid).await;
                                                                    reload();
                                                                });
                                                            }
                                                        },
                                                        "設為預設"
                                                    }
                                                    button {
                                                        class: "px-2 py-0.5 text-xs text-rose-400 hover:text-rose-300 hover:bg-slate-800 rounded transition-colors cursor-pointer",
                                                        onclick: {
                                                            let db = db.clone();
                                                            let reload = reload.clone();
                                                            move |_| {
                                                                let db = db.clone();
                                                                let reload = reload.clone();
                                                                spawn(async move {
                                                                    let _ = db.delete_payment_term(tid).await;
                                                                    reload();
                                                                });
                                                            }
                                                        },
                                                        "刪除"
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Section 2: 目錄自動監聽設定
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "border-b border-slate-800 pb-3",
                    h2 { class: "text-sm font-semibold text-slate-100", "目錄自動監聽設定 (Directory Watcher)" }
                    p { class: "text-xs text-slate-400 mt-0.5", "指定電腦資料夾，只要新增圖片檔系統即自動在背景排隊解析，無損原檔且具備自動排重機制" }
                }

                div { class: "flex flex-col gap-4",
                    // Enable/Disable Toggle
                    div { class: "flex items-center justify-between bg-slate-950 p-3 rounded-lg border border-slate-800",
                        div { class: "flex flex-col",
                            span { class: "text-sm font-medium text-slate-200", "自動監聽開關" }
                            span { class: "text-xs text-slate-400", "啟用後系統將在背景定期檢查並監控新加入的圖片" }
                        }
                        button {
                            class: if monitor_enabled() {
                                "px-4 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer"
                            } else {
                                "px-4 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 text-xs font-medium rounded-lg transition-colors cursor-pointer"
                            },
                            onclick: move |_| monitor_enabled.set(!monitor_enabled()),
                            if monitor_enabled() { "● 監聽運作中 (點擊停用)" } else { "○ 已停用 (點擊啟用)" }
                        }
                    }

                    // Directory Path Selector
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-medium text-slate-300", "監聽資料夾路徑" }
                        div { class: "flex items-center gap-2",
                            input {
                                r#type: "text",
                                placeholder: "尚未指定資料夾路徑...",
                                value: "{monitor_dir}",
                                oninput: move |e| monitor_dir.set(e.value()),
                                class: "flex-1 bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-100 placeholder-slate-600 outline-none font-mono"
                            }
                            button {
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition-colors cursor-pointer shrink-0",
                                onclick: handle_pick_directory,
                                "📂 選擇資料夾"
                            }
                        }
                    }
                }
            }

            // Section 3: Ollama 視覺模型連接設定
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "border-b border-slate-800 pb-3",
                    h2 { class: "text-sm font-semibold text-slate-100", "Ollama 視覺模型設定 (Local Vision AI)" }
                    p { class: "text-xs text-slate-400 mt-0.5", "設定本地端 Ollama 伺服器介面與視覺多模態模型，提供高效本地資料提取" }
                }

                div { class: "grid grid-cols-2 gap-4",
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-medium text-slate-300", "Ollama 伺服器網址" }
                        input {
                            r#type: "text",
                            value: "{ollama_url}",
                            oninput: move |e| ollama_url.set(e.value()),
                            class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                        }
                    }
                    div { class: "flex flex-col gap-1.5",
                        label { class: "text-xs font-medium text-slate-300", "視覺模型名稱" }
                        input {
                            r#type: "text",
                            value: "{ollama_model}",
                            oninput: move |e| ollama_model.set(e.value()),
                            class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                        }
                    }
                }

                // Test Connection Area
                div { class: "flex items-center justify-between pt-1",
                    button {
                        class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition-colors cursor-pointer",
                        onclick: handle_test_ollama,
                        "🔌 測試 Ollama 連線"
                    }
                    if let Some((msg, is_ok)) = test_result() {
                        span {
                            class: if is_ok { "text-xs text-emerald-400 font-medium" } else { "text-xs text-rose-400 font-medium" },
                            "{msg}"
                        }
                    }
                }
            }

            // Section 4: 單據圖檔儲存與雲端目錄指向 (Scanned Images Storage & Cloud Sync)
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "border-b border-slate-800 pb-3",
                    h2 { class: "text-sm font-semibold text-slate-100 flex items-center gap-2",
                        span { "☁️" }
                        "單據圖檔儲存與雲端目錄指向 (Scanned Images Cloud Storage)"
                    }
                    p { class: "text-xs text-slate-400 mt-0.5",
                        "可獨立開關此功能，將掃描識別後的單據圖檔直接存放至 Google 雲端硬碟、OneDrive 或自訂目錄中。系統內建雙軌檢索，舊有圖片絕不遺失破圖。"
                    }
                }

                if let Some((msg, is_err)) = cloud_images_notice() {
                    div {
                        class: if is_err {
                            "px-4 py-3 rounded-lg bg-rose-950/40 border border-rose-800 text-rose-300 text-xs flex items-center justify-between"
                        } else {
                            "px-4 py-3 rounded-lg bg-emerald-950/40 border border-emerald-800 text-emerald-300 text-xs flex items-center justify-between"
                        },
                        span { "{msg}" }
                        button {
                            class: "text-slate-400 hover:text-white text-xs cursor-pointer",
                            onclick: move |_| cloud_images_notice.set(None),
                            "✕"
                        }
                    }
                }

                div { class: "flex flex-col gap-3 p-4 bg-slate-950 rounded-xl border border-slate-800",
                    // Switch row
                    div { class: "flex items-center justify-between",
                        div { class: "flex flex-col gap-0.5",
                            span { class: "text-xs font-semibold text-slate-200", "雲端/自訂圖檔儲存指向開關" }
                            span { class: "text-[11px] text-slate-400",
                                "啟用後，新掃描入庫之單據圖片將直接存放於下方指定之雲端目錄；停用時則存於系統預設本機路徑。"
                            }
                        }

                        button {
                            class: if cloud_images_enabled() {
                                "px-4 py-1.5 bg-emerald-600/20 text-emerald-400 border border-emerald-500/30 text-xs font-semibold rounded-lg hover:bg-emerald-600/30 transition-colors cursor-pointer"
                            } else {
                                "px-4 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-400 text-xs font-medium rounded-lg transition-colors cursor-pointer"
                            },
                            onclick: move |_| cloud_images_enabled.set(!cloud_images_enabled()),
                            if cloud_images_enabled() { "● 雲端指向運作中 (點擊停用)" } else { "○ 預設本機儲存 (點擊啟用雲端指向)" }
                        }
                    }

                    // Directory Path Selector
                    div { class: "flex flex-col gap-1.5 pt-2 border-t border-slate-800/60",
                        label { class: "text-xs font-medium text-slate-300", "雲端/自訂同步資料夾路徑 (例如 Google Drive / OneDrive 資料夾)" }
                        div { class: "flex items-center gap-2",
                            input {
                                r#type: "text",
                                placeholder: "尚未設定雲端資料夾，例如：/Users/.../Google 雲端硬碟/單據圖片...",
                                value: "{cloud_images_dir}",
                                oninput: move |e| cloud_images_dir.set(e.value()),
                                class: "flex-1 bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-100 placeholder-slate-600 outline-none font-mono"
                            }
                            button {
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 transition-colors cursor-pointer shrink-0",
                                onclick: handle_pick_cloud_images_dir,
                                "📁 選擇雲端目錄"
                            }
                        }
                    }

                    // Migration tool & default directory info
                    div { class: "flex items-center justify-between pt-2 border-t border-slate-800/60 flex-wrap gap-2",
                        button {
                            disabled: is_copying_images() || cloud_images_dir().trim().is_empty(),
                            class: "px-3.5 py-1.5 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white text-xs font-medium rounded-lg shadow-sm transition-colors cursor-pointer flex items-center gap-1.5",
                            onclick: handle_copy_images_to_cloud,
                            if is_copying_images() {
                                span { "⏳ 正在複製同步本機圖片中..." }
                            } else {
                                span { "📥 將本機現有圖檔全數複製同步至雲端目錄" }
                            }
                        }

                        div { class: "flex items-center gap-1.5 text-[11px] text-slate-400 font-mono",
                            span { "本機原生目錄：" }
                            span { class: "text-indigo-300 bg-slate-900 px-2 py-0.5 rounded border border-slate-800 truncate max-w-xs",
                                "{StorageService::get_images_dir().to_string_lossy()}"
                            }
                        }
                    }

                    // Smart Fallback Explanation
                    div { class: "p-3 bg-slate-900/60 border border-slate-800/60 rounded-lg flex items-start gap-2 text-[11px] text-slate-400",
                        span { "💡" }
                        span {
                            "智慧雙向搜尋保障：檢視工單照片時，系統會自動在雲端資料夾與本機資料夾雙向查找。無論何時切換資料夾或開關此功能，過去儲存的照片皆能正常顯示，絕不遺失或破圖。"
                        }
                    }
                }
            }

            // Section 5: 資料保留週期與清理 (Lifecycle & Retention)
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "border-b border-slate-800 pb-3",
                    h2 { class: "text-sm font-semibold text-slate-100", "歷史資料保留週期與清理 (Data Lifecycle)" }
                    p { class: "text-xs text-slate-400 mt-0.5",
                        "設定已收費工單的保存期限。※ 核心防護機制：未收費（unpaid）單據受到絕對安全保護，無論時間多久均絕對禁止刪除。"
                    }
                }

                if let Some(msg) = cleanup_notice() {
                    div { class: "px-4 py-3 rounded-lg bg-indigo-950/40 border border-indigo-800 text-indigo-300 text-xs",
                        "{msg}"
                    }
                }

                div { class: "flex items-center justify-between bg-slate-950 p-4 rounded-xl border border-slate-800",
                    div { class: "flex flex-col gap-1",
                        label { class: "text-xs font-semibold text-slate-200", "已收費歷史單據保留天數" }
                        select {
                            class: "bg-slate-900 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-slate-100 outline-none cursor-pointer mt-1",
                            value: "{retention_days}",
                            onchange: move |e| retention_days.set(e.value()),
                            option { value: "30", "30 天" }
                            option { value: "90", "90 天" }
                            option { value: "180", "180 天" }
                            option { value: "365", "365 天 (預設一年)" }
                            option { value: "0", "永久保存 (不清理)" }
                        }
                    }

                    // Trigger Cleanup Button
                    button {
                        class: "px-4 py-2 bg-rose-600/10 hover:bg-rose-600 text-rose-400 hover:text-white border border-rose-500/30 text-xs font-semibold rounded-lg transition-colors cursor-pointer",
                        onclick: move |_| show_cleanup_confirm_modal.set(true),
                        "🧹 立即清理已收費過期單據"
                    }
                }
            }

            // Section 6: 資料庫備份與資料復原 (Data Backup & Recovery)
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-5",
                div { class: "border-b border-slate-800 pb-3 flex items-center justify-between",
                    div {
                        h2 { class: "text-sm font-semibold text-slate-100 flex items-center gap-2",
                            span { "🛡️" }
                            "資料庫備份與資料復原 (Data Backup & Recovery)"
                        }
                        p { class: "text-xs text-slate-400 mt-0.5",
                            "提供完善的資料安全防護機制。支援自動將最新資料同步備份至雲端硬碟（Google 雲端硬碟 / OneDrive），亦可一鍵打包所有工單與照片匯出或還原。"
                        }
                    }
                }

                // Status Notice
                if let Some((msg, is_err)) = backup_notice() {
                    div {
                        class: if is_err {
                            "px-4 py-3 rounded-lg bg-rose-950/40 border border-rose-800 text-rose-300 text-xs flex items-center justify-between"
                        } else {
                            "px-4 py-3 rounded-lg bg-emerald-950/40 border border-emerald-800 text-emerald-300 text-xs flex items-center justify-between"
                        },
                        span { "{msg}" }
                        button {
                            class: "text-slate-400 hover:text-white text-xs cursor-pointer",
                            onclick: move |_| backup_notice.set(None),
                            "✕"
                        }
                    }
                }

                // Sub-section 1: 雲端同步硬碟自動備份 (Google Drive / OneDrive)
                div { class: "flex flex-col gap-3 p-4 bg-slate-950 rounded-xl border border-slate-800/80",
                    div { class: "flex items-center justify-between",
                        div { class: "flex flex-col gap-0.5",
                            span { class: "text-xs font-semibold text-slate-200 flex items-center gap-1.5",
                                span { "☁️" }
                                "雲端硬碟自動同步備份 (Cloud Auto Backup)"
                            }
                            span { class: "text-[11px] text-slate-400",
                                "可指定電腦中的 Google 雲端硬碟、OneDrive 或 Dropbox 資料夾。系統會定時將最新資料安全備份至該處，並由雲端軟體自動上傳保存。"
                            }
                        }
                        label { class: "relative inline-flex items-center cursor-pointer",
                            input {
                                r#type: "checkbox",
                                checked: auto_backup_enabled(),
                                onchange: move |e| auto_backup_enabled.set(e.value() == "true"),
                                class: "sr-only peer"
                            }
                            div { class: "w-11 h-6 bg-slate-800 peer-focus:outline-none rounded-full peer peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-slate-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all peer-checked:bg-indigo-600" }
                        }
                    }

                    div { class: "flex items-center gap-3 pt-2",
                        input {
                            r#type: "text",
                            readonly: true,
                            placeholder: "請選擇目標同步目錄（例如 ~/Google Drive/WHassistant/）",
                            value: "{auto_backup_dir}",
                            class: "flex-1 bg-slate-900 border border-slate-700 rounded-lg px-3 py-2 text-xs text-slate-200 outline-none font-mono"
                        }
                        button {
                            class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition-colors cursor-pointer shrink-0",
                            onclick: handle_pick_backup_dir,
                            "📁 選擇雲端目錄"
                        }
                        button {
                            disabled: auto_backup_dir().trim().is_empty(),
                            class: "px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-40 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer shrink-0",
                            onclick: handle_manual_snapshot,
                            "⚡ 立即手動備份"
                        }
                    }

                    div { class: "flex items-center justify-between text-[11px] text-slate-500 pt-1 font-mono",
                        span { "備份檔案：whassistant_snapshot.db（自動保持最新狀態）" }
                        span { "最近一次備份時間：{last_backup_time}" }
                    }
                }

                // Sub-section 2: 完整備份包匯出與還原 (ZIP 打包)
                div { class: "flex flex-col gap-3 p-4 bg-slate-950 rounded-xl border border-slate-800/80",
                    div { class: "flex flex-col gap-0.5",
                        span { class: "text-xs font-semibold text-slate-200 flex items-center gap-1.5",
                            span { "📦" }
                            "完整備份封裝與系統還原 (Full Backup & Restore)"
                        }
                        span { class: "text-[11px] text-slate-400",
                            "將所有工單資料與留存的照片原圖完整打包成一個 ZIP 壓縮檔，適合用來搬移到新電腦或存入隨身碟永久保存。"
                        }
                    }

                    div { class: "flex items-center justify-between pt-2 border-t border-slate-800/60",
                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-slate-400", "備份內容涵蓋：" }
                            span { class: "px-2 py-0.5 bg-slate-800 text-slate-300 rounded text-[11px] font-mono", "工單資料" }
                            span { class: "px-2 py-0.5 bg-slate-800 text-slate-300 rounded text-[11px] font-mono", "全部照片原圖" }
                            span { class: "px-2 py-0.5 bg-slate-800 text-slate-300 rounded text-[11px] font-mono", "設定參數" }
                        }

                        div { class: "flex items-center gap-3",
                            button {
                                disabled: is_exporting_backup(),
                                class: "px-4 py-2 bg-indigo-600 hover:bg-indigo-500 disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer flex items-center gap-1.5",
                                onclick: handle_export_zip,
                                if is_exporting_backup() {
                                    span { "正在打包匯出中..." }
                                } else {
                                    span { "💾 匯出完整備份包 (.zip)" }
                                }
                            }

                            button {
                                disabled: is_restoring_backup(),
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-amber-400 hover:text-amber-300 border border-slate-700 rounded-lg text-xs font-semibold transition-colors cursor-pointer flex items-center gap-1.5",
                                onclick: handle_pick_restore_file,
                                span { "♻️ 從備份檔還原系統..." }
                            }
                        }
                    }
                }
            }

            // Section 7: 軟體版本與熱更新 (Software Version & Hot Update)
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-4",
                div { class: "border-b border-slate-800 pb-3",
                    h2 { class: "text-sm font-semibold text-slate-100", "軟體版本與線上熱更新 (Software Updates)" }
                    p { class: "text-xs text-slate-400 mt-0.5",
                        "串接 GitHub Releases 官方發布庫 (duel80003/wh-assistant)，支援背景探測與一鍵就地熱替換。"
                    }
                }

                if let Some(msg) = manual_check_notice() {
                    div { class: "px-4 py-3 rounded-lg bg-indigo-950/40 border border-indigo-800 text-indigo-300 text-xs flex items-center justify-between",
                        span { "{msg}" }
                        button {
                            class: "text-slate-400 hover:text-white text-xs cursor-pointer",
                            onclick: move |_| manual_check_notice.set(None),
                            "✕"
                        }
                    }
                }

                div { class: "flex items-center justify-between bg-slate-950 p-4 rounded-xl border border-slate-800",
                    div { class: "flex flex-col gap-1",
                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-slate-400", "目前版本：" }
                            span { class: "text-xs font-mono font-bold text-slate-200", "v{env!(\"CARGO_PKG_VERSION\")}" }
                        }
                        div { class: "flex items-center gap-2",
                            span { class: "text-xs text-slate-400", "最新發布：" }
                            if update_info().has_update {
                                span { class: "text-xs font-mono font-bold text-emerald-400", "{update_info().latest_version} (有新版本！)" }
                            } else {
                                span { class: "text-xs font-mono text-slate-400", "目前已是最新版本" }
                            }
                        }
                    }

                    div { class: "flex items-center gap-3",
                        button {
                            disabled: is_checking_update(),
                            class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-semibold rounded-lg border border-slate-700 disabled:opacity-50 transition-colors cursor-pointer",
                            onclick: move |_| {
                                is_checking_update.set(true);
                                spawn(async move {
                                    match UpdaterService::check_for_updates().await {
                                        Ok(Some(info)) => {
                                            update_info.set(info);
                                            manual_check_notice.set(Some("檢測完成：發現新版本！".to_string()));
                                        }
                                        Ok(None) => {
                                            manual_check_notice.set(Some("檢測完成：目前已是最新版本，無需更新。".to_string()));
                                        }
                                        Err(e) => {
                                            manual_check_notice.set(Some(format!("檢查更新失敗: {:#}", e)));
                                        }
                                    }
                                    is_checking_update.set(false);
                                });
                            },
                            if is_checking_update() { "檢查中..." } else { "🔍 檢查更新" }
                        }

                        if update_info().has_update {
                            button {
                                class: "px-4 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer animate-pulse",
                                onclick: {
                                    let dl_url = update_info().download_url.clone();
                                    move |_| {
                                        if let Some(ref url) = dl_url {
                                            let url = url.clone();
                                            update_status.set(UpdateStatus::Downloading { progress: 0 });
                                            spawn(async move {
                                                update_status.set(UpdateStatus::Extracting);
                                                match UpdaterService::download_and_install_update(&url).await {
                                                    Ok(()) => {
                                                        update_status.set(UpdateStatus::ReadyToRestart);
                                                        manual_check_notice.set(Some("更新成功安裝！請重啟應用程式以生效。".to_string()));
                                                    }
                                                    Err(e) => {
                                                        update_status.set(UpdateStatus::Failed(format!("{:#}", e)));
                                                        manual_check_notice.set(Some(format!("更新失敗: {:#}", e)));
                                                    }
                                                }
                                            });
                                        }
                                    }
                                },
                                "🚀 立即熱更新 (Self-Update)"
                            }
                        }
                    }
                }
            }

            // Modal: Add Payment Term
            if show_add_term_modal() {
                div { class: "fixed inset-0 bg-black/75 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                    div { class: "bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 shadow-2xl flex flex-col gap-4",
                        div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                            h3 { class: "text-base font-bold text-slate-100", "新增收費期限方案" }
                            button {
                                class: "text-slate-400 hover:text-white text-lg font-bold cursor-pointer",
                                onclick: move |_| show_add_term_modal.set(false),
                                "✕"
                            }
                        }

                        div { class: "flex flex-col gap-3",
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "方案名稱" }
                                input {
                                    r#type: "text",
                                    placeholder: "例如：月結 45 天",
                                    value: "{new_term_name}",
                                    oninput: move |e| new_term_name.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none"
                                }
                            }
                            div { class: "grid grid-cols-2 gap-3",
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "週期代碼" }
                                    input {
                                        r#type: "text",
                                        placeholder: "例如：45d",
                                        value: "{new_term_code}",
                                        oninput: move |e| new_term_code.set(e.value()),
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                    }
                                }
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "折合天數" }
                                    input {
                                        r#type: "number",
                                        value: "{new_term_days}",
                                        oninput: move |e| {
                                            if let Ok(d) = e.value().parse::<i64>() {
                                                new_term_days.set(d);
                                            }
                                        },
                                        class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                    }
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "方案備註說明 (選填)" }
                                input {
                                    r#type: "text",
                                    placeholder: "選填說明...",
                                    value: "{new_term_desc}",
                                    oninput: move |e| new_term_desc.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none"
                                }
                            }
                        }

                        div { class: "flex items-center justify-end gap-3 pt-3 border-t border-slate-800",
                            button {
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs rounded-lg transition-colors cursor-pointer",
                                onclick: move |_| show_add_term_modal.set(false),
                                "取消"
                            }
                            button {
                                class: "px-5 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                onclick: handle_create_term,
                                "確認新增"
                            }
                        }
                    }
                }
            }

            // Modal: Confirm Retention Cleanup
            if show_cleanup_confirm_modal() {
                div { class: "fixed inset-0 bg-black/75 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                    div { class: "bg-slate-900 border border-slate-800 rounded-2xl max-w-md w-full p-6 shadow-2xl flex flex-col gap-4",
                        h3 { class: "text-base font-bold text-slate-100", "確認執行過期單據清理？" }
                        p { class: "text-xs text-slate-300 leading-relaxed",
                            "系統將依據保留天數（{retention_days} 天），清理「已完成收款（paid）」且施工日期超過保留期限的歷史單據與圖片檔案。"
                        }
                        div { class: "bg-amber-950/30 border border-amber-800/50 p-3 rounded-lg text-amber-300 text-xs leading-normal",
                            "🛡️ 安全保證：未收費（unpaid）單據受到絕對保護，即使時間再久也絕對不會被刪除。"
                        }
                        div { class: "flex items-center justify-end gap-3 pt-3 border-t border-slate-800",
                            button {
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs rounded-lg transition-colors cursor-pointer",
                                onclick: move |_| show_cleanup_confirm_modal.set(false),
                                "取消"
                            }
                            button {
                                class: "px-4 py-2 bg-rose-600 hover:bg-rose-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                onclick: handle_execute_cleanup,
                                "確認執行清理"
                            }
                        }
                    }
                }
            }

            // Modal: Confirm Restore from Backup
            if show_restore_confirm_modal() {
                div { class: "fixed inset-0 bg-black/80 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                    div { class: "bg-slate-900 border border-amber-500/40 rounded-2xl max-w-md w-full p-6 shadow-2xl flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150",
                        div { class: "flex items-center gap-3 text-amber-400 font-bold text-base border-b border-slate-800 pb-3",
                            span { class: "text-2xl", "⚠️" }
                            span { "重要提醒：準備進行系統資料還原" }
                        }

                        if restore_success() {
                            div { class: "flex flex-col gap-3 py-2",
                                div { class: "p-3 bg-emerald-950/40 border border-emerald-500/30 rounded-lg text-emerald-300 text-xs flex items-center gap-2",
                                    span { "✅" }
                                    span { "資料庫與單據照片已成功還原！" }
                                }
                                p { class: "text-xs text-slate-300 leading-relaxed",
                                    "為確保新載入之資料庫連接正常生效，請立即重啟應用程式。"
                                }
                                div { class: "flex justify-end pt-3 border-t border-slate-800",
                                    button {
                                        class: "px-5 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                        onclick: move |_| {
                                            let _ = UpdaterService::restart_app();
                                        },
                                        "立即重啟應用程式"
                                    }
                                }
                            }
                        } else {
                            div { class: "flex flex-col gap-3 text-xs text-slate-300 leading-relaxed",
                                p { class: "text-rose-400 font-semibold",
                                    "【注意】從備份還原將會使用選取的備份檔案覆蓋目前軟體中的所有工單資料與圖片！"
                                }
                                if let Some(path) = restore_file_path() {
                                    div { class: "p-2.5 bg-slate-950 rounded border border-slate-800 font-mono text-[11px] text-slate-400 break-all",
                                        "選取檔案：{path.display()}"
                                    }
                                }
                                p {
                                    "還原完成後，應用程式將提示您重新啟動以完整讀取新資料庫。請問是否確認繼續？"
                                }
                            }

                            div { class: "flex justify-end gap-3 pt-3 border-t border-slate-800",
                                button {
                                    disabled: is_restoring_backup(),
                                    class: "px-4 py-2 text-xs font-medium text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-lg transition-colors cursor-pointer",
                                    onclick: move |_| {
                                        show_restore_confirm_modal.set(false);
                                        restore_file_path.set(None);
                                    },
                                    "取消"
                                }
                                button {
                                    disabled: is_restoring_backup(),
                                    class: "px-5 py-2 bg-rose-600 hover:bg-rose-500 disabled:opacity-50 text-white text-xs font-semibold rounded-lg shadow transition-colors cursor-pointer",
                                    onclick: handle_execute_restore,
                                    if is_restoring_backup() {
                                        "正在解壓還原中..."
                                    } else {
                                        "確認覆蓋並還原"
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
