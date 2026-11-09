//! 스케줄러·트레이·화면 명령이 함께 쓰는 동작 모음.

use chrono::{Duration, Local, TimeZone};
use tauri::{AppHandle, Emitter, Manager};

use crate::error::Result;
use crate::settings::PAUSE_FOREVER;
use crate::srs::{self, Answer};
use crate::state::{AppState, PopupPayload};
use crate::{now, popup, tray};

/// 메인 창이 열려 있으면 학습 현황을 다시 불러오게 한다.
pub fn emit_changed(app: &AppHandle) {
    let _ = app.emit("ppok:changed", ());
    tray::refresh(app);
}

/// 다음 단어를 골라 팝업으로 보여준다. 출제할 단어가 없으면 false.
pub fn show_next(app: &AppHandle) -> Result<bool> {
    let state = app.state::<AppState>();
    let payload = {
        let mut s = state.lock();
        if let Some(current) = &s.current {
            // 이미 떠 있으면 다시 띄우기만 한다 (트레이에서 '지금 보기'를 또 누른 경우 등).
            let p = current.clone();
            drop(s);
            popup::show(app, &p)?;
            return Ok(true);
        }
        let t = now();
        s.last_popup_at = t;
        let Some(card) = srs::next_card(&s.conn, &s.settings, t, s.last_word)? else {
            drop(s);
            emit_changed(app);
            return Ok(false);
        };
        srs::mark_shown(&s.conn, card.word_id, t)?;
        s.seq += 1;
        s.last_word = Some(card.word_id);
        s.shown_at = t;
        let corner = s.settings.popup_corner.resolve(s.last_corner);
        s.last_corner = Some(corner);
        let payload = PopupPayload {
            card,
            mode: s.settings.mode,
            seconds: s.settings.popup_seconds,
            corner,
            seq: s.seq,
        };
        s.current = Some(payload.clone());
        payload
    };
    popup::show(app, &payload)?;
    emit_changed(app);
    Ok(true)
}

pub fn answer(app: &AppHandle, word_id: i64, answer: Answer) -> Result<()> {
    let state = app.state::<AppState>();
    {
        let mut s = state.lock();
        srs::answer(&s.conn, word_id, answer, now())?;
        if s.current.as_ref().is_some_and(|c| c.card.word_id == word_id) {
            s.current = None;
        }
    }
    popup::hide(app)?;
    emit_changed(app);
    Ok(())
}

#[derive(Debug, Clone, Copy, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PauseFor {
    Hour,
    Today,
    Forever,
}

pub fn pause(app: &AppHandle, how: PauseFor) -> Result<()> {
    let until = match how {
        PauseFor::Hour => now() + 3600,
        PauseFor::Today => {
            let tomorrow = Local::now().date_naive() + Duration::days(1);
            Local
                .from_local_datetime(&tomorrow.and_hms_opt(0, 0, 0).unwrap_or_default())
                .earliest()
                .map(|d| d.timestamp())
                .unwrap_or_else(|| now() + 86_400)
        }
        PauseFor::Forever => PAUSE_FOREVER,
    };
    set_paused(app, Some(until))
}

pub fn resume(app: &AppHandle) -> Result<()> {
    set_paused(app, None)
}

fn set_paused(app: &AppHandle, until: Option<i64>) -> Result<()> {
    let state = app.state::<AppState>();
    {
        let mut s = state.lock();
        s.settings.paused_until = until;
        if until.is_none() {
            // 재개하면 간격을 처음부터 다시 센다.
            s.last_popup_at = now();
        }
        s.settings.save(&s.conn)?;
    }
    emit_changed(app);
    Ok(())
}

/// 오늘 0시(로컬)의 unix 초.
pub fn today_start() -> i64 {
    let today = Local::now().date_naive();
    Local
        .from_local_datetime(&today.and_hms_opt(0, 0, 0).unwrap_or_default())
        .earliest()
        .map(|d| d.timestamp())
        .unwrap_or_else(|| now() - 86_400)
}
