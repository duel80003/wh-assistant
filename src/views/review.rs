use dioxus::prelude::*;
use std::path::PathBuf;

use crate::db::Database;
use crate::models::{PaymentTerm, Receipt};
use crate::services::ollama::OllamaService;
use crate::services::storage::StorageService;
use crate::services::watcher::{DirectoryWatcherState, WatcherService};
use crate::utils::{calculate_due_date, normalize_work_date};
use crate::views::layout::RefreshBadges;

#[component]
pub fn ReviewView() -> Element {
    let db = use_context::<Database>();
    let refresh_badges = use_context::<RefreshBadges>();
    let watcher_state = use_context::<Signal<DirectoryWatcherState>>();
    let mut receipts = use_signal(Vec::<Receipt>::new);
    let mut current_idx = use_signal(|| 0usize);
    let mut payment_terms = use_signal(Vec::<PaymentTerm>::new);
    let mut is_loading = use_signal(|| false);
    let mut status_message = use_signal(|| Option::<(String, bool)>::None); // (message, is_error)
    let mut duplicate_prompt = use_signal(|| Option::<PathBuf>::None);

    // Track last loaded receipt ID to avoid overwriting user edits when list updates
    let mut last_loaded_id = use_signal(|| Option::<i64>::None);

    // Form editing state for current receipt
    let mut form_no = use_signal(String::new);
    let mut form_matainer = use_signal(String::new);
    let mut form_work_date = use_signal(String::new);
    let mut form_due_date = use_signal(String::new);
    let mut form_total_amount = use_signal(|| 0.0f64);
    let mut form_term_id = use_signal(|| Option::<i64>::None);

    // Image zoom scale state
    let mut zoom_scale = use_signal(|| 1.0f32);

    // Refresh function
    let reload_data = {
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                is_loading.set(true);
                if let Ok(terms) = db.get_payment_terms().await {
                    payment_terms.set(terms);
                }
                if let Ok(list) = db.get_unconfirmed_receipts().await {
                    let len = list.len();
                    receipts.set(list);
                    if current_idx() >= len && len > 0 {
                        current_idx.set(len - 1);
                    }
                }
                is_loading.set(false);
            });
        }
    };

    // Load data initially and automatically re-render & reload whenever refresh_badges triggers (e.g. background watcher task done)
    use_effect({
        let reload = reload_data.clone();
        move || {
            let _ = refresh_badges.0();
            reload();
        }
    });

    // Update form when current receipt changes or is loaded
    use_effect(move || {
        let list = receipts();
        let idx = current_idx();
        if let Some(r) = list.get(idx) {
            if last_loaded_id() != Some(r.id) {
                last_loaded_id.set(Some(r.id));
                form_no.set(r.no.map(|n| n.to_string()).unwrap_or_default());
                form_matainer.set(r.matainer.clone().unwrap_or_default());
                form_work_date.set(r.work_date.clone());
                form_due_date.set(r.due_date.clone());
                form_total_amount.set(r.total_amount);
                form_term_id.set(r.payment_term_id);
                zoom_scale.set(1.0);
            }
        } else {
            last_loaded_id.set(None);
        }
    });

    // Handlers
    let handle_file_pick = {
        let db = db.clone();
        let reload = reload_data.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            spawn(async move {
                if let Some(handle) = rfd::AsyncFileDialog::new()
                    .add_filter("圖片檔案", &["png", "jpg", "jpeg", "webp"])
                    .pick_file()
                    .await
                {
                    let path = handle.path().to_path_buf();
                    // Check if file hash has already been processed
                    if let Ok(bytes) = std::fs::read(&path) {
                        let hash = StorageService::compute_sha256(&bytes);
                        if let Ok(true) = db.is_file_hash_processed(&hash).await {
                            duplicate_prompt.set(Some(path));
                            return;
                        }
                    }

                    is_loading.set(true);
                    status_message.set(Some((
                        "正在匯入並透過 Ollama 辨識工單中，請稍候...".to_string(),
                        false,
                    )));
                    match WatcherService::process_image_file(&db, &path, false).await {
                        Ok(_) => {
                            status_message
                                .set(Some(("工單圖片匯入並辨識完成！".to_string(), false)));
                            reload();
                            refresh_badges.trigger();
                        }
                        Err(e) => {
                            status_message.set(Some((format!("匯入失敗: {:#}", e), true)));
                        }
                    }
                    is_loading.set(false);
                }
            });
        }
    };

    let handle_confirm_save = {
        let db = db.clone();
        let reload = reload_data.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            let list = receipts();
            let idx = current_idx();
            if let Some(mut r) = list.get(idx).cloned() {
                let parsed_no = crate::utils::normalize_receipt_no(&form_no());
                if parsed_no.is_none() {
                    status_message.set(Some((
                        "工單號碼（NO.）不可為空，請輸入有效純數字工單號！".to_string(),
                        true,
                    )));
                    return;
                }
                r.no = parsed_no;
                r.matainer = Some(form_matainer().trim().to_string()).filter(|s| !s.is_empty());
                r.work_date = normalize_work_date(&form_work_date());
                r.due_date = form_due_date().trim().to_string();
                r.total_amount = form_total_amount();
                r.payment_term_id = form_term_id();
                r.status = "confirmed".to_string();
                r.error_message = None;

                spawn(async move {
                    match db.update_receipt(&r).await {
                        Ok(_) => {
                            status_message.set(Some((
                                "工單已確認並成功歸檔存入資料庫！".to_string(),
                                false,
                            )));
                            reload();
                            refresh_badges.trigger();
                            let _ = crate::services::backup::BackupService::trigger_auto_cloud_backup(&db).await;
                        }
                        Err(e) => {
                            status_message.set(Some((format!("儲存工單失敗: {:#}", e), true)));
                        }
                    }
                });
            }
        }
    };

    let handle_delete = {
        let db = db.clone();
        let reload = reload_data.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            let list = receipts();
            let idx = current_idx();
            if let Some(r) = list.get(idx) {
                let id = r.id;
                spawn(async move {
                    if db.delete_receipt(id).await.is_ok() {
                        status_message.set(Some(("單據及圖片檔案已成功刪除。".to_string(), false)));
                        reload();
                        refresh_badges.trigger();
                        let _ = crate::services::backup::BackupService::trigger_auto_cloud_backup(&db).await;
                    }
                });
            }
        }
    };

    let handle_retry_ollama = {
        let db = db.clone();
        let reload = reload_data.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            let list = receipts();
            let idx = current_idx();
            if let Some(r) = list.get(idx).cloned() {
                spawn(async move {
                    is_loading.set(true);
                    status_message.set(Some((
                        "正在重新呼叫 Ollama 模型進行視覺辨識...".to_string(),
                        false,
                    )));

                    let full_path = StorageService::resolve_image_path(&r.image_path);
                    if let Ok(bytes) = std::fs::read(&full_path) {
                        let url = db
                            .get_setting("ollama_url")
                            .await
                            .unwrap_or(None)
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        let model = db
                            .get_setting("ollama_model")
                            .await
                            .unwrap_or(None)
                            .unwrap_or_else(|| "llama3.2-vision".to_string());

                        match OllamaService::extract_receipt(&url, &model, &bytes).await {
                            Ok(extracted) => {
                                let mut updated = r;
                                updated.no = extracted.no;
                                updated.matainer = extracted.matainer;
                                if let Some(wd) = extracted.work_date {
                                    let norm = normalize_work_date(&wd);
                                    updated.work_date = norm.clone();
                                    updated.due_date = calculate_due_date(&norm, 30);
                                }
                                if let Some(amt) = extracted.total_amount {
                                    updated.total_amount = amt;
                                }
                                updated.status = "unconfirmed".to_string();
                                updated.error_message = None;
                                let _ = db.update_receipt(&updated).await;
                                status_message.set(Some(("重新辨識成功！".to_string(), false)));
                                reload();
                            }
                            Err(e) => {
                                status_message.set(Some((format!("重新辨識失敗: {:#}", e), true)));
                            }
                        }
                    }
                    is_loading.set(false);
                });
            }
        }
    };

    let active_receipt = receipts().get(current_idx()).cloned();
    let total_pending = receipts().len();

    rsx! {
        div { class: "flex flex-col gap-6",
            // Page Header
            div { class: "flex items-center justify-between",
                div {
                    h1 { class: "text-2xl font-bold text-slate-100 tracking-tight", "待審單據工作台" }
                    p { class: "text-xs text-slate-400 mt-1", "拖曳上傳工單照片，由 Ollama 模型自動提取資訊，左圖右表即時核對與確認入庫" }
                }

                if total_pending > 0 {
                    div { class: "flex items-center gap-2 px-3 py-1.5 bg-slate-900 border border-slate-800 rounded-lg text-xs",
                        span { class: "text-slate-400", "待審核進度：" }
                        span { class: "font-mono font-bold text-indigo-400", "{current_idx + 1}" }
                        span { class: "text-slate-500", "/" }
                        span { class: "font-mono text-slate-300", "{total_pending}" }
                    }
                }
            }

            // Background Directory Watcher Working Tip
            if let DirectoryWatcherState::Processing { filename, .. } = &*watcher_state.read() {
                div { class: "bg-sky-950/40 border border-sky-500/40 rounded-xl p-3 px-4 flex items-center justify-between text-xs text-sky-300 shadow-sm animate-pulse",
                    div { class: "flex items-center gap-2.5",
                        div { class: "w-4 h-4 border-2 border-sky-400 border-t-transparent rounded-full animate-spin shrink-0" }
                        span {
                            "目錄監控正在背景辨識單據 "
                            span { class: "font-mono font-bold text-white", "「{filename}」" }
                            "（辨識完成後將自動為您刷新此頁面）..."
                        }
                    }
                    span { class: "text-[11px] px-2 py-0.5 bg-sky-500/20 text-sky-300 rounded font-medium", "AI 模型運算中" }
                }
            }

            // Top Status / Alert Message Banner
            if let Some((msg, is_err)) = status_message() {
                div {
                    class: if is_err {
                        "px-4 py-3 rounded-xl bg-rose-950/40 border border-rose-800 text-rose-300 text-xs flex items-center justify-between"
                    } else {
                        "px-4 py-3 rounded-xl bg-indigo-950/40 border border-indigo-800 text-indigo-300 text-xs flex items-center justify-between"
                    },
                    span { "{msg}" }
                    button {
                        class: "text-slate-400 hover:text-white text-sm font-bold ml-4 cursor-pointer",
                        onclick: move |_| status_message.set(None),
                        "✕"
                    }
                }
            }

            // Drop Zone Banner
            div {
                class: "border-2 border-dashed border-slate-700 hover:border-indigo-500 rounded-xl bg-slate-900/50 hover:bg-indigo-950/20 py-6 px-4 flex flex-col items-center justify-center cursor-pointer transition-all gap-2 group",
                onclick: handle_file_pick,
                span { class: "text-3xl group-hover:scale-110 transition-transform", "📷" }
                p { class: "text-sm font-medium text-slate-200", "點擊選擇或拖曳工單圖片至此處上傳" }
                span { class: "text-xs text-slate-500", "支援格式：PNG、JPG、JPEG、WEBP (自動儲存至應用託管目錄)" }
            }

            // Split-Screen Reviewer
            if let Some(r) = active_receipt {
                div { class: "grid grid-cols-12 gap-6 min-h-[550px]",
                    // Left Screen: 45% (col-span-5) Image Preview Container
                    div { class: "col-span-5 bg-slate-900 border border-slate-800 rounded-xl p-4 flex flex-col items-center justify-center relative overflow-hidden group select-none",
                        // Top Floating Toolbar
                        div { class: "absolute top-3 right-3 bg-slate-800/90 backdrop-blur rounded-lg p-1 flex gap-1 border border-slate-700 z-10",
                            button {
                                class: "p-1.5 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded transition-colors",
                                title: "放大",
                                onclick: move |_| zoom_scale.set((zoom_scale() + 0.2).min(3.0)),
                                "🔍+"
                            }
                            button {
                                class: "p-1.5 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded transition-colors",
                                title: "縮小",
                                onclick: move |_| zoom_scale.set((zoom_scale() - 0.2).max(0.5)),
                                "🔍-"
                            }
                            button {
                                class: "p-1.5 text-xs text-slate-300 hover:text-white hover:bg-slate-700 rounded transition-colors",
                                title: "重設大小",
                                onclick: move |_| zoom_scale.set(1.0),
                                "↺"
                            }
                        }

                        // Image Rendering
                        div { class: "w-full h-full flex items-center justify-center overflow-auto p-2",
                            if let Some(data_url) = StorageService::read_image_as_data_url(&r.image_path) {
                                img {
                                    src: "{data_url}",
                                    style: "transform: scale({zoom_scale()}); transition: transform 0.15s ease-out;",
                                    class: "max-h-[480px] max-w-full object-contain rounded shadow-lg",
                                    alt: "工單原圖"
                                }
                            } else {
                                div { class: "text-slate-500 text-xs text-center", "無法載入圖片檔案" }
                            }
                        }
                    }

                    // Right Screen: 55% (col-span-7) Form Editing Area
                    div { class: "col-span-7 bg-slate-900 border border-slate-800 rounded-xl p-6 flex flex-col justify-between",
                        div { class: "flex flex-col gap-5",
                            // Title & Status
                            div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                                h2 { class: "text-base font-semibold text-slate-100 flex items-center gap-2",
                                    "核對單據資料"
                                }
                                if r.status == "processing" {
                                    span { class: "inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-xs font-medium bg-sky-500/10 text-sky-400 border border-sky-500/30 animate-pulse",
                                        "辨識中..."
                                    }
                                } else if r.status == "failed" {
                                    span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-semibold bg-rose-500/10 text-rose-400 border border-rose-500/30",
                                        "⚠ 辨識失敗"
                                    }
                                } else {
                                    span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30",
                                        "● 待確認"
                                    }
                                }
                            }

                            // Error notice if failed
                            if let Some(ref err) = r.error_message {
                                div { class: "p-3 rounded-lg bg-rose-950/30 border border-rose-800/50 text-rose-300 text-xs",
                                    span { class: "font-bold", "模型解析提示：" }
                                    span { "{err}" }
                                }
                            }

                            // Form Fields Grid (2 columns)
                            div { class: "grid grid-cols-2 gap-4",
                                // 1. 工單號碼
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "工單號碼 (純數字，唯一鍵)" }
                                    input {
                                        r#type: "text",
                                        placeholder: "例如：12345678 (前綴NO.將自動去除)",
                                        value: "{form_no}",
                                        oninput: move |e| form_no.set(e.value()),
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none font-mono"
                                    }
                                }

                                // 2. 施工人員
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "施工人員 / 保養者" }
                                    input {
                                        r#type: "text",
                                        placeholder: "例如：王小明",
                                        value: "{form_matainer}",
                                        oninput: move |e| form_matainer.set(e.value()),
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none"
                                    }
                                }

                                // 3. 施工日期 (基準點)
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "施工日期 (計算基準點)" }
                                    input {
                                        r#type: "text",
                                        placeholder: "YYYY-MM-DD",
                                        value: "{form_work_date}",
                                        oninput: move |e| {
                                            let v = e.value();
                                            form_work_date.set(v.clone());
                                            // auto update due date if valid
                                            let norm = normalize_work_date(&v);
                                            let days = payment_terms().iter()
                                                .find(|t| Some(t.id) == form_term_id())
                                                .map(|t| t.duration_days)
                                                .unwrap_or(30);
                                            form_due_date.set(calculate_due_date(&norm, days));
                                        },
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none font-mono"
                                    }
                                }

                                // 4. 總金額
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "總金額 (TWD)" }
                                    input {
                                        r#type: "number",
                                        step: "any",
                                        placeholder: "0.00",
                                        value: "{form_total_amount}",
                                        oninput: move |e| {
                                            if let Ok(val) = e.value().parse::<f64>() {
                                                form_total_amount.set(val);
                                            }
                                        },
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none font-mono font-semibold"
                                    }
                                }

                                // 5. 收費期限方案
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "收費期限方案" }
                                    select {
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none cursor-pointer",
                                        value: form_term_id().map(|id| id.to_string()).unwrap_or_default(),
                                        onchange: move |e| {
                                            if let Ok(id) = e.value().parse::<i64>() {
                                                form_term_id.set(Some(id));
                                                if let Some(t) = payment_terms().iter().find(|t| t.id == id) {
                                                    let norm = normalize_work_date(&form_work_date());
                                                    form_due_date.set(calculate_due_date(&norm, t.duration_days));
                                                }
                                            }
                                        },
                                        for term in payment_terms() {
                                            option { value: "{term.id}", "{term.name} ({term.duration_code})" }
                                        }
                                    }
                                }

                                // 6. 應收截止日
                                div { class: "flex flex-col gap-1.5",
                                    label { class: "text-xs font-medium text-slate-300", "應收截止日 (可手動微調)" }
                                    input {
                                        r#type: "text",
                                        placeholder: "YYYY-MM-DD",
                                        value: "{form_due_date}",
                                        oninput: move |e| form_due_date.set(e.value()),
                                        class: "w-full bg-slate-950 border border-slate-700 focus:border-indigo-500 focus:ring-1 focus:ring-indigo-500 rounded-lg px-3 py-2 text-sm text-slate-100 placeholder-slate-500 outline-none font-mono"
                                    }
                                }
                            }
                        }

                        // Bottom Actions Row
                        div { class: "flex items-center justify-between pt-6 border-t border-slate-800 mt-6",
                            // Left Danger / Retry Buttons
                            div { class: "flex items-center gap-2",
                                button {
                                    class: "inline-flex items-center justify-center gap-1.5 px-3 py-2 bg-rose-600/10 hover:bg-rose-600 text-rose-400 hover:text-white border border-rose-500/20 text-xs font-semibold rounded-lg transition-all cursor-pointer",
                                    onclick: handle_delete,
                                    "🗑 刪除單據"
                                }
                                button {
                                    class: "inline-flex items-center justify-center gap-1.5 px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-200 text-xs font-medium rounded-lg border border-slate-700 transition-all cursor-pointer",
                                    onclick: handle_retry_ollama,
                                    "↻ 重新辨識"
                                }
                            }

                            // Right Paging & Save Buttons
                            div { class: "flex items-center gap-3",
                                button {
                                    disabled: current_idx() == 0,
                                    class: "px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium rounded-lg border border-slate-700 transition-all disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer",
                                    onclick: move |_| {
                                        if current_idx() > 0 {
                                            current_idx.set(current_idx() - 1);
                                        }
                                    },
                                    "◀ 上一筆"
                                }
                                button {
                                    disabled: current_idx() + 1 >= total_pending,
                                    class: "px-3 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium rounded-lg border border-slate-700 transition-all disabled:opacity-40 disabled:cursor-not-allowed cursor-pointer",
                                    onclick: move |_| {
                                        if current_idx() + 1 < total_pending {
                                            current_idx.set(current_idx() + 1);
                                        }
                                    },
                                    "下一筆 ▶"
                                }
                                button {
                                    class: "inline-flex items-center justify-center gap-2 px-5 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-semibold rounded-lg shadow-sm shadow-indigo-600/30 transition-all cursor-pointer",
                                    onclick: handle_confirm_save,
                                    "✓ 確認儲存"
                                }
                            }
                        }
                    }
                }

                // Bottom Queue Carousel Ribbon
                if total_pending > 1 {
                    div { class: "flex flex-col gap-2 pt-2",
                        span { class: "text-xs font-medium text-slate-400", "待審核佇列縮圖：" }
                        div { class: "flex items-center gap-3 overflow-x-auto pb-2",
                            for (idx, item) in receipts().iter().enumerate() {
                                div {
                                    key: "{item.id}",
                                    class: if idx == current_idx() {
                                        "w-20 h-20 rounded-lg overflow-hidden border-2 border-indigo-500 p-0.5 cursor-pointer shrink-0 transition-transform scale-105"
                                    } else {
                                        "w-20 h-20 rounded-lg overflow-hidden border border-slate-800 hover:border-slate-600 p-0.5 cursor-pointer shrink-0 opacity-70 hover:opacity-100 transition-opacity"
                                    },
                                    onclick: move |_| current_idx.set(idx),
                                    if let Some(thumb_url) = StorageService::read_image_as_data_url(&item.image_path) {
                                        img { src: "{thumb_url}", class: "w-full h-full object-cover rounded", alt: "縮圖" }
                                    } else {
                                        div { class: "w-full h-full bg-slate-800 flex items-center justify-center text-[10px] text-slate-400", "圖檔" }
                                    }
                                }
                            }
                        }
                    }
                }
            } else {
                // Empty Queue State
                div { class: "flex flex-col items-center justify-center py-20 text-center gap-3 bg-slate-900/40 rounded-xl border border-slate-800/80",
                    span { class: "text-5xl", "🎉" }
                    h3 { class: "text-base font-bold text-slate-200", "目前沒有待確認的工單" }
                    p { class: "text-xs text-slate-400 max-w-md",
                        "您可以點選上方拖曳區域手動匯入新圖片，或前往【系統設定】開啟目錄自動監聽功能。"
                    }
                }
            }

            // Duplicate Photo Prompt Modal
            if let Some(dup_path) = duplicate_prompt() {
                div {
                    class: "fixed inset-0 bg-black/80 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                    div {
                        class: "bg-slate-900 border border-amber-500/40 rounded-xl p-6 max-w-md w-full shadow-2xl flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150",
                        div { class: "flex items-center gap-3 text-amber-400 font-semibold text-base",
                            span { class: "text-2xl", "⚠️" }
                            span { "偵測到重複照片" }
                        }
                        p { class: "text-sm text-slate-300 leading-relaxed",
                            "此工單照片先前已經上傳處理過並存在紀錄。請問是否仍要繼續進行辨識並建立工單？"
                        }
                        div { class: "flex justify-end gap-3 pt-3 border-t border-slate-800",
                            button {
                                class: "px-4 py-2 text-xs font-medium text-slate-400 hover:text-slate-200 hover:bg-slate-800 rounded-lg transition-colors cursor-pointer",
                                onclick: move |_| duplicate_prompt.set(None),
                                "取消上傳"
                            }
                            button {
                                class: "px-4 py-2 text-xs font-medium text-white bg-indigo-600 hover:bg-indigo-500 rounded-lg shadow transition-colors cursor-pointer",
                                onclick: {
                                    let db = db.clone();
                                    let reload = reload_data.clone();
                                    let path = dup_path.clone();
                                    move |_| {
                                        duplicate_prompt.set(None);
                                        let db = db.clone();
                                        let reload = reload.clone();
                                        let path = path.clone();
                                        spawn(async move {
                                            is_loading.set(true);
                                            status_message.set(Some(("正在強制匯入並透過 Ollama 辨識工單中，請稍候...".to_string(), false)));
                                            match WatcherService::process_image_file(&db, &path, true).await {
                                                Ok(_) => {
                                                    status_message.set(Some(("工單圖片匯入並辨識完成！".to_string(), false)));
                                                    reload();
                                                    refresh_badges.trigger();
                                                }
                                                Err(e) => {
                                                    status_message.set(Some((format!("匯入失敗: {:#}", e), true)));
                                                }
                                            }
                                            is_loading.set(false);
                                        });
                                    }
                                },
                                "仍要繼續處理"
                            }
                        }
                    }
                }
            }
        }
    }
}
