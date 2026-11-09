//! 15초마다 팝업을 띄울 때인지 판단한다. 메인 창이 닫혀 있어도 동작한다.

use std::thread;
use std::time::Duration;

use tauri::{AppHandle, Manager};

use crate::error::Result;
use crate::srs::Answer;
use crate::state::AppState;
use crate::{engine, now, platform, tray};

const TICK: Duration = Duration::from_secs(15);
/// 팝업이 응답 없이 이 시간 넘게 떠 있으면 자동으로 사라진 것으로 처리한다.
const STALE_POPUP_SECS: i64 = 10 * 60;

pub fn start(app: AppHandle) {
    thread::Builder::new()
        .name("ppok-scheduler".into())
        .spawn(move || {
            loop {
                thread::sleep(TICK);
                if let Err(e) = tick(&app) {
                    log::error!("스케줄러 오류: {e}");
                }
            }
        })
        .expect("scheduler thread");
}

fn tick(app: &AppHandle) -> Result<()> {
    let t = now();
    let state = app.state::<AppState>();
    let (due, stale_word) = {
        let mut s = state.lock();
        // 기한이 지난 일시정지는 풀어 둔다.
        if matches!(s.settings.paused_until, Some(until) if until <= t) {
            s.settings.paused_until = None;
            s.settings.save(&s.conn)?;
        }
        let stale = s.current.as_ref().filter(|_| t - s.shown_at > STALE_POPUP_SECS).map(|c| c.card.word_id);
        let due = s.settings.onboarded
            && s.current.is_none()
            && !s.settings.is_paused(t)
            && s.settings.within_hours_now()
            && t >= s.next_popup_at();
        (due, stale)
    };

    if let Some(word_id) = stale_word {
        engine::answer(app, word_id, Answer::Timeout)?;
    }
    if due {
        if platform::should_suppress_popup() {
            // 전체 화면 중에는 건너뛰고 다음 틱에 다시 확인한다.
            log::info!("전체 화면 앱 실행 중 — 팝업 보류");
        } else {
            engine::show_next(app)?;
        }
    }
    tray::refresh(app);
    Ok(())
}
