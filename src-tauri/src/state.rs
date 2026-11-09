use std::sync::{Mutex, MutexGuard};

use rusqlite::Connection;
use serde::Serialize;

use crate::settings::{Corner, Mode, Settings};
use crate::srs::Card;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PopupPayload {
    #[serde(flatten)]
    pub card: Card,
    pub mode: Mode,
    pub seconds: u32,
    /// 이번에 띄울 모서리 (Random은 이미 풀린 상태)
    pub corner: Corner,
    /// 같은 단어가 연달아 나와도 팝업이 새 카드로 인식하도록 하는 일련번호
    pub seq: u64,
}

pub struct Inner {
    pub conn: Connection,
    pub settings: Settings,
    pub last_popup_at: i64,
    pub current: Option<PopupPayload>,
    pub shown_at: i64,
    pub last_word: Option<i64>,
    pub last_corner: Option<Corner>,
    pub seq: u64,
}

/// DB 연결과 실행 중 상태를 하나의 잠금으로 묶어 잠금 순서 문제를 없앤다.
pub struct AppState(Mutex<Inner>);

impl AppState {
    pub fn new(conn: Connection, settings: Settings) -> Self {
        Self(Mutex::new(Inner {
            conn,
            settings,
            last_popup_at: crate::now(),
            current: None,
            shown_at: 0,
            last_word: None,
            last_corner: None,
            seq: 0,
        }))
    }

    pub fn lock(&self) -> MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }
}

impl Inner {
    pub fn next_popup_at(&self) -> i64 {
        self.last_popup_at + self.settings.interval_min as i64 * 60
    }
}
