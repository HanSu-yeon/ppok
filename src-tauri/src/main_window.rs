use tauri::{AppHandle, Manager};
use tauri_plugin_dialog::DialogExt;

use crate::db;
use crate::error::Result;
use crate::state::AppState;

pub const LABEL: &str = "main";

pub fn show(app: &AppHandle) -> Result<()> {
    if let Some(w) = app.get_webview_window(LABEL) {
        w.show()?;
        w.unminimize()?;
        w.set_focus()?;
    }
    Ok(())
}

#[cfg(windows)]
const CLOSE_HINT: &str = "창을 닫아도 뽁은 계속 실행되고, 정해진 시간마다 단어 카드가 나와요.\n\n\
다시 열거나 완전히 끄려면 화면 오른쪽 아래 작업 표시줄의 뽁 아이콘을 눌러 주세요.\n\
아이콘이 안 보이면 작업 표시줄의 ^ 버튼 안에 있어요.";
#[cfg(not(windows))]
const CLOSE_HINT: &str = "창을 닫아도 뽁은 계속 실행되고, 정해진 시간마다 단어 카드가 나와요.\n\n\
다시 열거나 완전히 끄려면 화면 위쪽 메뉴 막대의 뽁 아이콘을 눌러 주세요.";

/// 처음 창을 닫을 때 한 번만, 앱이 꺼진 게 아니라고 알려 준다.
pub fn show_close_hint_once(app: &AppHandle) {
    const KEY: &str = "close_hint_shown";
    let state = app.state::<AppState>();
    {
        let s = state.lock();
        if db::get_meta(&s.conn, KEY).ok().flatten().is_some() {
            return;
        }
        let _ = db::set_meta(&s.conn, KEY, "1");
    }
    app.dialog().message(CLOSE_HINT).title("뽁은 계속 실행 중이에요").show(|_| {});
}
