use dioxus::prelude::*;
use std::collections::HashSet;

use crate::db::Database;
use crate::models::{OverdueKpi, Receipt};
use crate::services::storage::StorageService;
use crate::utils::calculate_overdue_days;
use crate::views::layout::RefreshBadges;

#[component]
pub fn OverdueView() -> Element {
    let db = use_context::<Database>();
    let refresh_badges = use_context::<RefreshBadges>();
    let mut overdue_list = use_signal(Vec::<Receipt>::new);
    let mut kpi = use_signal(OverdueKpi::default);
    let mut selected_ids = use_signal(HashSet::<i64>::new);
    let mut preview_image_url = use_signal(|| Option::<String>::None);

    let reload = {
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                if let Ok(list) = db.get_overdue_receipts().await {
                    overdue_list.set(list);
                }
                if let Ok(kpi_data) = db.get_overdue_kpi().await {
                    kpi.set(kpi_data);
                }
                selected_ids.write().clear();
            });
        }
    };

    use_effect({
        let reload = reload.clone();
        move || {
            reload();
        }
    });

    let selected_count = selected_ids().len();
    let total_overdue = overdue_list().len();

    // Batch mark as paid handler
    let handle_batch_mark_paid = {
        let db = db.clone();
        let reload = reload.clone();
        move |_| {
            let db = db.clone();
            let reload = reload.clone();
            let ids: Vec<i64> = selected_ids().into_iter().collect();
            spawn(async move {
                for id in ids {
                    let _ = db.set_payment_status(id, "paid").await;
                }
                reload();
                refresh_badges.trigger();
            });
        }
    };

    rsx! {
        div { class: "flex flex-col gap-6",
            // Page Header
            div {
                h1 { class: "text-2xl font-bold text-slate-100 tracking-tight", "逾期未收單據催收看板" }
                p { class: "text-xs text-slate-400 mt-1", "專門列管已超過約定應收截止日（due_date）但仍未付款之工單，嚴格受系統保護禁止刪除" }
            }

            // Top 2-Card KPI Banner
            div { class: "grid grid-cols-2 gap-6",
                // Card 1: 逾期未收總金額
                div { class: "bg-rose-950/20 border border-rose-900/40 rounded-xl p-5 flex flex-col gap-1 shadow-sm",
                    div { class: "flex items-center justify-between",
                        span { class: "text-xs font-semibold text-rose-400 uppercase tracking-wider", "逾期未收總金額 (TWD)" }
                        span { class: "text-rose-400/80 text-base", "💰" }
                    }
                    div { class: "text-3xl font-extrabold text-rose-400 font-mono tracking-tight mt-1",
                        "NT$ {kpi().total_amount:.2}"
                    }
                    span { class: "text-[11px] text-rose-400/70 mt-1", "※ 嚴格保護中，未收款前系統絕不允許刪除" }
                }

                // Card 2: 逾期單據總筆數
                div { class: "bg-amber-950/20 border border-amber-900/40 rounded-xl p-5 flex flex-col gap-1 shadow-sm",
                    div { class: "flex items-center justify-between",
                        span { class: "text-xs font-semibold text-amber-400 uppercase tracking-wider", "逾期未收單據總數" }
                        span { class: "text-amber-400/80 text-base", "📋" }
                    }
                    div { class: "text-3xl font-extrabold text-amber-400 font-mono tracking-tight mt-1",
                        "{kpi().total_count} 筆"
                    }
                    span { class: "text-[11px] text-amber-400/70 mt-1", "超過與客戶約定之請款天數，請儘速催收" }
                }
            }

            // Batch Action Row (when items selected)
            if selected_count > 0 {
                div { class: "bg-slate-900 border border-indigo-500/50 rounded-xl p-3 px-4 flex items-center justify-between shadow-md",
                    div { class: "flex items-center gap-2 text-sm text-slate-200",
                        span { "已勾選" }
                        span { class: "font-mono font-bold text-indigo-400 px-1.5 py-0.5 bg-indigo-950 rounded", "{selected_count}" }
                        span { "筆逾期單據" }
                    }
                    div { class: "flex items-center gap-3",
                        button {
                            class: "px-3 py-1.5 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs rounded-lg transition-colors cursor-pointer",
                            onclick: move |_| selected_ids.write().clear(),
                            "取消選取"
                        }
                        button {
                            class: "px-4 py-1.5 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                            onclick: handle_batch_mark_paid,
                            "✓ 批次標記已收費"
                        }
                    }
                }
            }

            // Overdue Table
            div { class: "bg-slate-900 border border-slate-800 rounded-xl overflow-hidden shadow-sm flex flex-col",
                table { class: "w-full text-left border-collapse",
                    thead {
                        tr { class: "bg-slate-800/60 border-b border-slate-800 text-slate-400 text-xs uppercase font-semibold",
                            th { class: "py-3 px-4 w-10 text-center",
                                input {
                                    r#type: "checkbox",
                                    checked: selected_count == total_overdue && total_overdue > 0,
                                    onchange: move |e| {
                                        if e.checked() {
                                            let all_ids: HashSet<i64> = overdue_list().iter().map(|r| r.id).collect();
                                            selected_ids.set(all_ids);
                                        } else {
                                            selected_ids.write().clear();
                                        }
                                    },
                                    class: "cursor-pointer rounded border-slate-700"
                                }
                            }
                            th { class: "py-3 px-4 w-14", "縮圖" }
                            th { class: "py-3 px-4", "工單號碼" }
                            th { class: "py-3 px-4", "施工人員" }
                            th { class: "py-3 px-4", "施工日期" }
                            th { class: "py-3 px-4", "應收截止日" }
                            th { class: "py-3 px-4 text-center", "已逾期天數" }
                            th { class: "py-3 px-4 text-right", "總金額 (TWD)" }
                            th { class: "py-3 px-4 text-center", "快速催收操作" }
                        }
                    }
                    tbody { class: "divide-y divide-slate-800/60 text-sm text-slate-300",
                        if overdue_list().is_empty() {
                            tr {
                                td { colspan: "9", class: "py-16 text-center text-slate-500 text-xs",
                                    "🎉 太棒了！目前沒有任何逾期未收的單據，所有款項均在期限內或已結清。"
                                }
                            }
                        }
                        for r in overdue_list() {
                            {
                                let id = r.id;
                                let is_checked = selected_ids().contains(&id);
                                let overdue_days = calculate_overdue_days(&r.due_date);
                                let thumb_path = r.image_path.clone();

                                rsx! {
                                    tr { key: "{r.id}", class: "hover:bg-slate-800/40 transition-colors",
                                        // 1. Checkbox
                                        td { class: "py-2.5 px-4 text-center",
                                            input {
                                                r#type: "checkbox",
                                                checked: is_checked,
                                                onchange: move |e| {
                                                    if e.checked() {
                                                        selected_ids.write().insert(id);
                                                    } else {
                                                        selected_ids.write().remove(&id);
                                                    }
                                                },
                                                class: "cursor-pointer rounded border-slate-700"
                                            }
                                        }

                                        // 2. Thumbnail
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

                                        // 3. 工單號
                                        td { class: "py-2.5 px-4 font-mono font-medium text-slate-200",
                                            "{r.no.map(|n| n.to_string()).unwrap_or_else(|| \"-\".to_string())}"
                                        }

                                        // 4. 施工人員
                                        td { class: "py-2.5 px-4 text-slate-300",
                                            "{r.matainer.as_deref().unwrap_or(\"-\")}"
                                        }

                                        // 5. 施工日期
                                        td { class: "py-2.5 px-4 font-mono text-slate-300",
                                            "{r.work_date}"
                                        }

                                        // 6. 應收截止日
                                        td { class: "py-2.5 px-4 font-mono text-slate-400",
                                            "{r.due_date}"
                                        }

                                        // 7. 已逾期天數 Badge
                                        td { class: "py-2.5 px-4 text-center",
                                            span { class: "inline-flex items-center px-2.5 py-0.5 rounded-full text-xs font-bold bg-rose-500/10 text-rose-400 border border-rose-500/30 font-mono",
                                                "⚠ 逾期 {overdue_days} 天"
                                            }
                                        }

                                        // 8. 總金額
                                        td { class: "py-2.5 px-4 text-right font-mono tabular-nums font-bold text-rose-400",
                                            "NT$ {r.total_amount:.2}"
                                        }

                                        // 9. 操作
                                        td { class: "py-2.5 px-4 text-center",
                                            div { class: "flex items-center justify-center gap-2",
                                                button {
                                                    class: "px-3 py-1 bg-emerald-600 hover:bg-emerald-500 text-white rounded text-xs font-semibold shadow-sm transition-colors cursor-pointer",
                                                    title: "客戶已結清款項，點擊立即銷帳",
                                                    onclick: {
                                                        let db = db.clone();
                                                        let reload = reload.clone();
                                                        move |_| {
                                                            let db = db.clone();
                                                            let reload = reload.clone();
                                                            spawn(async move {
                                                                let _ = db.set_payment_status(id, "paid").await;
                                                                reload();
                                                                refresh_badges.trigger();
                                                            });
                                                        }
                                                    },
                                                    "✓ 標記為已收費"
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
