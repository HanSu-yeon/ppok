//! 간격 반복(라이트너 박스)과 다음 단어 선택.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::settings::{Settings, Source};

/// '외웠어'를 누를 때마다 박스가 올라가며 다음 복습까지의 간격(일).
const KNOWN_INTERVAL_DAYS: [i64; 6] = [1, 3, 7, 14, 30, 60];
/// '다시 볼래' 후 다시 출제 대상이 되기까지(초).
pub const AGAIN_DELAY_SECS: i64 = 20 * 60;

const DAY: i64 = 86_400;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Answer {
    Known,
    Again,
    /// 닫기 버튼
    Dismiss,
    /// 자동으로 사라짐
    Timeout,
}

impl Answer {
    fn as_str(self) -> &'static str {
        match self {
            Answer::Known => "known",
            Answer::Again => "again",
            Answer::Dismiss => "dismiss",
            Answer::Timeout => "timeout",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Card {
    pub word_id: i64,
    pub term: String,
    /// 일본어 읽기(후리가나). 없으면 None.
    pub reading: Option<String>,
    pub meaning: String,
    pub deck_name: String,
    /// "review" | "learning" | "new"
    pub kind: String,
}

/// 현재 설정으로 출제 가능한 단어 집합. ?1 = language, ?2 = level.
/// 내 단어장은 언어 설정과 무관하게 포함한다. 두 파라미터가 항상 쓰이도록 플래그를 리터럴로 넣는다.
fn pool_from(source: Source) -> String {
    let (builtin_on, user_on) = match source {
        Source::Builtin => (1, 0),
        Source::User => (0, 1),
    };
    format!(
        "FROM words w
         JOIN decks d ON d.id = w.deck_id
         LEFT JOIN progress p ON p.word_id = w.id
         WHERE d.enabled = 1 AND w.retired = 0
           AND ((d.kind = 'builtin' AND d.language = ?1 AND w.level = ?2 AND {builtin_on} = 1)
             OR (d.kind = 'user' AND {user_on} = 1))"
    )
}

#[derive(Debug, Clone, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PoolCounts {
    pub total: i64,
    pub learning: i64,
    pub known: i64,
    pub due: i64,
}

pub fn pool_counts(conn: &Connection, s: &Settings, now: i64) -> Result<PoolCounts> {
    let sql = format!(
        "SELECT COUNT(*),
                COALESCE(SUM(p.status = 'learning'), 0),
                COALESCE(SUM(p.status = 'known'), 0),
                COALESCE(SUM(p.status IN ('learning','known') AND p.due_at <= ?3), 0)
         {}",
        pool_from(s.source)
    );
    Ok(conn.query_row(&sql, params![s.language, s.level, now], |r| {
        Ok(PoolCounts { total: r.get(0)?, learning: r.get(1)?, known: r.get(2)?, due: r.get(3)? })
    })?)
}

/// 1) 복습 예정  2) 새 단어(노출 적은 순 → 빈도순)  3) 가장 빨리 돌아올 단어.
///
/// `exclude`는 직전에 보여준 단어 (연속 출제 방지).
pub fn next_card(conn: &Connection, s: &Settings, now: i64, exclude: Option<i64>) -> Result<Option<Card>> {
    let from = pool_from(s.source);
    let ex = exclude.unwrap_or(-1);
    let select = "SELECT w.id, w.term, w.meaning, d.name, COALESCE(p.status, 'new'), w.reading";
    let queries = [
        format!(
            "{select} {from} AND p.status IN ('learning','known') AND p.due_at <= ?3 AND w.id != ?4
             ORDER BY p.due_at LIMIT 1"
        ),
        format!(
            "{select} {from} AND COALESCE(p.status, 'new') = 'new' AND w.id != ?4
             ORDER BY COALESCE(p.seen_count, 0), w.rank, w.id LIMIT 1"
        ),
        format!(
            "{select} {from} AND w.id != ?4
             ORDER BY COALESCE(p.due_at, 0), w.rank LIMIT 1"
        ),
    ];
    for (i, sql) in queries.iter().enumerate() {
        let card = conn
            .query_row(sql, params![s.language, s.level, now, ex], |r| {
                let status: String = r.get(4)?;
                Ok(Card {
                    word_id: r.get(0)?,
                    term: r.get(1)?,
                    reading: r.get(5)?,
                    meaning: r.get(2)?,
                    deck_name: r.get(3)?,
                    kind: match (i, status.as_str()) {
                        (_, "new") => "new",
                        (_, "learning") => "learning",
                        _ => "review",
                    }
                    .into(),
                })
            })
            .optional()?;
        if card.is_some() {
            return Ok(card);
        }
    }
    Ok(None)
}

fn log(conn: &Connection, word_id: i64, action: &str, now: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO review_log(word_id, action, at) VALUES (?1, ?2, ?3)",
        params![word_id, action, now],
    )?;
    Ok(())
}

