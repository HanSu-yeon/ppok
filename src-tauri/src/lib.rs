mod commands;
mod db;
mod engine;
mod error;
mod import;
mod main_window;
mod platform;
mod popup;
mod scheduler;
mod seed;
mod settings;
mod srs;
mod state;
mod tray;
mod validate;

use tauri::{Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use crate::settings::Settings;
use crate::state::AppState;

/// 자동 실행으로 켜졌을 때 붙는 인자. 이때는 창을 띄우지 않고 트레이에만 둔다.
const MINIMIZED_ARG: &str = "--minimized";

pub fn now() -> i64 {
    chrono::Utc::now().timestamp()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            let _ = main_window::show(app);
        }))
        .plugin(tauri_plugin_log::Builder::default().level(log::LevelFilter::Info).build())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, Some(vec![MINIMIZED_ARG])))
        .setup(|app| {
            let data_dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&data_dir)?;
            let mut conn = db::open(&data_dir.join("ppok.db"))?;

            let words_dir = app.path().resource_dir()?.join("words");
            if let Err(e) = seed::sync_all(&mut conn, &words_dir) {
                log::error!("기본 단어장을 불러오지 못했어요: {e}");
            }

            let settings = Settings::load(&conn)?;
            let onboarded = settings.onboarded;
            app.manage(AppState::new(conn, settings));

            popup::create(app.handle())?;
            tray::create(app.handle())?;
            scheduler::start(app.handle().clone());

            let minimized = std::env::args().any(|a| a == MINIMIZED_ARG);
            if !minimized || !onboarded {
                main_window::show(app.handle())?;
            }
            Ok(())
        })
        .on_window_event(|window, event| {
            // 메인 창을 닫으면 숨기기만 한다. 종료는 트레이 메뉴에서.
            if let WindowEvent::CloseRequested { api, .. } = event
                && window.label() == main_window::LABEL
            {
                api.prevent_close();
                let _ = window.hide();
                main_window::show_close_hint_once(window.app_handle());
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::get_bootstrap,
            commands::save_settings,
            commands::get_today,
            commands::show_now,
            commands::pause,
            commands::resume,
            commands::list_words,
            commands::list_user_decks,
            commands::set_deck_enabled,
            commands::delete_deck,
            commands::preview_import,
            commands::commit_import,
            commands::save_template,
            commands::set_autostart,
            commands::get_autostart,
            commands::get_seed_reports,
            commands::get_data_dir,
            commands::popup_current,
            commands::popup_answer,
        ])
        .build(tauri::generate_context!())
        .expect("뽁을 시작하지 못했어요")
        .run(|_app, event| {
            // 창이 모두 숨겨져도 트레이에서 계속 실행한다. 명시적 종료(app.exit)만 허용.
            if let RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
