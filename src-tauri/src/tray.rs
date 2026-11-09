use std::sync::Mutex;

use chrono::{Local, TimeZone};
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem, Submenu};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{AppHandle, Emitter, Manager, Wry};

use crate::engine::{self, PauseFor};
use crate::error::Result;
use crate::settings::PAUSE_FOREVER;
use crate::state::AppState;
use crate::{main_window, now, srs};

const TRAY_ID: &str = "ppok";

struct TrayItems {
    status: MenuItem<Wry>,
    pause: Submenu<Wry>,
    resume: MenuItem<Wry>,
    last_status: Mutex<String>,
}

pub fn create(app: &AppHandle) -> Result<()> {
    let status = MenuItem::with_id(app, "status", "뽁", false, None::<&str>)?;
    let show_now = MenuItem::with_id(app, "show_now", "지금 단어 보기", true, None::<&str>)?;
    let pause = Submenu::with_id_and_items(
        app,
        "pause",
        "잠시 쉬기",
        true,
        &[
            &MenuItem::with_id(app, "pause_hour", "1시간 쉬기", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause_today", "오늘은 그만", true, None::<&str>)?,
            &MenuItem::with_id(app, "pause_forever", "계속 쉬기", true, None::<&str>)?,
        ],
    )?;
    let resume = MenuItem::with_id(app, "resume", "다시 나오게 하기", false, None::<&str>)?;
    let open = MenuItem::with_id(app, "open", "뽁 열기", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "설정", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "뽁 완전히 종료", true, None::<&str>)?;
    let menu = Menu::with_items(
        app,
        &[
            &status,
            &PredefinedMenuItem::separator(app)?,
            &show_now,
            &pause,
            &resume,
            &PredefinedMenuItem::separator(app)?,
            &open,
            &settings,
            &PredefinedMenuItem::separator(app)?,
            &quit,
        ],
    )?;

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .tooltip("뽁")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(|app, event| {
            let r = match event.id.as_ref() {
                "show_now" => engine::show_next(app).map(|_| ()),
                "pause_hour" => engine::pause(app, PauseFor::Hour),
                "pause_today" => engine::pause(app, PauseFor::Today),
                "pause_forever" => engine::pause(app, PauseFor::Forever),
                "resume" => engine::resume(app),
                "open" => main_window::show(app),
                "settings" => main_window::show(app).and_then(|_| {
                    app.emit_to("main", "ppok:navigate", "settings").map_err(Into::into)
                }),
                "quit" => {
                    app.exit(0);
                    Ok(())
                }
                _ => Ok(()),
            };
            if let Err(e) = r {
                log::error!("트레이 메뉴 처리 실패: {e}");
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                let _ = main_window::show(tray.app_handle());
            }
        });
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder.build(app)?;

    app.manage(TrayItems { status, pause, resume, last_status: Mutex::new(String::new()) });
    refresh(app);
    Ok(())
}

fn hm(ts: i64) -> String {
    Local.timestamp_opt(ts, 0).single().map(|d| d.format("%H:%M").to_string()).unwrap_or_default()
}

pub fn status_text(app: &AppHandle) -> (String, bool) {
    let s = app.state::<AppState>();
    let s = s.lock();
    let t = now();
    let paused = s.settings.is_paused(t);
    let no_words = srs::pool_counts(&s.conn, &s.settings, t).map(|c| c.total == 0).unwrap_or(false);
    let text = if !s.settings.onboarded {
        "설정을 마치면 시작돼요".to_string()
    } else if no_words {
        // 단어장을 끄거나 지워서 출제할 단어가 없으면 '다음 단어' 대신 이걸 알려 준다.
        "보여줄 단어가 없어요 · 단어장을 확인해 주세요".to_string()
    } else if paused {
        match s.settings.paused_until {
            Some(PAUSE_FOREVER) => "쉬는 중".to_string(),
            Some(until) => format!("쉬는 중 · {}부터 다시 나와요", hm(until)),
            None => String::new(),
        }
    } else if !s.settings.within_hours_now() {
        format!("알림 시간대 밖 · {}부터", s.settings.hours_start)
    } else {
        format!("다음 단어 · {}", hm(s.next_popup_at().max(t)))
    };
    (text, paused)
}

/// 상태 줄과 일시정지/재개 메뉴를 현재 상태에 맞춘다. 바뀐 경우에만 갱신.
pub fn refresh(app: &AppHandle) {
    let Some(items) = app.try_state::<TrayItems>() else { return };
    let (text, paused) = status_text(app);
    let mut last = items.last_status.lock().unwrap_or_else(|e| e.into_inner());
    if *last == text {
        return;
    }
    *last = text.clone();
    let _ = items.status.set_text(&text);
    let _ = items.pause.set_enabled(!paused);
    let _ = items.resume.set_enabled(paused);
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_tooltip(Some(format!("뽁 · {text}")));
    }
}
