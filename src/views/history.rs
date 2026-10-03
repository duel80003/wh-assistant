use dioxus::prelude::*;

use crate::db::Database;
use crate::models::{PaymentTerm, Receipt, ReceiptFilter};
use crate::services::storage::StorageService;
use crate::utils::{calculate_due_date, normalize_work_date};

#[component]
pub fn HistoryView() -> Element {
    let db = use_context::<Database>();
    let mut receipts = use_signal(Vec::<Receipt>::new);
    let mut total_count = use_signal(|| 0i64);
    let mut current_page = use_signal(|| 1i64);
    let mut reload_trigger = use_signal(|| 0i64);
    let page_size = 10i64;

    // Filter signals
    let mut keyword = use_signal(String::new);
    let mut payment_status_filter = use_signal(|| "all".to_string());
    let mut start_date = use_signal(String::new);
    let mut end_date = use_signal(String::new);

    // Modal state for viewing/editing details
    let mut selected_receipt = use_signal(|| Option::<Receipt>::None);
    let mut modal_no = use_signal(String::new);
    let mut modal_matainer = use_signal(String::new);
    let mut modal_work_date = use_signal(String::new);
    let mut modal_due_date = use_signal(String::new);
    let mut modal_total_amount = use_signal(|| 0.0f64);
    let mut modal_term_id = use_signal(|| Option::<i64>::None);
    let mut payment_terms = use_signal(Vec::<PaymentTerm>::new);

    // Image preview modal (full view)
    let mut preview_image_url = use_signal(|| Option::<String>::None);

    // Reactive data fetcher
    use_effect({
        let db = db.clone();
        move || {
            let _ = reload_trigger();
            let kw = keyword();
            let ps = payment_status_filter();
            let sd = start_date();
            let ed = end_date();
            let page = current_page();
            let db = db.clone();

            spawn(async move {
                let filter = ReceiptFilter {
                    keyword: Some(kw).filter(|s| !s.trim().is_empty()),
                    payment_status: Some(ps),
                    start_date: Some(sd).filter(|s| !s.trim().is_empty()),
                    end_date: Some(ed).filter(|s| !s.trim().is_empty()),
                    page,
                    page_size,
                };
                if let Ok((list, total)) = db.get_confirmed_receipts(&filter).await {
                    receipts.set(list);
                    total_count.set(total);
                }
                if let Ok(terms) = db.get_payment_terms().await {
                    payment_terms.set(terms);
                }
            });
        }
    });

    let total_pages = ((total_count() as f64) / (page_size as f64)).ceil() as i64;
    let total_pages = if total_pages == 0 { 1 } else { total_pages };

    rsx! {
        div { class: "flex flex-col gap-6",
            // Page Header
            div { class: "flex items-center justify-between",
                div {
                    h1 { class: "text-2xl font-bold text-slate-100 tracking-tight", "歷史單據庫" }
                    p { class: "text-xs text-slate-400 mt-1", "瀏覽已確認存檔的工單記錄，支援多條件檢索、收費狀態標記與分頁管理" }
                }
                div { class: "text-xs text-slate-400 bg-slate-900 border border-slate-800 px-3 py-1.5 rounded-lg",
                    span { "總計已確認單據：" }
                    span { class: "font-mono font-bold text-indigo-400 ml-1", "{total_count}" }
                    span { " 筆" }
                }
            }

            // Search & Filter Bar
            div { class: "bg-slate-900 border border-slate-800 rounded-xl p-4 flex flex-wrap items-center gap-3",
                // Keyword Search
                div { class: "flex items-center gap-2 bg-slate-950 border border-slate-700 rounded-lg px-3 py-1.5 flex-1 min-w-[200px]",
                    span { class: "text-slate-400 text-sm", "🔍" }
                    input {
                        r#type: "text",
                        placeholder: "搜尋工單號或施工人員...",
                        value: "{keyword}",
                        oninput: move |e| keyword.set(e.value()),
                        class: "w-full bg-transparent text-sm text-slate-100 placeholder-slate-500 outline-none"
                    }
                }

                // Payment Status Filter Dropdown
                div { class: "flex items-center gap-2",
                    label { class: "text-xs text-slate-400", "收費狀態：" }
                    select {
                        class: "bg-slate-950 border border-slate-700 rounded-lg px-3 py-1.5 text-xs text-slate-200 outline-none cursor-pointer",
                        value: "{payment_status_filter}",
                        onchange: move |e| {
                            payment_status_filter.set(e.value());
                            current_page.set(1);
                        },
                        option { value: "all", "全部狀態" }
                        option { value: "unpaid", "未收費" }
                        option { value: "paid", "已收費" }
                    }
                }

                // Start Date
                div { class: "flex items-center gap-1.5",
                    label { class: "text-xs text-slate-400", "施工日期：" }
                    input {
                        r#type: "date",
                        value: "{start_date}",
                        oninput: move |e| {
                            start_date.set(e.value());
                            current_page.set(1);
                        },
                        class: "bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1 text-xs text-slate-200 outline-none font-mono"
                    }
                    span { class: "text-slate-500 text-xs", "～" }
                    input {
                        r#type: "date",
                        value: "{end_date}",
                        oninput: move |e| {
                            end_date.set(e.value());
                            current_page.set(1);
                        },
                        class: "bg-slate-950 border border-slate-700 rounded-lg px-2.5 py-1 text-xs text-slate-200 outline-none font-mono"
                    }
                }

                // Filter Buttons
                div { class: "flex items-center gap-2 ml-auto",
                    button {
                        class: "px-4 py-1.5 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-lg transition-colors cursor-pointer",
                        onclick: move |_| {
                            current_page.set(1);
                            *reload_trigger.write() += 1;
                        },
                        "搜尋"
                    }
                    button {
                        class: "px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium rounded-lg transition-colors cursor-pointer",
                        onclick: move |_| {
                            keyword.set(String::new());
                            payment_status_filter.set("all".to_string());
                            start_date.set(String::new());
                            end_date.set(String::new());
                            current_page.set(1);
                            *reload_trigger.write() += 1;
                        },
                        "重設"
                    }
                }
            }

            // Data Table Container
            div { class: "bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-sm flex flex-col",
                table { class: "w-full text-left border-collapse",
                    thead {
                        tr { class: "bg-slate-800/60 border-b border-slate-800 text-slate-400 text-xs uppercase font-semibold",
                            th { class: "py-3 px-4 w-14", "縮圖" }
                            th { class: "py-3 px-4", "工單號碼" }
                            th { class: "py-3 px-4", "施工人員" }
                            th { class: "py-3 px-4", "施工日期" }
                            th { class: "py-3 px-4", "應收截止日" }
                            th { class: "py-3 px-4 text-right", "總金額 (TWD)" }
                            th { class: "py-3 px-4 text-center", "收費狀態" }
                            th { class: "py-3 px-4 text-center", "操作" }
                        }
                    }
                    tbody { class: "divide-y divide-slate-800/60 text-sm text-slate-300",
                        if receipts().is_empty() {
                            tr {
                                td { colspan: "8", class: "py-16 text-center text-slate-500 text-xs",
                                    "查無符合條件之單據記錄"
                                }
                            }
                        }
                        for r in receipts() {
                            {
                                let receipt_for_toggle = r.clone();
                                let receipt_for_modal = r.clone();
                                let receipt_for_delete = r.clone();
                                let thumb_path = r.image_path.clone();

                                rsx! {
                                    tr { key: "{r.id}", class: "hover:bg-slate-800/40 transition-colors",
                                        // 1. Thumbnail
                                        td { class: "py-2.5 px-4",
                                            if let Some(thumb) = StorageService::read_image_as_data_url(&r.image_path) {
                                                img {
                                                    src: "{thumb}",
                                                    class: "w-10 h-10 object-cover rounded cursor-pointer border border-slate-700 hover:border-indigo-500 transition-colors",
                                                    alt: "縮圖",
                                                    onclick: move |_| preview_image_url.set(Some(thumb_path.clone()))
                                                }
                                            } else {
                                                div { class: "w-10 h-10 bg-slate-800 rounded flex items-center justify-center text-[10px] text-slate-400", "無圖" }
                                            }
                                        }

                                        // 2. 工單號
                                        td { class: "py-2.5 px-4 font-mono font-medium text-slate-200",
                                            "{r.no.map(|n| n.to_string()).unwrap_or_else(|| \"-\".to_string())}"
                                        }

                                        // 3. 施工人員
                                        td { class: "py-2.5 px-4 text-slate-300",
                                            "{r.matainer.as_deref().unwrap_or(\"-\")}"
                                        }

                                        // 4. 施工日期
                                        td { class: "py-2.5 px-4 font-mono text-slate-300",
                                            "{r.work_date}"
                                        }

                                        // 5. 應收截止日
                                        td { class: "py-2.5 px-4 font-mono text-slate-400",
                                            "{r.due_date}"
                                        }

                                        // 6. 總金額
                                        td { class: "py-2.5 px-4 text-right font-mono tabular-nums font-bold text-slate-100",
                                            "NT$ {r.total_amount:.2}"
                                        }

                                        // 7. 收費狀態 Badge
                                        td { class: "py-2.5 px-4 text-center",
                                            if r.payment_status == "paid" {
                                                span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-emerald-500/10 text-emerald-400 border border-emerald-500/30",
                                                    "✓ 已收費"
                                                }
                                            } else {
                                                span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-medium bg-amber-500/10 text-amber-400 border border-amber-500/30",
                                                    "● 未收費"
                                                }
                                            }
                                        }

                                        // 8. Actions
                                        td { class: "py-2.5 px-4 text-center",
                                            div { class: "flex items-center justify-center gap-1.5",
                                                // Toggle Payment Status Button
                                                if r.payment_status == "unpaid" {
                                                    button {
                                                        class: "px-2.5 py-1 bg-emerald-600/20 hover:bg-emerald-600 text-emerald-400 hover:text-white border border-emerald-500/30 rounded text-xs font-medium transition-colors cursor-pointer",
                                                        title: "將此單據標記為已收款",
                                                        onclick: {
                                                            let db = db.clone();
                                                            let item = receipt_for_toggle.clone();
                                                            move |_| {
                                                                let db = db.clone();
                                                                let id = item.id;
                                                                spawn(async move {
                                                                    let _ = db.set_payment_status(id, "paid").await;
                                                                    *reload_trigger.write() += 1;
                                                                });
                                                            }
                                                        },
                                                        "標記已收"
                                                    }
                                                } else {
                                                    button {
                                                        class: "px-2.5 py-1 bg-slate-800 hover:bg-slate-700 text-slate-400 hover:text-slate-200 border border-slate-700 rounded text-xs transition-colors cursor-pointer",
                                                        title: "還原為未收款狀態",
                                                        onclick: {
                                                            let db = db.clone();
                                                            let item = receipt_for_toggle.clone();
                                                            move |_| {
                                                                let db = db.clone();
                                                                let id = item.id;
                                                                spawn(async move {
                                                                    let _ = db.set_payment_status(id, "unpaid").await;
                                                                    *reload_trigger.write() += 1;
                                                                });
                                                            }
                                                        },
                                                        "改為未收"
                                                    }
                                                }

                                                // Edit / Detail Button
                                                button {
                                                    class: "px-2 py-1 text-slate-400 hover:text-indigo-400 hover:bg-slate-800 rounded text-xs transition-colors cursor-pointer",
                                                    title: "檢視與編輯詳細資訊",
                                                    onclick: {
                                                        let item = receipt_for_modal.clone();
                                                        move |_| {
                                                            form_no_setup(
                                                                &item,
                                                                &mut selected_receipt,
                                                                &mut modal_no,
                                                                &mut modal_matainer,
                                                                &mut modal_work_date,
                                                                &mut modal_due_date,
                                                                &mut modal_total_amount,
                                                                &mut modal_term_id,
                                                            );
                                                        }
                                                    },
                                                    "✏️ 編輯"
                                                }

                                                // Delete Button
                                                button {
                                                    class: "px-2 py-1 text-slate-400 hover:text-rose-400 hover:bg-slate-800 rounded text-xs transition-colors cursor-pointer",
                                                    title: "刪除單據",
                                                    onclick: {
                                                        let db = db.clone();
                                                        let item = receipt_for_delete.clone();
                                                        move |_| {
                                                            let db = db.clone();
                                                            let id = item.id;
                                                            spawn(async move {
                                                                let _ = db.delete_receipt(id).await;
                                                                *reload_trigger.write() += 1;
                                                            });
                                                        }
                                                    },
                                                    "🗑"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                // Table Footer Pagination
                div { class: "bg-slate-900 border-t border-slate-800 px-4 py-3 flex items-center justify-between text-xs text-slate-400",
                    span {
                        "顯示第 {(current_page() - 1) * page_size + 1} 至 {((current_page() * page_size).min(total_count()))} 筆，共 {total_count} 筆"
                    }
                    div { class: "flex items-center gap-2",
                        button {
                            disabled: current_page() <= 1,
                            class: "px-3 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded border border-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors cursor-pointer",
                            onclick: move |_| {
                                if current_page() > 1 {
                                    current_page.set(current_page() - 1);
                                }
                            },
                            "上一頁"
                        }
                        span { class: "font-mono font-medium px-2 text-slate-200",
                            "{current_page} / {total_pages}"
                        }
                        button {
                            disabled: current_page() >= total_pages,
                            class: "px-3 py-1 bg-slate-800 hover:bg-slate-700 text-slate-300 rounded border border-slate-700 disabled:opacity-40 disabled:cursor-not-allowed transition-colors cursor-pointer",
                            onclick: move |_| {
                                if current_page() < total_pages {
                                    current_page.set(current_page() + 1);
                                }
                            },
                            "下一頁"
                        }
                    }
                }
            }

            // Edit Detail Modal Dialog
            if let Some(r) = selected_receipt() {
                div { class: "fixed inset-0 bg-black/75 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                    div { class: "bg-slate-900 border border-slate-800 rounded-2xl max-w-xl w-full p-6 shadow-2xl flex flex-col gap-5",
                        div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                            h3 { class: "text-base font-bold text-slate-100", "編輯單據詳細資訊" }
                            button {
                                class: "text-slate-400 hover:text-white text-lg font-bold cursor-pointer",
                                onclick: move |_| selected_receipt.set(None),
                                "✕"
                            }
                        }

                        // Modal Form
                        div { class: "grid grid-cols-2 gap-4",
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "工單號碼" }
                                input {
                                    r#type: "text",
                                    value: "{modal_no}",
                                    oninput: move |e| modal_no.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "施工人員" }
                                input {
                                    r#type: "text",
                                    value: "{modal_matainer}",
                                    oninput: move |e| modal_matainer.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none"
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "施工日期 (基準點)" }
                                input {
                                    r#type: "text",
                                    value: "{modal_work_date}",
                                    oninput: move |e| modal_work_date.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "總金額 (TWD)" }
                                input {
                                    r#type: "number",
                                    step: "any",
                                    value: "{modal_total_amount}",
                                    oninput: move |e| {
                                        if let Ok(v) = e.value().parse::<f64>() {
                                            modal_total_amount.set(v);
                                        }
                                    },
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "收費期限方案" }
                                select {
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none cursor-pointer",
                                    value: modal_term_id().map(|id| id.to_string()).unwrap_or_default(),
                                    onchange: move |e| {
                                        if let Ok(id) = e.value().parse::<i64>() {
                                            modal_term_id.set(Some(id));
                                            if let Some(t) = payment_terms().iter().find(|t| t.id == id) {
                                                let norm = normalize_work_date(&modal_work_date());
                                                modal_due_date.set(calculate_due_date(&norm, t.duration_days));
                                            }
                                        }
                                    },
                                    for term in payment_terms() {
                                        option { value: "{term.id}", "{term.name} ({term.duration_code})" }
                                    }
                                }
                            }
                            div { class: "flex flex-col gap-1.5",
                                label { class: "text-xs font-medium text-slate-300", "應收截止日" }
                                input {
                                    r#type: "text",
                                    value: "{modal_due_date}",
                                    oninput: move |e| modal_due_date.set(e.value()),
                                    class: "w-full bg-slate-950 border border-slate-700 rounded-lg px-3 py-2 text-sm text-slate-100 outline-none font-mono"
                                }
                            }
                        }

                        // Modal Actions
                        div { class: "flex items-center justify-end gap-3 pt-3 border-t border-slate-800",
                            button {
                                class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-sm font-medium rounded-lg transition-colors cursor-pointer",
                                onclick: move |_| selected_receipt.set(None),
                                "取消"
                            }
                            button {
                                class: "px-5 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-sm font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                onclick: {
                                    let db = db.clone();
                                    let r_id = r.id;
                                    let r_curr = r.currency.clone();
                                    let r_img = r.image_path.clone();
                                    let r_st = r.status.clone();
                                    let r_ps = r.payment_status.clone();
                                    let r_pa = r.paid_at.clone();
                                    let r_err = r.error_message.clone();
                                    let r_ca = r.created_at.clone();
                                    let r_ua = r.updated_at.clone();
                                    move |_| {
                                        let db = db.clone();
                                        let parsed_no = crate::utils::normalize_receipt_no(&modal_no());
                                        let r_save = Receipt {
                                            id: r_id,
                                            no: parsed_no,
                                            matainer: Some(modal_matainer().trim().to_string()).filter(|s| !s.is_empty()),
                                            work_date: normalize_work_date(&modal_work_date()),
                                            due_date: modal_due_date().trim().to_string(),
                                            total_amount: modal_total_amount(),
                                            currency: r_curr.clone(),
                                            image_path: r_img.clone(),
                                            status: r_st.clone(),
                                            payment_status: r_ps.clone(),
                                            payment_term_id: modal_term_id(),
                                            paid_at: r_pa.clone(),
                                            error_message: r_err.clone(),
                                            created_at: r_ca.clone(),
                                            updated_at: r_ua.clone(),
                                        };

                                        spawn(async move {
                                            let _ = db.update_receipt(&r_save).await;
                                            selected_receipt.set(None);
                                            *reload_trigger.write() += 1;
                                        });
                                    }
                                },
                                "儲存修改"
                            }
                        }
                    }
                }
            }

            // Image Lightbox Preview Modal
            if let Some(img_name) = preview_image_url() {
                div {
                    class: "fixed inset-0 bg-black/85 backdrop-blur-xs z-50 flex items-center justify-center p-6 cursor-zoom-out",
                    onclick: move |_| preview_image_url.set(None),
                    div { class: "relative max-w-4xl max-h-[85vh] overflow-hidden",
                        if let Some(url) = StorageService::read_image_as_data_url(&img_name) {
                            img { src: "{url}", class: "max-h-[85vh] max-w-full object-contain rounded-lg shadow-2xl", alt: "工單大圖" }
                        }
                    }
                }
            }
        }
    }
}

fn form_no_setup(
    item: &Receipt,
    selected: &mut Signal<Option<Receipt>>,
    no: &mut Signal<String>,
    matainer: &mut Signal<String>,
    work_date: &mut Signal<String>,
    due_date: &mut Signal<String>,
    total_amount: &mut Signal<f64>,
    term_id: &mut Signal<Option<i64>>,
) {
    selected.set(Some(item.clone()));
    no.set(item.no.map(|n| n.to_string()).unwrap_or_default());
    matainer.set(item.matainer.clone().unwrap_or_default());
    work_date.set(item.work_date.clone());
    due_date.set(item.due_date.clone());
    total_amount.set(item.total_amount);
    term_id.set(item.payment_term_id);
}
