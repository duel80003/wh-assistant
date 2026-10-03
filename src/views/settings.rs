use dioxus::prelude::*;
use chrono::Local;

use crate::db::Database;
use crate::models::PaymentTerm;
use crate::services::ollama::OllamaService;
use crate::services::storage::StorageService;

#[component]
pub fn SettingsView() -> Element {
    let db = use_context::<Database>();
    let mut payment_terms = use_signal(Vec::<PaymentTerm>::new);

    // Settings fields
    let mut ollama_url = use_signal(|| "http://localhost:11434".to_string());
    let mut ollama_model = use_signal(|| "llama3.2-vision".to_string());
    let mut monitor_dir = use_signal(String::new);
    let mut monitor_enabled = use_signal(|| false);
    let mut retention_days = use_signal(|| "365".to_string());

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
                let _ = db.set_setting("monitor_enabled", if monitor_enabled() { "true" } else { "false" }).await;
                let _ = db.set_setting("retention_days", &retention_days()).await;
                save_notice.set(Some("✓ 系統設定已成功儲存！".to_string()));
            });
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
                            format!("✓ 連線成功！Ollama 正常運行中，本機已安裝 {} 個模型 ({})", count, models.join(", ")),
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
                        cleanup_notice.set(Some(format!("✓ 清理完成！共刪除 {} 筆已收費過期單據與關聯圖片檔案。", count)));
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

            // Section 4: 系統託管儲存路徑
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-5 shadow-sm flex flex-col gap-2",
                h2 { class: "text-sm font-semibold text-slate-100", "應用程式原生託管目錄" }
                p { class: "text-xs text-slate-400", "系統已依據您當前的作業系統，自動將圖片與 SQLite 資料庫存放於標準原生資料目錄中：" }
                div { class: "bg-slate-950 p-3 rounded-lg border border-slate-800 font-mono text-xs text-indigo-300 break-all select-all",
                    "{StorageService::get_images_dir().to_string_lossy()}"
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
        }
    }
}
