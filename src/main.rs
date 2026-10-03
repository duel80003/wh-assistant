#![allow(non_snake_case)]

use dioxus::prelude::*;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

mod db;
mod models;
mod services;
mod utils;
mod views;

use db::Database;
use services::scheduler::SchedulerService;
use services::watcher::WatcherService;
use views::history::HistoryView;
use views::layout::AppShell;
use views::overdue::OverdueView;
use views::review::ReviewView;
use views::settings::SettingsView;

#[derive(Debug, Clone, Routable, PartialEq)]
#[rustfmt::skip]
pub enum Route {
    #[layout(AppShell)]
    #[route("/")]
    ReviewView {},
    #[route("/history")]
    HistoryView {},
    #[route("/overdue")]
    OverdueView {},
    #[route("/settings")]
    SettingsView {},
}

const FAVICON: Asset = asset!("/assets/favicon.ico");
const MAIN_CSS: Asset = asset!("/assets/main.css");
const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let cfg = Config::new().with_window(
            WindowBuilder::new()
                .with_title("WH assistant - 工單與收據管理系統")
                .with_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1280.0, 850.0))
                .with_min_inner_size(dioxus::desktop::tao::dpi::LogicalSize::new(1050.0, 700.0)),
        );
        dioxus::LaunchBuilder::desktop().with_cfg(cfg).launch(App);
    }

    #[cfg(not(feature = "desktop"))]
    {
        dioxus::launch(App);
    }
}

#[component]
fn App() -> Element {
    let mut db_signal = use_signal(|| Option::<Database>::None);
    let mut init_error = use_signal(|| Option::<String>::None);

    use_hook(move || {
        spawn(async move {
            match Database::init().await {
                Ok(db) => {
                    let shutdown = Arc::new(AtomicBool::new(false));

                    // Start background watcher task
                    let db_watch = db.clone();
                    let shutdown_watch = shutdown.clone();
                    tokio::spawn(async move {
                        WatcherService::start_background_watcher(db_watch, shutdown_watch).await;
                    });

                    // Start background scheduler task
                    let db_sched = db.clone();
                    let shutdown_sched = shutdown.clone();
                    tokio::spawn(async move {
                        SchedulerService::start_background_scheduler(db_sched, shutdown_sched)
                            .await;
                    });

                    db_signal.set(Some(db));
                }
                Err(e) => {
                    init_error.set(Some(format!("資料庫初始化失敗: {:#}", e)));
                }
            }
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Link { rel: "stylesheet", href: MAIN_CSS }
        document::Link { rel: "stylesheet", href: TAILWIND_CSS }

        if let Some(err) = init_error() {
            div { class: "h-screen w-screen flex items-center justify-center bg-slate-950 text-rose-400 p-8 text-center",
                div { class: "bg-slate-900 border border-rose-800 p-6 rounded-xl max-w-lg",
                    h2 { class: "text-lg font-bold mb-2", "系統啟動發生錯誤" }
                    p { class: "text-sm text-slate-300 font-mono", "{err}" }
                }
            }
        } else if let Some(db) = db_signal() {
            {
                use_context_provider(|| db);
                rsx! {
                    Router::<Route> {}
                }
            }
        } else {
            div { class: "h-screen w-screen flex flex-col items-center justify-center bg-slate-950 text-slate-300 gap-3",
                div { class: "w-8 h-8 border-4 border-indigo-500 border-t-transparent rounded-full animate-spin" }
                span { class: "text-xs font-mono text-slate-400", "正在初始化系統與資料庫..." }
            }
        }
    }
}
