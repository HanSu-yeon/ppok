//! 단어 데이터 검증. 기본 단어장(엄격)과 사용자 업로드(관대) 모두 여기서 판단한다.

use std::collections::HashMap;

pub const MAX_TERM_LEN: usize = 80;
pub const MAX_MEANING_LEN: usize = 200;

pub const EN_LEVELS: [&str; 4] = ["basic", "intermediate", "advanced", "expert"];
pub const JA_LEVELS: [&str; 5] = ["N5", "N4", "N3", "N2", "N1"];

pub fn levels_for(language: &str) -> &'static [&'static str] {
    match language {
        "ja" => &JA_LEVELS,
        _ => &EN_LEVELS,
    }
}

/// 중복 판단용 키: 앞뒤 공백 제거, 연속 공백 하나로, 소문자.
pub fn term_key(term: &str) -> String {
    term.split_whitespace().collect::<Vec<_>>().join(" ").to_lowercase()
}

/// 셀 값 정리: 제어문자 제거, 공백 정리.
pub fn clean(s: &str) -> String {
    let no_ctrl: String = s
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .filter(|c| *c != '\u{feff}')
        .collect();
    no_ctrl.split_whitespace().collect::<Vec<_>>().join(" ")
}

pub fn has_hangul(s: &str) -> bool {
    s.chars().any(|c| matches!(c, '\u{AC00}'..='\u{D7A3}' | '\u{1100}'..='\u{11FF}' | '\u{3130}'..='\u{318F}'))
}

fn is_kana(c: char) -> bool {
    matches!(c, '\u{3040}'..='\u{309F}' | '\u{30A0}'..='\u{30FF}')
}

fn is_kanji(c: char) -> bool {
    matches!(c, '\u{4E00}'..='\u{9FFF}' | '\u{3400}'..='\u{4DBF}' | '\u{3005}')
}

pub fn has_japanese(s: &str) -> bool {
    s.chars().any(|c| is_kana(c) || is_kanji(c))
}

/// 기본 단어장의 중복 판단 키. 읽기가 있으면 함께 본다 (今日: きょう / こんにち).
pub fn builtin_key(term: &str, reading: &str) -> String {
    if reading.is_empty() { term_key(term) } else { format!("{}|{}", term_key(term), reading) }
}

fn is_english_term(s: &str) -> bool {
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && s.chars().all(|c| c.is_ascii_alphabetic() || matches!(c, ' ' | '\'' | '-' | '.'))
}

/// 업로드/기본 공통 규칙. 문제 없으면 None, 있으면 사용자에게 보여줄 이유.
pub fn check_entry(term: &str, meaning: &str) -> Option<String> {
    if term.is_empty() {
        return Some("단어가 비어 있어요".into());
    }
    if meaning.is_empty() {
        return Some("뜻이 비어 있어요".into());
    }
    if term.chars().count() > MAX_TERM_LEN {
        return Some(format!("단어가 너무 길어요 ({MAX_TERM_LEN}자 이하)"));
    }
    if meaning.chars().count() > MAX_MEANING_LEN {
        return Some(format!("뜻이 너무 길어요 ({MAX_MEANING_LEN}자 이하)"));
    }
    if has_hangul(term) && !has_hangul(meaning) {
        return Some("단어 칸에 한글이 있어요. 열 순서(단어, 뜻)를 확인해 주세요".into());
    }
    None
}

/// 기본 단어장 전용 추가 규칙.
pub fn check_builtin(language: &str, term: &str, reading: &str, meaning: &str, level: &str) -> Option<String> {
    if let Some(e) = check_entry(term, meaning) {
        return Some(e);
    }
    if !levels_for(language).contains(&level) {
        return Some(format!("알 수 없는 난이도 '{level}'"));
    }
    if language == "en" && !is_english_term(term) {
        return Some("영어 단어 형식이 아니에요".into());
    }
    if language == "ja" {
        if !has_japanese(term) || has_hangul(term) {
            return Some("일본어 단어 형식이 아니에요".into());
        }
        if !reading.chars().all(|c| is_kana(c) || c == ' ') {
            return Some("읽기는 히라가나·가타카나로만 적어 주세요".into());
        }
        if reading.is_empty() && term.chars().any(is_kanji) {
            return Some("한자가 있는 단어는 읽기가 필요해요".into());
        }
    }
    if !has_hangul(meaning) {
        return Some("뜻에 한국어가 없어요".into());
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum RowStatus {
    Ok,
    Duplicate,
    Error,
}

/// 같은 파일 안 중복 감지. 처음 나온 행만 Ok로 남긴다.
pub struct DupTracker {
    seen: HashMap<String, usize>,
}

impl DupTracker {
    pub fn new() -> Self {
        Self { seen: HashMap::new() }
    }

    /// 이미 나온 키면 처음 나온 행 번호를 돌려준다.
    pub fn check(&mut self, term: &str, row: usize) -> Option<usize> {
        let key = term_key(term);
        match self.seen.get(&key) {
            Some(first) => Some(*first),
            None => {
                self.seen.insert(key, row);
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_normalize_case_and_spaces() {
        assert_eq!(term_key("  Look   Up "), "look up");
        assert_eq!(clean("a\tb\u{feff}  c"), "a b c");
    }

    #[test]
    fn swapped_columns_are_flagged() {
        assert!(check_entry("사과", "apple").is_some());
        assert!(check_entry("apple", "사과").is_none());
        assert!(check_entry("りんご", "사과").is_none());
    }

    #[test]
    fn builtin_rules() {
        assert!(check_builtin("en", "apple", "", "사과", "basic").is_none());
        assert!(check_builtin("en", "apple", "", "사과", "easy").is_some());
        assert!(check_builtin("en", "app1e", "", "사과", "basic").is_some());
        assert!(check_builtin("en", "apple", "", "apple", "basic").is_some());
        assert!(check_builtin("en", "well-being", "", "행복", "advanced").is_none());
    }

    #[test]
    fn japanese_rules() {
        assert!(check_builtin("ja", "食べる", "たべる", "먹다", "N5").is_none());
        assert!(check_builtin("ja", "ありがとう", "", "고마워", "N5").is_none());
        assert!(check_builtin("ja", "食べる", "", "먹다", "N5").is_some()); // 한자인데 읽기 없음
        assert!(check_builtin("ja", "食べる", "taberu", "먹다", "N5").is_some());
        assert!(check_builtin("ja", "eat", "", "먹다", "N5").is_some());
        assert!(check_builtin("ja", "食べる", "たべる", "먹다", "basic").is_some());
        assert_ne!(builtin_key("今日", "きょう"), builtin_key("今日", "こんにち"));
    }

    #[test]
    fn duplicates_detected_case_insensitive() {
        let mut d = DupTracker::new();
        assert_eq!(d.check("Apple", 2), None);
        assert_eq!(d.check("apple ", 5), Some(2));
    }
}
