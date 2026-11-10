use std::path::PathBuf;

use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_autostart::ManagerExt;

use crate::engine::{self, PauseFor};
use crate::error::{Error, Result};
use crate::import::{self, Preview};
use crate::seed::{self, SeedReport};
use crate::settings::Settings;
use crate::srs::{self, Answer, DayStats, PoolCounts};
use crate::state::{AppState, PopupPayload};
use crate::validate::levels_for;
use crate::{now, tray};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LevelCount {
    level: String,
    total: i64,
    learning: i64,
    known: i64,
    /// 난이도를 고를 때 감을 잡도록 보여주는 앞쪽 단어 몇 개
    examples: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LanguageInfo {
    code: String,
    available: bool,
    deck_id: Option<i64>,
    levels: Vec<LevelCount>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Bootstrap {
    settings: Settings,
    languages: Vec<LanguageInfo>,
    user_word_count: i64,
}

fn language_info(conn: &rusqlite::Connection, code: &str) -> Result<LanguageInfo> {
    let deck_id: Option<i64> = conn
        .query_row("SELECT id FROM decks WHERE kind = 'builtin' AND language = ?1", [code], |r| r.get(0))
        .optional()?;
    let mut levels: Vec<LevelCount> = levels_for(code)
        .iter()
        .map(|l| LevelCount { level: l.to_string(), total: 0, learning: 0, known: 0, examples: vec![] })
        .collect();
    if let Some(id) = deck_id {
        let mut stmt = conn.prepare(
            "SELECT w.level, COUNT(*),
                    COALESCE(SUM(p.status = 'learning'), 0),
                    COALESCE(SUM(p.status = 'known'), 0)
             FROM words w LEFT JOIN progress p ON p.word_id = w.id
             WHERE w.deck_id = ?1 AND w.retired = 0
             GROUP BY w.level",
        )?;
        let rows = stmt.query_map([id], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?, r.get::<_, i64>(3)?))
        })?;
        for row in rows {
            let (level, total, learning, known) = row?;
            if let Some(l) = levels.iter_mut().find(|l| l.level == level) {
                (l.total, l.learning, l.known) = (total, learning, known);
            }
        }
        let mut examples = conn.prepare(
            "SELECT term FROM words WHERE deck_id = ?1 AND level = ?2 AND retired = 0 ORDER BY rank LIMIT 3",
        )?;
        for l in levels.iter_mut().filter(|l| l.total > 0) {
            l.examples = examples
                .query_map(params![id, l.level], |r| r.get(0))?
                .collect::<rusqlite::Result<Vec<String>>>()?;
        }
    }
    let available = levels.iter().any(|l| l.total > 0);
    Ok(LanguageInfo { code: code.into(), available, deck_id, levels })
}

#[tauri::command]
pub fn get_bootstrap(state: State<AppState>) -> Result<Bootstrap> {
    let s = state.lock();
    let languages = seed::LANGUAGES.iter().map(|c| language_info(&s.conn, c)).collect::<Result<Vec<_>>>()?;
    let user_word_count = s.conn.query_row(
        "SELECT COUNT(*) FROM words w JOIN decks d ON d.id = w.deck_id WHERE d.kind = 'user'",
        [],
        |r| r.get(0),
    )?;
    Ok(Bootstrap { settings: s.settings.clone(), languages, user_word_count })
}

