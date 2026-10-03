use crate::db::Database;
use crate::services::ollama::OllamaService;
use crate::services::updater::{UpdateInfo, UpdateStatus, UpdaterService};
use crate::services::watcher::{DirectoryWatcherState, WatcherEvent, WatcherService};
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

    let mut watcher_state = use_signal(DirectoryWatcherState::default);
    use_context_provider(|| watcher_state);
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

    // Background directory watcher event listener
    use_hook({
        let db = db.clone();
        move || {
            let db = db.clone();
            spawn(async move {
                // Initialize initial watcher state from DB
                let enabled = db.get_setting("monitor_enabled").await.unwrap_or(None).unwrap_or_default() == "true";
                let dir_str = db.get_setting("monitor_dir").await.unwrap_or(None).unwrap_or_default();
                if enabled && !dir_str.is_empty() {
                    watcher_state.set(DirectoryWatcherState::Idle { dir: dir_str });
                } else {
                    watcher_state.set(DirectoryWatcherState::Disabled);
                }

                let mut rx = WatcherService::subscribe();
                while let Ok(event) = rx.recv().await {
                    match event {
                        WatcherEvent::TaskStarted { file_name, .. } => {
                            let current_dir = match &*watcher_state.read() {
                                DirectoryWatcherState::Idle { dir } => dir.clone(),
                                DirectoryWatcherState::Processing { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Completed { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Failed { dir, .. } => dir.clone(),
                                _ => String::new(),
                            };
                            watcher_state.set(DirectoryWatcherState::Processing {
                                dir: current_dir,
                                filename: file_name,
                            });
                        }
                        WatcherEvent::TaskCompleted { file_name, receipt_no, .. } => {
                            let current_dir = match &*watcher_state.read() {
                                DirectoryWatcherState::Processing { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Idle { dir } => dir.clone(),
                                DirectoryWatcherState::Completed { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Failed { dir, .. } => dir.clone(),
                                _ => String::new(),
                            };
                            watcher_state.set(DirectoryWatcherState::Completed {
                                dir: current_dir.clone(),
                                filename: file_name,
                                receipt_no,
                            });

                            // Immediate trigger for badge & active page rerender
                            RefreshBadges(refresh_trigger).trigger();

                            // Automatically revert to Idle after 5 seconds
                            let current_dir_for_reset = current_dir.clone();
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                                if matches!(&*watcher_state.read(), DirectoryWatcherState::Completed { .. }) {
                                    watcher_state.set(DirectoryWatcherState::Idle { dir: current_dir_for_reset });
                                }
                            });
                        }
                        WatcherEvent::TaskFailed { file_name, error, .. } => {
                            let current_dir = match &*watcher_state.read() {
                                DirectoryWatcherState::Processing { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Idle { dir } => dir.clone(),
                                DirectoryWatcherState::Completed { dir, .. } => dir.clone(),
                                DirectoryWatcherState::Failed { dir, .. } => dir.clone(),
                                _ => String::new(),
                            };
                            watcher_state.set(DirectoryWatcherState::Failed {
                                dir: current_dir.clone(),
                                filename: file_name,
                                error,
                            });

                            // Immediate trigger for badge & active page rerender
                            RefreshBadges(refresh_trigger).trigger();

                            // Automatically revert to Idle after 8 seconds
                            let current_dir_for_reset = current_dir.clone();
                            spawn(async move {
                                tokio::time::sleep(std::time::Duration::from_secs(8)).await;
                                if matches!(&*watcher_state.read(), DirectoryWatcherState::Failed { .. }) {
                                    watcher_state.set(DirectoryWatcherState::Idle { dir: current_dir_for_reset });
                                }
                            });
                        }
                        WatcherEvent::StatusChanged { enabled, dir } => {
                            if enabled && !dir.is_empty() {
                                if !matches!(&*watcher_state.read(), DirectoryWatcherState::Processing { .. }) {
                                    watcher_state.set(DirectoryWatcherState::Idle { dir });
                                }
                            } else {
                                watcher_state.set(DirectoryWatcherState::Disabled);
                            }
                        }
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

                    // Directory Watcher Status Row
                    match &*watcher_state.read() {
                        DirectoryWatcherState::Disabled => rsx! {
                            div { class: "flex items-center justify-between text-slate-400",
                                span { "目錄監控：" }
                                span { class: "flex items-center gap-1.5 text-slate-500 font-medium",
                                    span { class: "w-2 h-2 rounded-full bg-slate-600" }
                                    "未啟用"
                                }
                            }
                        },
                        DirectoryWatcherState::Idle { .. } => rsx! {
                            div { class: "flex items-center justify-between text-slate-400",
                                span { "目錄監控：" }
                                span { class: "flex items-center gap-1.5 text-emerald-400 font-medium",
                                    span { class: "w-2 h-2 rounded-full bg-emerald-400" }
                                    "監控中"
                                }
                            }
                        },
                        DirectoryWatcherState::Processing { filename, .. } => rsx! {
                            div { class: "flex flex-col gap-1 py-0.5",
                                div { class: "flex items-center justify-between text-slate-400",
                                    span { "目錄監控：" }
                                    span { class: "flex items-center gap-1.5 text-sky-400 font-semibold animate-pulse",
                                        span { class: "w-2 h-2 rounded-full bg-sky-400 animate-ping" }
                                        "辨識處理中"
                                    }
                                }
                                div { class: "text-[11px] text-sky-300 font-mono truncate px-1.5 py-0.5 bg-sky-950/40 rounded border border-sky-500/20",
                                    "📄 {filename}"
                                }
                            }
                        },
                        DirectoryWatcherState::Completed { filename, .. } => rsx! {
                            div { class: "flex flex-col gap-1 py-0.5",
                                div { class: "flex items-center justify-between text-slate-400",
                                    span { "目錄監控：" }
                                    span { class: "flex items-center gap-1.5 text-emerald-400 font-medium",
                                        span { class: "w-2 h-2 rounded-full bg-emerald-400" }
                                        "已完成"
                                    }
                                }
                                div { class: "text-[11px] text-emerald-400 font-mono truncate pl-1",
                                    "✓ {filename}"
                                }
                            }
                        },
                        DirectoryWatcherState::Failed { filename, .. } => rsx! {
                            div { class: "flex flex-col gap-1 py-0.5",
                                div { class: "flex items-center justify-between text-slate-400",
                                    span { "目錄監控：" }
                                    span { class: "flex items-center gap-1.5 text-rose-400 font-medium",
                                        span { class: "w-2 h-2 rounded-full bg-rose-400" }
                                        "失敗"
                                    }
                                }
                                div { class: "text-[11px] text-rose-400 font-mono truncate pl-1",
                                    "✕ {filename}"
                                }
                            }
                        },
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

                // Floating Directory Watcher Background Task Tip Banner
                match &*watcher_state.read() {
                    DirectoryWatcherState::Processing { filename, .. } => rsx! {
                        div {
                            class: "fixed bottom-6 right-6 z-40 max-w-sm bg-slate-900/95 border border-sky-500/50 rounded-xl p-3.5 shadow-2xl backdrop-blur-md flex items-center gap-3 animate-in fade-in slide-in-from-bottom-3 duration-200 pointer-events-auto",
                            div { class: "w-5 h-5 border-2 border-sky-400 border-t-transparent rounded-full animate-spin shrink-0" }
                            div { class: "flex flex-col gap-0.5 min-w-0 flex-1",
                                div { class: "flex items-center gap-2",
                                    span { class: "text-xs font-bold text-sky-400", "目錄監控背景作業中" }
                                    span { class: "text-[10px] px-1.5 py-0.2 rounded bg-sky-500/20 text-sky-300 font-medium animate-pulse", "AI 辨識中" }
                                }
                                span { class: "text-xs text-slate-300 font-mono truncate", "正在辨識：{filename}" }
                            }
                        }
                    },
                    DirectoryWatcherState::Completed { filename, receipt_no, dir } => {
                        let dir = dir.clone();
                        rsx! {
                            div {
                                class: "fixed bottom-6 right-6 z-40 max-w-sm bg-slate-900/95 border border-emerald-500/50 rounded-xl p-3.5 shadow-2xl backdrop-blur-md flex items-center gap-3 animate-in fade-in slide-in-from-bottom-3 duration-200 pointer-events-auto",
                                div { class: "w-6 h-6 rounded-full bg-emerald-500/20 text-emerald-400 flex items-center justify-center font-bold text-xs shrink-0", "✓" }
                                div { class: "flex flex-col gap-0.5 min-w-0 flex-1",
                                    span { class: "text-xs font-bold text-emerald-400", "目錄監控：單據辨識完成" }
                                    span { class: "text-xs text-slate-300 font-mono truncate",
                                        if let Some(no) = receipt_no {
                                            "「{filename}」(工單 #{no}) 已存入待審清單"
                                        } else {
                                            "「{filename}」已成功存入待審清單"
                                        }
                                    }
                                }
                                button {
                                    class: "text-slate-400 hover:text-white text-xs px-1.5 py-1 cursor-pointer",
                                    onclick: {
                                        let dir = dir.clone();
                                        move |_| watcher_state.set(DirectoryWatcherState::Idle { dir: dir.clone() })
                                    },
                                    "✕"
                                }
                            }
                        }
                    },
                    DirectoryWatcherState::Failed { filename, error, dir } => {
                        let dir = dir.clone();
                        rsx! {
                            div {
                                class: "fixed bottom-6 right-6 z-40 max-w-sm bg-slate-900/95 border border-rose-500/50 rounded-xl p-3.5 shadow-2xl backdrop-blur-md flex items-center gap-3 animate-in fade-in slide-in-from-bottom-3 duration-200 pointer-events-auto",
                                div { class: "w-6 h-6 rounded-full bg-rose-500/20 text-rose-400 flex items-center justify-center font-bold text-xs shrink-0", "✕" }
                                div { class: "flex flex-col gap-0.5 min-w-0 flex-1",
                                    span { class: "text-xs font-bold text-rose-400", "目錄監控：單據辨識失敗" }
                                    span { class: "text-xs text-slate-300 font-mono truncate", "檔案：{filename}" }
                                    span { class: "text-[11px] text-rose-400/80 truncate", "{error}" }
                                }
                                button {
                                    class: "text-slate-400 hover:text-white text-xs px-1.5 py-1 cursor-pointer",
                                    onclick: {
                                        let dir = dir.clone();
                                        move |_| watcher_state.set(DirectoryWatcherState::Idle { dir: dir.clone() })
                                    },
                                    "✕"
                                }
                            }
                        }
                    },
                    _ => rsx! {},
                }

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
