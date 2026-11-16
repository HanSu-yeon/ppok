//! 외웠어 / 다시 볼래 처리와 다음 단어 선택.
//!
//! '외웠어'를 누른 단어는 다시 나오지 않는다. 설정에서 복습을 켜면 가끔 섞여 나온다.
//! '다시 볼래'를 누른 단어는 시간이 아니라 그 뒤에 본 단어 수로 돌아온다. 팝업 간격을
//! 1분으로 두든 30분으로 두든 그 사이에 다른 단어를 충분히 보고 와야
//! "방금 봐서 아는 느낌"이 안 남는다.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::error::Result;
use crate::settings::{Settings, Source};

/// 복습을 켜 두면 팝업 이만큼에 1번은 외운 단어를 보여준다.
pub const REVIEW_EVERY: i64 = 10;

/// 지금까지 팝업으로 보여준 단어 수. '다시 볼래' 단어가 돌아올 차례를 이 값으로 잰다.
const SHOWN_SEQ_KEY: &str = "shown_seq";

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
    /// "new" | "learning" | "review"(외운 단어 복습)
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

fn shown_seq(conn: &Connection) -> Result<i64> {
    Ok(conn.query_row(
        "SELECT COALESCE((SELECT CAST(value AS INTEGER) FROM meta WHERE key = ?1), 0)",
        [SHOWN_SEQ_KEY],
        |r| r.get(0),
    )?)
}

pub fn pool_counts(conn: &Connection, s: &Settings) -> Result<PoolCounts> {
    let sql = format!(
        "SELECT COUNT(*),
                COALESCE(SUM(p.status = 'learning'), 0),
                COALESCE(SUM(p.status = 'known'), 0),
                COALESCE(SUM(p.status = 'learning' AND p.due_seq <= ?3), 0)
         {}",
        pool_from(s.source)
    );
    Ok(conn.query_row(&sql, params![s.language, s.level, shown_seq(conn)?], |r| {
        Ok(PoolCounts { total: r.get(0)?, learning: r.get(1)?, known: r.get(2)?, due: r.get(3)? })
    })?)
}

