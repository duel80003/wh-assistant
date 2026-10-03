use crate::db::Database;
use crate::services::ollama::OllamaService;
use crate::services::updater::{UpdateInfo, UpdateStatus, UpdaterService};
use crate::Route;
use dioxus::prelude::*;

#[derive(Clone, Copy)]
pub struct RefreshBadges(pub Signal<u64>);

impl RefreshBadges {
    pub fn trigger(mut self) {
        *self.0.write() += 1;
    }
}

#[component]
pub fn AppShell() -> Element {
    let db = use_context::<Database>();
    let mut update_info = use_context::<Signal<UpdateInfo>>();
    let mut update_status = use_context::<Signal<UpdateStatus>>();
    let mut show_update_modal = use_signal(|| false);

    let mut unconfirmed_count = use_signal(|| 0i64);
    let mut overdue_count = use_signal(|| 0i64);
    let mut ollama_online = use_signal(|| false);
    let refresh_trigger = use_signal(|| 0u64);

    use_context_provider(|| RefreshBadges(refresh_trigger));

    let current_route = use_route::<Route>();
    let mut active_route = use_signal(|| current_route.clone());
    if *active_route.read() != current_route {
        active_route.set(current_route.clone());
    }

    // Refresh badge counts whenever refresh_trigger or route changes
    use_effect({
        let db = db.clone();
        move || {
            let _ = refresh_trigger();
            let _ = active_route();
            let db = db.clone();
            spawn(async move {
                if let Ok(count) = db.get_unconfirmed_count().await {
                    unconfirmed_count.set(count);
                }
                if let Ok(kpi) = db.get_overdue_kpi().await {
                    overdue_count.set(kpi.total_count);
                }
                let url = db
                    .get_setting("ollama_url")
                    .await
                    .unwrap_or(None)
                    .unwrap_or_else(|| "http://localhost:11434".to_string());
                let online = OllamaService::test_connection(&url).await.is_ok();
                ollama_online.set(online);
            });
        }
    });

    // Background periodic refresh (every 4 seconds) to keep badges in sync with filesystem watcher
    use_hook({
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                let mut ticker = 0u32;
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(4)).await;
                    if let Ok(count) = db.get_unconfirmed_count().await {
                        unconfirmed_count.set(count);
                    }
                    if let Ok(kpi) = db.get_overdue_kpi().await {
                        overdue_count.set(kpi.total_count);
                    }
                    ticker += 1;
                    if ticker.is_multiple_of(3) {
                        let url = db
                            .get_setting("ollama_url")
                            .await
                            .unwrap_or(None)
                            .unwrap_or_else(|| "http://localhost:11434".to_string());
                        let online = OllamaService::test_connection(&url).await.is_ok();
                        ollama_online.set(online);
                    }
                }
            });
        }
    });

    rsx! {
        div { class: "flex h-screen w-screen overflow-hidden bg-slate-950 text-slate-100 font-sans",
            // Left Sidebar (240px w-60)
            aside { class: "w-60 bg-slate-900 border-r border-slate-800 flex flex-col justify-between p-4 shrink-0 select-none",
                // Top Brand & Nav
                div { class: "flex flex-col gap-6",
                    // Brand Logo & Title
                    div { class: "flex items-center gap-3 px-2 pt-2",
                        div { class: "w-9 h-9 rounded-xl bg-indigo-600 flex items-center justify-center text-white shadow-md shadow-indigo-600/30 font-extrabold text-lg",
                            "W"
                        }
                        div { class: "flex flex-col",
                            span { class: "font-bold text-sm tracking-wide text-white", "WHassistant" }
                            span { class: "text-[11px] text-slate-400", "工單與收據管理系統" }
                        }
                    }

                    // Navigation List
                    nav { class: "flex flex-col gap-1.5",
                        // 1. 待審單據
                        Link {
                            to: Route::ReviewView {},
                            class: if current_route == (Route::ReviewView {}) {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20 transition-all"
                            } else {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
                            },
                            span { class: "text-base", "📥" }
                            span { "待審單據" }
                            if unconfirmed_count() > 0 {
                                span { class: "ml-auto px-2 py-0.5 rounded-full text-xs font-bold bg-amber-500 text-slate-950 font-mono",
                                    "{unconfirmed_count}"
                                }
                            }
                        }

                        // 2. 歷史單據
                        Link {
                            to: Route::HistoryView {},
                            class: if current_route == (Route::HistoryView {}) {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20 transition-all"
                            } else {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
                            },
                            span { class: "text-base", "📁" }
                            span { "歷史單據" }
                        }

                        // 3. 逾期未收單據
                        Link {
                            to: Route::OverdueView {},
                            class: if current_route == (Route::OverdueView {}) {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20 transition-all"
                            } else {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
                            },
                            span { class: "text-base", "⚠️" }
                            span { "逾期未收" }
                            if overdue_count() > 0 {
                                span { class: "ml-auto px-2 py-0.5 rounded-full text-xs font-bold bg-rose-500 text-white font-mono animate-pulse",
                                    "{overdue_count}"
                                }
                            }
                        }

                        // 4. 系統設定
                        Link {
                            to: Route::SettingsView {},
                            class: if current_route == (Route::SettingsView {}) {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm font-semibold text-white bg-indigo-600/90 shadow-sm shadow-indigo-500/20 transition-all"
                            } else {
                                "flex items-center gap-3 px-3 py-2.5 rounded-lg text-sm text-slate-400 hover:text-slate-200 hover:bg-slate-800/60 transition-colors"
                            },
                            span { class: "text-base", "⚙️" }
                            span { "系統設定" }
                        }
                    }
                }

                // Bottom Status Footer
                div { class: "border-t border-slate-800/80 pt-4 flex flex-col gap-2 px-2 text-xs",
                    div { class: "flex items-center justify-between text-slate-400",
                        span { "Ollama 服務：" }
                        if ollama_online() {
                            span { class: "flex items-center gap-1.5 text-emerald-400 font-medium",
                                span { class: "w-2 h-2 rounded-full bg-emerald-400 animate-pulse" }
                                "已連線"
                            }
                        } else {
                            span { class: "flex items-center gap-1.5 text-rose-400 font-medium",
                                span { class: "w-2 h-2 rounded-full bg-rose-400" }
                                "未連線"
                            }
                        }
                    }
                    div { class: "flex items-center justify-between text-slate-500 text-[11px]",
                        span { "版本" }
                        span { class: "font-mono", "v{env!(\"CARGO_PKG_VERSION\")} (繁體)" }
                    }

                    if update_info().has_update {
                        button {
                            class: "mt-1 w-full flex items-center justify-center gap-1.5 py-1.5 px-2 bg-indigo-600/20 hover:bg-indigo-600/30 text-indigo-400 border border-indigo-500/30 rounded-lg text-xs font-medium transition-colors cursor-pointer animate-pulse",
                            onclick: move |_| show_update_modal.set(true),
                            span { "🚀" }
                            span { "新版本 {update_info().latest_version}" }
                        }
                    }
                }
            }

            // Main Content Area (Auto-scroll)
            main { class: "flex-1 h-screen overflow-y-auto bg-slate-950 p-6 flex flex-col relative",
                // Top Update Alert Bar (if update available)
                if update_info().has_update {
                    div { class: "mb-4 bg-gradient-to-r from-indigo-950/80 via-slate-900 to-indigo-950/80 border border-indigo-500/40 rounded-xl p-3 px-4 flex items-center justify-between shadow-lg text-xs",
                        div { class: "flex items-center gap-2.5",
                            span { class: "text-base", "🎉" }
                            span { class: "text-slate-200",
                                "發現最新版本 "
                                span { class: "font-mono font-bold text-indigo-400", "{update_info().latest_version}" }
                                "（目前版本 v{env!(\"CARGO_PKG_VERSION\")}），支援熱替換自動更新！"
                            }
                        }
                        div { class: "flex items-center gap-2",
                            button {
                                class: "px-3 py-1 bg-indigo-600 hover:bg-indigo-500 text-white font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                onclick: move |_| show_update_modal.set(true),
                                "立即更新"
                            }
                            button {
                                class: "text-slate-400 hover:text-slate-200 px-1.5 py-1 cursor-pointer",
                                onclick: move |_| update_info.write().has_update = false,
                                "✕"
                            }
                        }
                    }
                }

                Outlet::<Route> {}

                // In-App Self-Update Modal Dialog
                if show_update_modal() {
                    div { class: "fixed inset-0 bg-black/80 backdrop-blur-xs z-50 flex items-center justify-center p-4",
                        div { class: "bg-slate-900 border border-indigo-500/40 rounded-2xl max-w-lg w-full p-6 shadow-2xl flex flex-col gap-4 animate-in fade-in zoom-in-95 duration-150",
                            div { class: "flex items-center justify-between border-b border-slate-800 pb-3",
                                div { class: "flex items-center gap-2",
                                    span { class: "text-xl", "🚀" }
                                    h3 { class: "text-base font-bold text-slate-100", "軟體更新 - {update_info().latest_version}" }
                                }
                                if *update_status.read() == UpdateStatus::Idle || matches!(*update_status.read(), UpdateStatus::Failed(_)) {
                                    button {
                                        class: "text-slate-400 hover:text-white text-lg font-bold cursor-pointer",
                                        onclick: move |_| show_update_modal.set(false),
                                        "✕"
                                    }
                                }
                            }

                            div { class: "flex flex-col gap-2.5 text-xs text-slate-300",
                                div { class: "p-3 bg-slate-950 border border-slate-800 rounded-lg flex flex-col gap-1.5 font-mono",
                                    div { class: "flex justify-between",
                                        span { class: "text-slate-500 font-sans", "目前版本：" }
                                        span { class: "text-slate-300", "v{env!(\"CARGO_PKG_VERSION\")}" }
                                    }
                                    div { class: "flex justify-between",
                                        span { class: "text-slate-500 font-sans", "最新發布：" }
                                        span { class: "text-indigo-400 font-bold", "{update_info().latest_version}" }
                                    }
                                }

                                if !update_info().release_notes.is_empty() {
                                    div { class: "flex flex-col gap-1",
                                        span { class: "text-slate-400 font-semibold", "更新說明 (Release Notes)：" }
                                        div { class: "p-2.5 bg-slate-950/70 border border-slate-800 rounded-lg max-h-36 overflow-y-auto text-slate-300 whitespace-pre-wrap leading-relaxed",
                                            "{update_info().release_notes}"
                                        }
                                    }
                                }
                            }

                            // Dynamic status indicator during update
                            match update_status() {
                                UpdateStatus::Idle => rsx! {
                                    div { class: "text-slate-400 text-xs", "點擊下方【開始更新】後，系統將自動從 GitHub 下載最新檔案並完成就地熱替換。" }
                                },
                                UpdateStatus::Downloading { .. } => rsx! {
                                    div { class: "flex items-center gap-3 p-3 bg-indigo-950/30 border border-indigo-500/20 rounded-lg",
                                        div { class: "w-5 h-5 border-2 border-indigo-400 border-t-transparent rounded-full animate-spin shrink-0" }
                                        span { class: "text-xs text-indigo-300", "正在自 GitHub 下載更新壓縮包..." }
                                    }
                                },
                                UpdateStatus::Extracting => rsx! {
                                    div { class: "flex items-center gap-3 p-3 bg-indigo-950/30 border border-indigo-500/20 rounded-lg",
                                        div { class: "w-5 h-5 border-2 border-indigo-400 border-t-transparent rounded-full animate-spin shrink-0" }
                                        span { class: "text-xs text-indigo-300", "正在解壓並進行就地熱替換 (Self-Updating)..." }
                                    }
                                },
                                UpdateStatus::ReadyToRestart => rsx! {
                                    div { class: "p-3 bg-emerald-950/30 border border-emerald-500/30 rounded-lg text-emerald-400 text-xs flex items-center gap-2",
                                        span { "✅" }
                                        span { "更新已安裝成功！點擊下方按鈕重啟以生效。" }
                                    }
                                },
                                UpdateStatus::Failed(err) => rsx! {
                                    div { class: "p-3 bg-rose-950/30 border border-rose-500/30 rounded-lg text-rose-400 text-xs flex flex-col gap-1",
                                        span { class: "font-semibold", "更新失敗：" }
                                        span { class: "font-mono text-[11px]", "{err}" }
                                    }
                                }
                            }

                            // Actions
                            div { class: "flex items-center justify-end gap-3 pt-3 border-t border-slate-800",
                                if *update_status.read() == UpdateStatus::ReadyToRestart {
                                    button {
                                        class: "px-5 py-2 bg-emerald-600 hover:bg-emerald-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                        onclick: move |_| {
                                            let _ = UpdaterService::restart_app();
                                        },
                                        "立即重啟應用程式"
                                    }
                                } else if *update_status.read() == UpdateStatus::Idle || matches!(*update_status.read(), UpdateStatus::Failed(_)) {
                                    button {
                                        class: "px-4 py-2 bg-slate-800 hover:bg-slate-700 text-slate-300 text-xs font-medium rounded-lg transition-colors cursor-pointer",
                                        onclick: move |_| show_update_modal.set(false),
                                        "稍後再說"
                                    }
                                    if let Some(dl_url) = update_info().download_url {
                                        button {
                                            class: "px-5 py-2 bg-indigo-600 hover:bg-indigo-500 text-white text-xs font-semibold rounded-lg shadow-sm transition-colors cursor-pointer",
                                            onclick: {
                                                let dl_url = dl_url.clone();
                                                move |_| {
                                                    let dl_url = dl_url.clone();
                                                    update_status.set(UpdateStatus::Downloading { progress: 0 });
                                                    spawn(async move {
                                                        update_status.set(UpdateStatus::Extracting);
                                                        match UpdaterService::download_and_install_update(&dl_url).await {
                                                            Ok(()) => {
                                                                update_status.set(UpdateStatus::ReadyToRestart);
                                                            }
                                                            Err(e) => {
                                                                update_status.set(UpdateStatus::Failed(format!("{:#}", e)));
                                                            }
                                                        }
                                                    });
                                                }
                                            },
                                            "開始自動更新 (Self-Update)"
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
