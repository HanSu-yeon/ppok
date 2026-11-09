use std::path::Path;

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::Result;

const SCHEMA_VERSION: i64 = 2;

pub fn open(path: &Path) -> Result<Connection> {
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "journal_mode", "WAL")?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    conn.pragma_update(None, "synchronous", "NORMAL")?;
    migrate(&conn)?;
    Ok(conn)
}

#[cfg(test)]
pub fn open_in_memory() -> Result<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", "ON")?;
    migrate(&conn)?;
    Ok(conn)
}

fn migrate(conn: &Connection) -> Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    if version < 1 {
        conn.execute_batch(
            r#"
            CREATE TABLE IF NOT EXISTS decks (
              id          INTEGER PRIMARY KEY,
              kind        TEXT NOT NULL CHECK (kind IN ('builtin','user')),
              language    TEXT NOT NULL,
              name        TEXT NOT NULL,
              enabled     INTEGER NOT NULL DEFAULT 1,
              created_at  INTEGER NOT NULL
            );
            CREATE UNIQUE INDEX IF NOT EXISTS decks_builtin_lang
              ON decks(language) WHERE kind = 'builtin';

            CREATE TABLE IF NOT EXISTS words (
              id          INTEGER PRIMARY KEY,
              deck_id     INTEGER NOT NULL REFERENCES decks(id) ON DELETE CASCADE,
              term        TEXT NOT NULL,
              term_key    TEXT NOT NULL,
              meaning     TEXT NOT NULL,
              level       TEXT,
              rank        INTEGER NOT NULL DEFAULT 0,
              retired     INTEGER NOT NULL DEFAULT 0,
              updated_at  INTEGER NOT NULL,
              UNIQUE (deck_id, term_key)
            );
            CREATE INDEX IF NOT EXISTS words_deck_level ON words(deck_id, level, retired);

            -- 학습 기록은 단어 데이터와 분리해 둔다. 기본 단어 갱신은 words만 건드린다.
            CREATE TABLE IF NOT EXISTS progress (
              word_id      INTEGER PRIMARY KEY REFERENCES words(id) ON DELETE CASCADE,
              status       TEXT NOT NULL DEFAULT 'new' CHECK (status IN ('new','learning','known')),
              box          INTEGER NOT NULL DEFAULT 0,
              due_at       INTEGER,
              seen_count   INTEGER NOT NULL DEFAULT 0,
              last_seen_at INTEGER
            );
            CREATE INDEX IF NOT EXISTS progress_due ON progress(status, due_at);

            CREATE TABLE IF NOT EXISTS review_log (
              id       INTEGER PRIMARY KEY,
              word_id  INTEGER NOT NULL REFERENCES words(id) ON DELETE CASCADE,
              action   TEXT NOT NULL,
              at       INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS review_log_at ON review_log(at, action);

            CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS meta     (key TEXT PRIMARY KEY, value TEXT NOT NULL);
            "#,
        )?;
    }
    if version < 2 {
        // 일본어 등 읽기(후리가나)가 필요한 언어용. 영어와 내 단어장은 NULL.
        conn.execute_batch("ALTER TABLE words ADD COLUMN reading TEXT;")?;
    }
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM meta WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

pub fn get_setting(conn: &Connection, key: &str) -> Result<Option<String>> {
    Ok(conn
        .query_row("SELECT value FROM settings WHERE key = ?1", [key], |r| r.get(0))
        .optional()?)
}

pub fn set_setting(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO settings(key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}