#[tauri::command]
pub fn save_settings(app: AppHandle, state: State<AppState>, mut settings: Settings) -> Result<Settings> {
    settings.validate()?;
    let just_onboarded = {
        let mut s = state.lock();
        // 일시정지·자동 실행은 전용 명령으로만 바꾼다 (화면이 들고 있던 오래된 값으로 덮어쓰지 않게).
        settings.paused_until = s.settings.paused_until;
        settings.autostart = s.settings.autostart;
        let just_onboarded = !s.settings.onboarded && settings.onboarded;
        if just_onboarded || settings.interval_min != s.settings.interval_min {
            s.last_popup_at = now();
        }
        settings.save(&s.conn)?;
        s.settings = settings.clone();
        just_onboarded
    };
    if just_onboarded {
        // 온보딩을 마치면 첫 단어가 바로 뽁!
        let app = app.clone();
        std::thread::spawn(move || {
            std::thread::sleep(std::time::Duration::from_millis(1200));
            if let Err(e) = engine::show_next(&app) {
                log::error!("첫 단어 표시 실패: {e}");
            }
        });
    }
    engine::emit_changed(&app);
    Ok(settings)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Today {
    #[serde(flatten)]
    day: DayStats,
    goal: u32,
    pool: PoolCounts,
    next_popup_at: i64,
    paused_until: Option<i64>,
    within_hours: bool,
    status_text: String,
    /// true면 화면이 nextPopupAt까지 남은 시간을 실시간으로 센다
    counting: bool,
    showing: bool,
}

#[tauri::command]
pub fn get_today(app: AppHandle, state: State<AppState>) -> Result<Today> {
    let (status_text, _, counting) = tray::status_text(&app);
    let s = state.lock();
    let t = now();
    Ok(Today {
        day: srs::day_stats(&s.conn, engine::today_start())?,
        goal: s.settings.daily_goal,
        pool: srs::pool_counts(&s.conn, &s.settings, t)?,
        next_popup_at: s.next_popup_at(),
        paused_until: s.settings.paused_until.filter(|u| *u > t),
        within_hours: s.settings.within_hours_now(),
        status_text,
        counting,
        showing: s.current.is_some(),
    })
}

#[tauri::command]
pub fn show_now(app: AppHandle) -> Result<bool> {
    engine::show_next(&app)
}

#[tauri::command]
pub fn pause(app: AppHandle, how: PauseFor) -> Result<()> {
    engine::pause(&app, how)
}

#[tauri::command]
pub fn resume(app: AppHandle) -> Result<()> {
    engine::resume(&app)
}

// ---------- 단어 목록 ----------

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WordQuery {
    deck_id: i64,
    level: Option<String>,
    query: Option<String>,
    offset: i64,
    limit: i64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WordRow {
    id: i64,
    term: String,
    reading: Option<String>,
    meaning: String,
    level: Option<String>,
    status: String,
}

#[derive(Serialize)]
pub struct WordPage {
    total: i64,
    items: Vec<WordRow>,
}

#[tauri::command]
pub fn list_words(state: State<AppState>, q: WordQuery) -> Result<WordPage> {
    let s = state.lock();
    let search = q.query.as_deref().map(str::trim).filter(|x| !x.is_empty()).map(|x| format!("%{x}%"));
    let filter = "FROM words w LEFT JOIN progress p ON p.word_id = w.id
                  WHERE w.deck_id = ?1 AND w.retired = 0
                    AND (?2 IS NULL OR w.level = ?2)
                    AND (?3 IS NULL OR w.term LIKE ?3 OR w.meaning LIKE ?3 OR w.reading LIKE ?3)";
    let total = s.conn.query_row(
        &format!("SELECT COUNT(*) {filter}"),
        params![q.deck_id, q.level, search],
        |r| r.get(0),
    )?;
    let mut stmt = s.conn.prepare(&format!(
        "SELECT w.id, w.term, w.meaning, w.level, COALESCE(p.status, 'new'), w.reading {filter}
         ORDER BY w.rank, w.id LIMIT ?4 OFFSET ?5"
    ))?;
    let items = stmt
        .query_map(params![q.deck_id, q.level, search, q.limit.clamp(1, 500), q.offset.max(0)], |r| {
            Ok(WordRow {
                id: r.get(0)?,
                term: r.get(1)?,
                reading: r.get(5)?,
                meaning: r.get(2)?,
                level: r.get(3)?,
                status: r.get(4)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(WordPage { total, items })
}

// ---------- 내 단어장 ----------

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct UserDeck {
    id: i64,
    name: String,
    enabled: bool,
    created_at: i64,
    total: i64,
    learning: i64,
    known: i64,
}

#[tauri::command]
pub fn list_user_decks(state: State<AppState>) -> Result<Vec<UserDeck>> {
    let s = state.lock();
    let mut stmt = s.conn.prepare(
        "SELECT d.id, d.name, d.enabled, d.created_at, COUNT(w.id),
                COALESCE(SUM(p.status = 'learning'), 0), COALESCE(SUM(p.status = 'known'), 0)
         FROM decks d
         LEFT JOIN words w ON w.deck_id = d.id
         LEFT JOIN progress p ON p.word_id = w.id
         WHERE d.kind = 'user'
         GROUP BY d.id ORDER BY d.created_at DESC, d.id DESC",
    )?;
    let decks = stmt
        .query_map([], |r| {
            Ok(UserDeck {
                id: r.get(0)?,
                name: r.get(1)?,
                enabled: r.get(2)?,
                created_at: r.get(3)?,
                total: r.get(4)?,
                learning: r.get(5)?,
                known: r.get(6)?,
            })
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    Ok(decks)
}

#[tauri::command]
pub fn set_deck_enabled(app: AppHandle, state: State<AppState>, id: i64, enabled: bool) -> Result<()> {
    state.lock().conn.execute(
        "UPDATE decks SET enabled = ?2 WHERE id = ?1 AND kind = 'user'",
        params![id, enabled],
    )?;
    engine::emit_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn delete_deck(app: AppHandle, state: State<AppState>, id: i64) -> Result<()> {
    {
        let mut s = state.lock();
        let n = s.conn.execute("DELETE FROM decks WHERE id = ?1 AND kind = 'user'", [id])?;
        if n == 0 {
            return Err(Error::msg("단어장을 찾지 못했어요"));
        }
        // 지금 떠 있는 카드가 지운 단어장 것이면 비워 둔다.
        let current_gone = match &s.current {
            Some(c) => s
                .conn
                .query_row("SELECT COUNT(*) FROM words WHERE id = ?1", [c.card.word_id], |r| r.get::<_, i64>(0))?
                == 0,
            None => false,
        };
        if current_gone {
            s.current = None;
        }
    }
    crate::popup::hide(&app)?;
    engine::emit_changed(&app);
    Ok(())
}

#[tauri::command]
pub fn preview_import(path: PathBuf) -> Result<Preview> {
    import::preview(&path)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportResult {
    deck_id: i64,
    count: usize,
}

/// 앱에 들어 있는 엑셀 양식을 사용자가 고른 위치에 복사한다.
#[tauri::command]
pub fn save_template(app: AppHandle, path: PathBuf) -> Result<()> {
    let src = app.path().resource_dir()?.join("template/word-template.xlsx");
    std::fs::copy(src, path)?;
    Ok(())
}

#[tauri::command]
pub fn commit_import(app: AppHandle, state: State<AppState>, path: PathBuf, name: String) -> Result<ImportResult> {
    let (deck_id, count) = {
        let mut s = state.lock();
        let language = s.settings.language.clone();
        import::commit(&mut s.conn, &path, &name, &language)?
    };
    engine::emit_changed(&app);
    Ok(ImportResult { deck_id, count })
}

// ---------- 시스템 ----------

#[tauri::command]
pub fn set_autostart(app: AppHandle, state: State<AppState>, enabled: bool) -> Result<bool> {
    let launcher = app.autolaunch();
    let r = if enabled { launcher.enable() } else { launcher.disable() };
    r.map_err(|e| Error::msg(format!("자동 실행 설정에 실패했어요: {e}")))?;
    let actual = launcher.is_enabled().unwrap_or(enabled);
    let mut s = state.lock();
    s.settings.autostart = actual;
    s.settings.save(&s.conn)?;
    Ok(actual)
}

#[tauri::command]
pub fn get_autostart(app: AppHandle) -> bool {
    app.autolaunch().is_enabled().unwrap_or(false)
}

#[tauri::command]
pub fn get_seed_reports(state: State<AppState>) -> Result<Vec<SeedReport>> {
    let s = state.lock();
    Ok(seed::LANGUAGES.iter().filter_map(|l| seed::load_report(&s.conn, l).ok().flatten()).collect())
}

#[tauri::command]
pub fn get_data_dir(app: AppHandle) -> Result<String> {
    Ok(app.path().app_data_dir()?.to_string_lossy().to_string())
}

// ---------- 팝업 ----------

#[tauri::command]
pub fn popup_current(state: State<AppState>) -> Option<PopupPayload> {
    state.lock().current.clone()
}

#[tauri::command]
pub fn popup_answer(app: AppHandle, word_id: i64, action: Answer) -> Result<()> {
    engine::answer(&app, word_id, action)
}
