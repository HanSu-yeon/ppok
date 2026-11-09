//! 내 단어장 업로드: .xlsx / .xls / .csv 를 (단어, 뜻) 두 열로 읽는다.

use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};
use rusqlite::{Connection, params};
use serde::Serialize;

use crate::error::{Error, Result};
use crate::validate::{self, DupTracker, RowStatus};

const MAX_ROWS: usize = 20_000;
const HEADER_WORDS: [&str; 6] = ["단어", "영단어", "word", "words", "term", "vocabulary"];
const HEADER_MEANINGS: [&str; 5] = ["뜻", "의미", "meaning", "meanings", "definition"];

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PreviewRow {
    pub row: usize,
    pub term: String,
    pub meaning: String,
    pub status: RowStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub file_name: String,
    pub suggested_name: String,
    pub rows: Vec<PreviewRow>,
    pub ok_count: usize,
    pub duplicate_count: usize,
    pub error_count: usize,
    pub skipped_blank: usize,
}

/// 파일을 (원래 행 번호, A열, B열) 목록으로 읽는다.
fn read_cells(path: &Path) -> Result<Vec<(usize, String, String)>> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("").to_ascii_lowercase();
    match ext.as_str() {
        "csv" | "tsv" | "txt" => read_csv(path),
        "xlsx" | "xlsm" | "xls" | "ods" => read_sheet(path),
        _ => Err(Error::msg("엑셀(.xlsx) 또는 CSV(.csv) 파일만 올릴 수 있어요")),
    }
}

fn read_sheet(path: &Path) -> Result<Vec<(usize, String, String)>> {
    let mut wb = open_workbook_auto(path).map_err(|e| Error::Excel(e.to_string()))?;
    let range = wb
        .worksheet_range_at(0)
        .ok_or_else(|| Error::msg("시트가 비어 있어요"))?
        .map_err(|e| Error::Excel(e.to_string()))?;
    let start_row = range.start().map(|(r, _)| r as usize).unwrap_or(0);
    let cell = |c: Option<&Data>| c.map(|d| d.to_string()).unwrap_or_default();
    Ok(range
        .rows()
        .enumerate()
        .map(|(i, r)| (start_row + i + 1, cell(r.first()), cell(r.get(1))))
        .collect())
}

/// 한국어 엑셀에서 저장한 CSV는 CP949인 경우가 많아 UTF-8이 아니면 CP949로 읽는다.
pub fn decode_text(bytes: &[u8]) -> String {
    let bytes = bytes.strip_prefix(b"\xEF\xBB\xBF").unwrap_or(bytes);
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => encoding_rs::EUC_KR.decode(bytes).0.into_owned(),
    }
}

fn read_csv(path: &Path) -> Result<Vec<(usize, String, String)>> {
    let text = decode_text(&std::fs::read(path)?);
    let first_line = text.lines().next().unwrap_or("");
    let delimiter = if first_line.contains('\t') && !first_line.contains(',') { b'\t' } else { b',' };
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(false)
        .flexible(true)
        .delimiter(delimiter)
        .from_reader(text.as_bytes());
    // csv 크레이트의 position().line()은 헤더 없이 읽을 때 첫 줄 번호가 어긋나서 직접 센다.
    // (셀 안 줄바꿈이 있으면 엑셀 행 번호와 달라질 수 있지만 단어장에서는 드물다)
    let mut out = Vec::new();
    for (i, rec) in rdr.records().enumerate() {
        let rec = rec?;
        out.push((i + 1, rec.get(0).unwrap_or("").to_string(), rec.get(1).unwrap_or("").to_string()));
    }
    Ok(out)
}

fn is_header(a: &str, b: &str) -> bool {
    let (a, b) = (a.to_lowercase(), b.to_lowercase());
    HEADER_WORDS.contains(&a.as_str()) || HEADER_MEANINGS.contains(&b.as_str())
}

pub fn preview(path: &Path) -> Result<Preview> {
    let cells = read_cells(path)?;
    Ok(build_preview(path, cells))
}

fn build_preview(path: &Path, cells: Vec<(usize, String, String)>) -> Preview {
    let file_name = path.file_name().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
    let suggested_name = path
        .file_stem()
        .map(|s| s.to_string_lossy().to_string())
        .unwrap_or_else(|| "내 단어장".into());

    let mut rows = Vec::new();
    let mut dups = DupTracker::new();
    let mut skipped_blank = 0;
    let mut header_checked = false;

    for (row, a, b) in cells {
        let (term, meaning) = (validate::clean(&a), validate::clean(&b));
        if term.is_empty() && meaning.is_empty() {
            skipped_blank += 1;
            continue;
        }
        if !header_checked {
            header_checked = true;
            if is_header(&term, &meaning) {
                continue;
            }
        }
        let (status, message) = if let Some(reason) = validate::check_entry(&term, &meaning) {
            (RowStatus::Error, Some(reason))
        } else if let Some(first) = dups.check(&term, row) {
            (RowStatus::Duplicate, Some(format!("{first}행과 같은 단어예요")))
        } else {
            (RowStatus::Ok, None)
        };
        rows.push(PreviewRow { row, term, meaning, status, message });
    }

    let count = |s: RowStatus| rows.iter().filter(|r| r.status == s).count();
    Preview {
        file_name,
        suggested_name,
        ok_count: count(RowStatus::Ok),
        duplicate_count: count(RowStatus::Duplicate),
        error_count: count(RowStatus::Error),
        skipped_blank,
        rows,
    }
}