/// 1) 돌아올 차례가 된 '다시 볼래' 단어  2) 새 단어(노출 적은 순 → 빈도순)
/// 3) 둘 다 없으면 가장 먼저 돌아올 '다시 볼래' 단어.
///
/// '외웠어' 단어는 복습을 켰을 때만 나온다. `REVIEW_EVERY`번째 팝업마다, 그리고 더 보여줄
/// 단어가 없을 때 오래 안 본 단어부터.
///
/// `exclude`는 직전에 보여준 단어 (연속 출제 방지).
pub fn next_card(conn: &Connection, s: &Settings, exclude: Option<i64>) -> Result<Option<Card>> {
    let from = pool_from(s.source);
    let ex = exclude.unwrap_or(-1);
    let seq = shown_seq(conn)?;
    let select = "SELECT w.id, w.term, w.meaning, d.name, COALESCE(p.status, 'new'), w.reading";
    let review = format!(
        "{select} {from} AND p.status = 'known' AND w.id != ?4
         ORDER BY COALESCE(p.last_seen_at, 0), w.rank LIMIT 1"
    );
    let review_turn = s.review_known && (seq + 1) % REVIEW_EVERY == 0;
    let mut queries: Vec<String> = Vec::with_capacity(5);
    if review_turn {
        queries.push(review.clone());
    }
    queries.extend([
        format!(
            "{select} {from} AND p.status = 'learning' AND p.due_seq <= ?3 AND w.id != ?4
             ORDER BY p.due_seq LIMIT 1"
        ),
        format!(
            "{select} {from} AND COALESCE(p.status, 'new') = 'new' AND w.id != ?4
             ORDER BY COALESCE(p.seen_count, 0), w.rank, w.id LIMIT 1"
        ),
        format!(
            "{select} {from} AND COALESCE(p.status, 'new') != 'known' AND w.id != ?4
             ORDER BY COALESCE(p.due_seq, 0), w.rank LIMIT 1"
        ),
    ]);
    if s.review_known && !review_turn {
        queries.push(review);
    }
    for sql in &queries {
        let card = conn
            .query_row(sql, params![s.language, s.level, seq, ex], |r| {
                let status: String = r.get(4)?;
                Ok(Card {
                    word_id: r.get(0)?,
                    term: r.get(1)?,
                    reading: r.get(5)?,
                    meaning: r.get(2)?,
                    deck_name: r.get(3)?,
                    kind: match status.as_str() {
                        "new" => "new",
                        "learning" => "learning",
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
    conn.execute(
        "INSERT INTO meta(key, value) VALUES (?1, '1')
         ON CONFLICT(key) DO UPDATE SET value = CAST(value AS INTEGER) + 1",
        [SHOWN_SEQ_KEY],
    )?;
    log(conn, word_id, "shown", now)
}

pub fn answer(conn: &Connection, s: &Settings, word_id: i64, answer: Answer, now: i64) -> Result<()> {
    match answer {
        Answer::Known => {
            conn.execute(
                "INSERT INTO progress(word_id, status, box, due_at, due_seq) VALUES (?1, 'known', 1, NULL, NULL)
                 ON CONFLICT(word_id) DO UPDATE SET status = 'known', box = box + 1, due_at = NULL, due_seq = NULL",
                [word_id],
            )?;
        }
        Answer::Again => {
            // 아직 안 외운 단어(이 단어 포함)가 적으면 그 수만큼만 기다린다. 즉 한 바퀴.
            let active: i64 = conn.query_row(
                &format!("SELECT COUNT(*) {} AND COALESCE(p.status, 'new') != 'known'", pool_from(s.source)),
                params![s.language, s.level],
                |r| r.get(0),
            )?;
            let lap = active - 1;
            let gap = match s.again_after {
                0 => lap,
                n => (n as i64).min(lap),
            }
            .max(1);
            conn.execute(
                "INSERT INTO progress(word_id, status, box, due_seq) VALUES (?1, 'learning', 0, ?2)
                 ON CONFLICT(word_id) DO UPDATE SET status = 'learning', box = 0, due_seq = ?2",
                params![word_id, shown_seq(conn)? + gap],
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

    fn add_words(conn: &Connection, n: usize) {
        for i in 0..n {
            let t = format!("w{i:03}");
            conn.execute(
                "INSERT INTO words(deck_id, term, term_key, meaning, level, rank, updated_at)
                 VALUES (1, ?1, ?1, '뜻', 'basic', ?2, 0)",
                params![t, 10 + i as i64],
            )
            .unwrap();
        }
    }

    /// 다음 단어를 골라 보여준 것으로 기록한다. 엔진이 하는 일과 같다.
    fn show(conn: &Connection, s: &Settings, last: Option<i64>) -> Card {
        let c = next_card(conn, s, last).unwrap().unwrap();
        mark_shown(conn, c.word_id, 0).unwrap();
        c
    }

    #[test]
    fn again_comes_back_after_one_lap_in_small_deck() {
        let (conn, s) = setup();
        let apple = id(&conn, "apple");
        assert_eq!(show(&conn, &s, None).term, "apple");
        answer(&conn, &s, apple, Answer::Again, 0).unwrap();

        // 나머지 두 단어를 다 본 뒤에야 돌아온다.
        let b = show(&conn, &s, Some(apple));
        assert_eq!(b.term, "book");
        let c = show(&conn, &s, Some(b.word_id));
        assert_eq!(c.term, "cat");
        let back = next_card(&conn, &s, Some(c.word_id)).unwrap().unwrap();
        assert_eq!((back.term.as_str(), back.kind.as_str()), ("apple", "learning"));
    }

    /// '다시 볼래'를 누른 단어가 다른 단어를 정확히 `gap`개 본 뒤에 돌아오는지 확인한다.
    fn assert_again_gap(s: &Settings, extra_words: usize, gap: i64) {
        let (conn, _) = setup();
        add_words(&conn, extra_words);
        let apple = id(&conn, "apple");
        assert_eq!(show(&conn, s, None).term, "apple");
        answer(&conn, s, apple, Answer::Again, 0).unwrap();

        let mut last = Some(apple);
        for _ in 0..gap - 1 {
            let c = show(&conn, s, last);
            assert_ne!(c.word_id, apple);
            last = Some(c.word_id);
        }
        assert_eq!(pool_counts(&conn, s).unwrap().due, 0);
        let c = show(&conn, s, last);
        assert_ne!(c.word_id, apple);
        assert_eq!(pool_counts(&conn, s).unwrap().due, 1);
        assert_eq!(next_card(&conn, s, Some(c.word_id)).unwrap().unwrap().word_id, apple);
    }

    #[test]
    fn again_waits_the_chosen_number_of_cards() {
        let s = Settings::default();
        assert_eq!(s.again_after, 100);
        assert_again_gap(&s, 200, 100);
        assert_again_gap(&Settings { again_after: 20, ..Settings::default() }, 200, 20);
        // 한 바퀴: 단어 203개 중 자기 자신을 뺀 202개를 본 뒤
        assert_again_gap(&Settings { again_after: 0, ..Settings::default() }, 200, 202);
    }

    #[test]
    fn review_mixes_known_words_in_every_tenth_popup() {
        let (conn, _) = setup();
        add_words(&conn, 30);
        let s = Settings { review_known: true, ..Settings::default() };
        let apple = id(&conn, "apple");
        answer(&conn, &s, apple, Answer::Known, 0).unwrap();

        let mut last = None;
        for i in 1..=20 {
            let c = show(&conn, &s, last);
            if i % REVIEW_EVERY == 0 {
                assert_eq!((c.word_id, c.kind.as_str()), (apple, "review"), "{i}번째 팝업");
            } else {
                assert_ne!(c.word_id, apple, "{i}번째 팝업");
            }
            last = Some(c.word_id);
        }

        // 복습을 끄면 외운 단어는 나오지 않는다.
        let off = Settings::default();
        for _ in 0..20 {
            let c = show(&conn, &off, last);
            assert_ne!(c.word_id, apple);
            last = Some(c.word_id);
        }
    }

    #[test]
    fn review_keeps_going_when_everything_is_known() {
        let (conn, _) = setup();
        let s = Settings { review_known: true, ..Settings::default() };
        for term in ["apple", "book", "cat"] {
            answer(&conn, &s, id(&conn, term), Answer::Known, 0).unwrap();
        }
        let c = next_card(&conn, &s, None).unwrap().unwrap();
        assert_eq!(c.kind, "review");
    }

    #[test]
    fn known_never_comes_back_and_timeout_is_not_learning() {
        let (conn, s) = setup();
        let book = id(&conn, "book");
        mark_shown(&conn, book, 0).unwrap();
        answer(&conn, &s, book, Answer::Timeout, 0).unwrap();
        let st: String =
            conn.query_row("SELECT status FROM progress WHERE word_id=?1", [book], |r| r.get(0)).unwrap();
        assert_eq!(st, "new");

        for term in ["apple", "book", "cat"] {
            answer(&conn, &s, id(&conn, term), Answer::Known, 0).unwrap();
        }
        // 새 단어도 '다시 볼래' 단어도 없으면 외운 단어로 돌아가지 않고 끝난다.
        assert!(next_card(&conn, &s, None).unwrap().is_none());

        let st = day_stats(&conn, 0).unwrap();
        assert_eq!((st.seen, st.known, st.again), (1, 3, 0));
    }

    #[test]
    fn learning_rows_from_old_schema_are_due_right_away() {
        let (conn, s) = setup();
        let cat = id(&conn, "cat");
        conn.execute("INSERT INTO progress(word_id, status, box, due_seq) VALUES (?1, 'learning', 0, 0)", [cat])
            .unwrap();
        assert_eq!(next_card(&conn, &s, None).unwrap().unwrap().term, "cat");
    }
}