pub fn mark_shown(conn: &Connection, word_id: i64, now: i64) -> Result<()> {
    conn.execute(
        "INSERT INTO progress(word_id, status, box, seen_count, last_seen_at) VALUES (?1, 'new', 0, 1, ?2)
         ON CONFLICT(word_id) DO UPDATE SET seen_count = seen_count + 1, last_seen_at = excluded.last_seen_at",
        params![word_id, now],
    )?;
    log(conn, word_id, "shown", now)
}

pub fn answer(conn: &Connection, word_id: i64, answer: Answer, now: i64) -> Result<()> {
    match answer {
        Answer::Known => {
            let current: i64 = conn
                .query_row("SELECT box FROM progress WHERE word_id = ?1", [word_id], |r| r.get(0))
                .optional()?
                .unwrap_or(0);
            let next_box = (current + 1).min(KNOWN_INTERVAL_DAYS.len() as i64);
            let due = now + KNOWN_INTERVAL_DAYS[(next_box - 1) as usize] * DAY;
            conn.execute(
                "INSERT INTO progress(word_id, status, box, due_at) VALUES (?1, 'known', ?2, ?3)
                 ON CONFLICT(word_id) DO UPDATE SET status = 'known', box = ?2, due_at = ?3",
                params![word_id, next_box, due],
            )?;
        }
        Answer::Again => {
            conn.execute(
                "INSERT INTO progress(word_id, status, box, due_at) VALUES (?1, 'learning', 0, ?2)
                 ON CONFLICT(word_id) DO UPDATE SET status = 'learning', box = 0, due_at = ?2",
                params![word_id, now + AGAIN_DELAY_SECS],
            )?;
        }
        // 닫거나 자동으로 사라진 단어는 학습 완료로 보지 않는다. 기록만 남긴다.
        Answer::Dismiss | Answer::Timeout => {}
    }
    log(conn, word_id, answer.as_str(), now)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DayStats {
    pub seen: i64,
    pub known: i64,
    pub again: i64,
}

pub fn day_stats(conn: &Connection, since: i64) -> Result<DayStats> {
    let count = |action: &str| -> Result<i64> {
        Ok(conn.query_row(
            "SELECT COUNT(DISTINCT word_id) FROM review_log WHERE action = ?1 AND at >= ?2",
            params![action, since],
            |r| r.get(0),
        )?)
    };
    Ok(DayStats { seen: count("shown")?, known: count("known")?, again: count("again")? })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db;

    fn setup() -> (Connection, Settings) {
        let conn = db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO decks(id, kind, language, name, enabled, created_at) VALUES (1,'builtin','en','기본 영어',1,0)",
            [],
        )
        .unwrap();
        for (i, (t, m)) in [("apple", "사과"), ("book", "책"), ("cat", "고양이")].iter().enumerate() {
            conn.execute(
                "INSERT INTO words(deck_id, term, term_key, meaning, level, rank, updated_at)
                 VALUES (1, ?1, ?1, ?2, 'basic', ?3, 0)",
                params![t, m, i as i64],
            )
            .unwrap();
        }
        (conn, Settings::default())
    }

    fn id(conn: &Connection, term: &str) -> i64 {
        conn.query_row("SELECT id FROM words WHERE term = ?1", [term], |r| r.get(0)).unwrap()
    }

    #[test]
    fn new_words_in_rank_order_and_due_first() {
        let (conn, s) = setup();
        let now = 1_000_000;
        assert_eq!(next_card(&conn, &s, now, None).unwrap().unwrap().term, "apple");

        let apple = id(&conn, "apple");
        mark_shown(&conn, apple, now).unwrap();
        answer(&conn, apple, Answer::Again, now).unwrap();
        // 아직 20분이 안 지났으니 새 단어
        assert_eq!(next_card(&conn, &s, now + 60, Some(apple)).unwrap().unwrap().term, "book");
        // 20분 뒤에는 복습 예정 단어가 우선
        let c = next_card(&conn, &s, now + AGAIN_DELAY_SECS, None).unwrap().unwrap();
        assert_eq!((c.term.as_str(), c.kind.as_str()), ("apple", "learning"));
    }

    #[test]
    fn known_grows_interval_and_timeout_is_not_learning() {
        let (conn, _) = setup();
        let now = 1_000_000;
        let book = id(&conn, "book");
        mark_shown(&conn, book, now).unwrap();
        answer(&conn, book, Answer::Timeout, now).unwrap();
        let st: String =
            conn.query_row("SELECT status FROM progress WHERE word_id=?1", [book], |r| r.get(0)).unwrap();
        assert_eq!(st, "new");

        answer(&conn, book, Answer::Known, now).unwrap();
        answer(&conn, book, Answer::Known, now).unwrap();
        let (b, due): (i64, i64) = conn
            .query_row("SELECT box, due_at FROM progress WHERE word_id=?1", [book], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(b, 2);
        assert_eq!(due, now + 3 * DAY);

        let st = day_stats(&conn, 0).unwrap();
        assert_eq!((st.seen, st.known, st.again), (1, 1, 0));
    }
}
