//! 기본 단어장 동기화.
//!
//! `resources/words/<lang>/*.csv` (헤더: word,meaning,level) 를 읽어 로컬 DB에 반영한다.
//! 파일 내용 해시가 바뀐 경우에만 동작하고, `(deck, term_key)` 기준 upsert라서
//! 기존 단어의 id와 학습 기록(progress)은 그대로 유지된다.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::db;
use crate::error::Result;
use crate::validate::{self, DupTracker};

pub const LANGUAGES: [&str; 2] = ["en", "ja"];

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SkippedRow {
    pub file: String,
    pub row: usize,
    pub term: String,
    pub reason: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SeedReport {
    pub language: String,
    pub files: Vec<String>,
    pub accepted: usize,
    pub retired: usize,
    pub skipped: Vec<SkippedRow>,
    pub synced_at: i64,
}

struct Entry {
    term: String,
    reading: String,
    meaning: String,
    level: String,
}

pub fn builtin_deck_name(language: &str) -> &'static str {
    match language {
        "ja" => "기본 일본어",
        _ => "기본 영어",
    }
}

fn csv_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(read) = fs::read_dir(dir) else { return vec![] };
    let mut files: Vec<PathBuf> = read
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().is_some_and(|x| x.eq_ignore_ascii_case("csv")))
        .collect();
    files.sort();
    files
}

pub fn sync_all(conn: &mut Connection, words_dir: &Path) -> Result<()> {
    for lang in LANGUAGES {
        let report = sync_language(conn, lang, &words_dir.join(lang))?;
        if let Some(r) = report {
            log::info!(
                "기본 단어장({lang}) 동기화: 반영 {}개, 제외 {}개, 은퇴 {}개",
                r.accepted,
                r.skipped.len(),
                r.retired
            );
            for s in r.skipped.iter().take(20) {
                log::warn!("  {}:{} '{}' — {}", s.file, s.row, s.term, s.reason);
            }
        }
    }
    Ok(())
}

/// 해시가 같으면 아무것도 하지 않고 None.
pub fn sync_language(conn: &mut Connection, lang: &str, dir: &Path) -> Result<Option<SeedReport>> {
    let files = csv_files(dir);
    if files.is_empty() {
        // 파일이 하나도 없으면 패키징 실수일 수 있으니 기존 데이터를 건드리지 않는다.
        return Ok(None);
    }

    let mut hasher = Sha256::new();
    let mut contents = Vec::new();
    for f in &files {
        let bytes = fs::read(f)?;
        hasher.update(f.file_name().unwrap_or_default().as_encoded_bytes());
        hasher.update(&bytes);
        contents.push((f.file_name().unwrap_or_default().to_string_lossy().to_string(), bytes));
    }
    let hash = format!("{:x}", hasher.finalize());
    let hash_key = format!("seed_hash_{lang}");
    if db::get_meta(conn, &hash_key)?.as_deref() == Some(hash.as_str()) {
        return Ok(None);
    }

    let mut report = SeedReport {
        language: lang.into(),
        files: contents.iter().map(|(n, _)| n.clone()).collect(),
        synced_at: chrono::Utc::now().timestamp(),
        ..Default::default()
    };
    let entries = parse_all(lang, &contents, &mut report);
    report.accepted = entries.len();

    let now = chrono::Utc::now().timestamp();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT OR IGNORE INTO decks(kind, language, name, enabled, created_at)
         VALUES ('builtin', ?1, ?2, 1, ?3)",
        params![lang, builtin_deck_name(lang), now],
    )?;
    let deck_id: i64 = tx.query_row(
        "SELECT id FROM decks WHERE kind = 'builtin' AND language = ?1",
        [lang],
        |r| r.get(0),
    )?;
    tx.execute("UPDATE words SET retired = 1 WHERE deck_id = ?1", [deck_id])?;
    {
        let mut upsert = tx.prepare(
            "INSERT INTO words(deck_id, term, term_key, reading, meaning, level, rank, retired, updated_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, 0, ?8)
             ON CONFLICT(deck_id, term_key) DO UPDATE SET
               term = excluded.term, reading = excluded.reading, meaning = excluded.meaning, level = excluded.level,
               rank = excluded.rank, retired = 0, updated_at = excluded.updated_at",
        )?;
        for (i, e) in entries.iter().enumerate() {
            upsert.execute(params![
                deck_id,
                e.term,
                validate::builtin_key(&e.term, &e.reading),
                (!e.reading.is_empty()).then_some(&e.reading),
                e.meaning,
                e.level,
                i as i64,
                now
            ])?;
        }
    }
    report.retired = tx.query_row(
        "SELECT COUNT(*) FROM words WHERE deck_id = ?1 AND retired = 1",
        [deck_id],
        |r| r.get::<_, i64>(0),
    )? as usize;
    db::set_meta(&tx, &hash_key, &hash)?;
    db::set_meta(&tx, &format!("seed_report_{lang}"), &serde_json::to_string(&report)?)?;
    tx.commit()?;
    Ok(Some(report))
}

