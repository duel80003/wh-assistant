use dioxus::prelude::*;
use crate::Route;
use crate::db::Database;
use crate::services::ollama::OllamaService;

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
                let url = db.get_setting("ollama_url").await.unwrap_or(None).unwrap_or_else(|| "http://localhost:11434".to_string());
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
                    if ticker % 3 == 0 {
                        let url = db.get_setting("ollama_url").await.unwrap_or(None).unwrap_or_else(|| "http://localhost:11434".to_string());
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
                        span { class: "font-mono", "v0.1.0 (繁體)" }
                    }
                }
            }

            // Main Content Area (Auto-scroll)
            main { class: "flex-1 h-screen overflow-y-auto bg-slate-950 p-6 flex flex-col",
                Outlet::<Route> {}
            }
        }
    }
}