/// 미리보기와 같은 규칙으로 다시 읽어 정상 행만 새 단어장으로 저장한다.
pub fn commit(conn: &mut Connection, path: &Path, name: &str, language: &str) -> Result<(i64, usize)> {
    let name = validate::clean(name);
    if name.is_empty() {
        return Err(Error::msg("단어장 이름을 입력해 주세요"));
    }
    if name.chars().count() > 40 {
        return Err(Error::msg("단어장 이름은 40자 이하로 해 주세요"));
    }
    let exists: bool = conn.query_row(
        "SELECT EXISTS(SELECT 1 FROM decks WHERE kind = 'user' AND name = ?1)",
        [&name],
        |r| r.get(0),
    )?;
    if exists {
        return Err(Error::msg("같은 이름의 단어장이 이미 있어요"));
    }

    let preview = preview(path)?;
    let ok: Vec<&PreviewRow> = preview.rows.iter().filter(|r| r.status == RowStatus::Ok).collect();
    if ok.is_empty() {
        return Err(Error::msg("저장할 수 있는 단어가 없어요"));
    }
    if ok.len() > MAX_ROWS {
        return Err(Error::msg(format!("한 번에 {MAX_ROWS}개까지 올릴 수 있어요")));
    }

    let now = chrono::Utc::now().timestamp();
    let tx = conn.transaction()?;
    tx.execute(
        "INSERT INTO decks(kind, language, name, enabled, created_at) VALUES ('user', ?1, ?2, 1, ?3)",
        params![language, name, now],
    )?;
    let deck_id = tx.last_insert_rowid();
    {
        let mut ins = tx.prepare(
            "INSERT INTO words(deck_id, term, term_key, meaning, level, rank, retired, updated_at)
             VALUES (?1, ?2, ?3, ?4, NULL, ?5, 0, ?6)",
        )?;
        for (i, r) in ok.iter().enumerate() {
            ins.execute(params![deck_id, r.term, validate::term_key(&r.term), r.meaning, i as i64, now])?;
        }
    }
    tx.commit()?;
    Ok((deck_id, ok.len()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cells(rows: &[(&str, &str)]) -> Vec<(usize, String, String)> {
        rows.iter().enumerate().map(|(i, (a, b))| (i + 1, a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn preview_marks_header_blank_dup_error() {
        let p = build_preview(
            Path::new("업무 단어.xlsx"),
            cells(&[
                ("단어", "뜻"),
                ("apple", "사과"),
                ("", ""),
                ("postpone", "연기하다"),
                ("Apple", "사과"),
                ("efficient", ""),
                ("효율적인", "efficient"),
            ]),
        );
        assert_eq!(p.suggested_name, "업무 단어");
        assert_eq!((p.ok_count, p.duplicate_count, p.error_count, p.skipped_blank), (2, 1, 2, 1));
        assert_eq!(p.rows[2].message.as_deref(), Some("2행과 같은 단어예요"));
    }

    #[test]
    fn cp949_csv_is_decoded() {
        let (bytes, _, _) = encoding_rs::EUC_KR.encode("apple,사과\n");
        assert_eq!(decode_text(&bytes), "apple,사과\n");
        assert_eq!(decode_text("\u{feff}a,b".as_bytes()), "a,b");
    }
}

#[cfg(test)]
mod sample_files {
    use super::*;

    /// docs/samples 의 실제 파일로 엑셀·CP949 CSV 경로를 확인한다.
    #[test]
    fn sample_xlsx_and_cp949_csv() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../docs/samples");
        for name in ["my-words.xlsx", "my-words-cp949.csv"] {
            let p = preview(&dir.join(name)).unwrap();
            assert_eq!((p.ok_count, p.duplicate_count, p.error_count), (5, 1, 2), "{name}");
            assert_eq!(p.rows[0].term, "apple", "{name}");
            assert_eq!(p.rows[3].meaning, "안건, 의제", "{name}");
            assert_eq!(p.rows[3].row, 6, "{name}: 빈 행을 건너뛰어도 원래 행 번호 유지");
        }
    }

    /// 앱이 나눠 주는 엑셀 양식 그대로 올려도 오류 없이 읽혀야 한다.
    #[test]
    fn bundled_template_imports_cleanly() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("resources/template/word-template.xlsx");
        let p = preview(&path).unwrap();
        assert_eq!((p.ok_count, p.duplicate_count, p.error_count), (3, 0, 0));
        assert_eq!(p.rows[0].term, "apple");
    }
}