fn parse_all(lang: &str, contents: &[(String, Vec<u8>)], report: &mut SeedReport) -> Vec<Entry> {
    let mut dups = DupTracker::new();
    let mut out = Vec::new();
    let mut skip = |file: &str, row: usize, term: &str, reason: String| {
        report.skipped.push(SkippedRow { file: file.into(), row, term: term.into(), reason });
    };

    for (file, bytes) in contents {
        let text = String::from_utf8_lossy(bytes);
        let mut rdr = csv::ReaderBuilder::new().flexible(true).from_reader(text.as_bytes());
        let headers: Vec<String> = match rdr.headers() {
            Ok(h) => h.iter().map(|s| validate::clean(s).to_lowercase()).collect(),
            Err(e) => {
                skip(file, 1, "", format!("헤더를 읽지 못했어요: {e}"));
                continue;
            }
        };
        let col = |name: &str| headers.iter().position(|h| h == name);
        let (Some(c_word), Some(c_meaning), Some(c_level)) = (col("word"), col("meaning"), col("level")) else {
            skip(file, 1, "", "헤더는 word,meaning,level 이어야 해요 (일본어는 reading 열 추가)".into());
            continue;
        };
        let c_reading = col("reading");

        for (i, rec) in rdr.records().enumerate() {
            let row = i + 2; // 헤더가 1행
            let rec = match rec {
                Ok(r) => r,
                Err(e) => {
                    skip(file, row, "", format!("행을 읽지 못했어요: {e}"));
                    continue;
                }
            };
            let get = |c: usize| validate::clean(rec.get(c).unwrap_or(""));
            let (term, meaning, level) = (get(c_word), get(c_meaning), get(c_level));
            let reading = c_reading.map(get).unwrap_or_default();
            if term.is_empty() && meaning.is_empty() {
                continue;
            }
            if let Some(reason) = validate::check_builtin(lang, &term, &reading, &meaning, &level) {
                skip(file, row, &term, reason);
                continue;
            }
            if let Some(first) = dups.check(&validate::builtin_key(&term, &reading), row) {
                skip(file, row, &term, format!("중복 단어 (처음 나온 행: {first})"));
                continue;
            }
            out.push(Entry { term, reading, meaning, level });
        }
    }
    out
}

pub fn load_report(conn: &Connection, lang: &str) -> Result<Option<SeedReport>> {
    Ok(db::get_meta(conn, &format!("seed_report_{lang}"))?
        .and_then(|s| serde_json::from_str(&s).ok()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, body: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join(name), body).unwrap();
    }

    /// 앱에 실제로 들어가는 기본 단어 파일이 검증을 모두 통과하는지.
    #[test]
    fn bundled_word_files_are_clean() {
        let words = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/words");
        let mut conn = db::open_in_memory().unwrap();
        for (lang, expected) in [("en", 48), ("ja", 36)] {
            let r = sync_language(&mut conn, lang, &words.join(lang)).unwrap().unwrap();
            assert!(r.skipped.is_empty(), "{lang}: {:?}", r.skipped);
            assert_eq!(r.accepted, expected, "{lang}");
        }
        let reading: String = conn
            .query_row("SELECT reading FROM words WHERE term = '間に合う'", [], |r| r.get(0))
            .unwrap();
        assert_eq!(reading, "まにあう");
        let en_reading: Option<String> =
            conn.query_row("SELECT reading FROM words WHERE term = 'apple'", [], |r| r.get(0)).unwrap();
        assert_eq!(en_reading, None);
    }

    #[test]
    fn resync_keeps_progress_and_retires_missing() {
        let tmp = std::env::temp_dir().join(format!("ppok-seed-{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        let en = tmp.join("en");
        write(&en, "a.csv", "word,meaning,level\napple,사과,basic\nbanana,바나나,basic\nApple,사과,basic\n,,\n");

        let mut conn = db::open_in_memory().unwrap();
        let r = sync_language(&mut conn, "en", &en).unwrap().unwrap();
        assert_eq!(r.accepted, 2);
        assert_eq!(r.skipped.len(), 1); // Apple 중복

        let apple_id: i64 =
            conn.query_row("SELECT id FROM words WHERE term_key='apple'", [], |r| r.get(0)).unwrap();
        conn.execute("INSERT INTO progress(word_id, status, box) VALUES (?1, 'known', 3)", [apple_id])
            .unwrap();

        // 해시가 같으면 다시 돌지 않는다.
        assert!(sync_language(&mut conn, "en", &en).unwrap().is_none());

        // 뜻 수정 + banana 삭제 + cherry 추가
        write(&en, "a.csv", "word,meaning,level\napple,사과; 사과나무,basic\ncherry,체리,basic\n");
        let r = sync_language(&mut conn, "en", &en).unwrap().unwrap();
        assert_eq!(r.accepted, 2);
        assert_eq!(r.retired, 1);

        let (id, meaning): (i64, String) = conn
            .query_row("SELECT id, meaning FROM words WHERE term_key='apple'", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(id, apple_id);
        assert_eq!(meaning, "사과; 사과나무");
        let status: String =
            conn.query_row("SELECT status FROM progress WHERE word_id=?1", [id], |r| r.get(0)).unwrap();
        assert_eq!(status, "known");
        let _ = fs::remove_dir_all(&tmp);
    }
}
