use chrono::{Local, NaiveTime, Timelike};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::db;
use crate::error::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Source {
    /// 예전 버전의 "기본 + 내 단어장"(mixed) 설정은 기본 단어장으로 읽는다.
    #[serde(alias = "mixed")]
    Builtin,
    User,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// 맞혀 보기: 단어만 먼저, 누르면 뜻 공개
    Reveal,
    /// 바로 보기: 단어와 뜻을 함께
    Instant,
}

/// 팝업이 나타나는 화면 모서리. `Random`이면 팝업마다 네 모서리 중 하나.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Corner {
    BottomRight,
    BottomLeft,
    TopRight,
    TopLeft,
    Random,
}

impl Corner {
    const FIXED: [Corner; 4] = [Corner::BottomRight, Corner::BottomLeft, Corner::TopRight, Corner::TopLeft];

    /// 실제로 띄울 모서리. 랜덤이면 직전과 다른 모서리를 고른다.
    pub fn resolve(self, previous: Option<Corner>) -> Corner {
        if self != Corner::Random {
            return self;
        }
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        let candidates: Vec<Corner> = Self::FIXED.into_iter().filter(|c| Some(*c) != previous).collect();
        candidates[nanos as usize % candidates.len()]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Settings {
    pub onboarded: bool,
    pub language: String,
    pub source: Source,
    pub level: String,
    pub mode: Mode,
    pub interval_min: u32,
    pub daily_goal: u32,
    pub popup_seconds: u32,
    pub popup_corner: Corner,
    pub hours_enabled: bool,
    pub hours_start: String,
    pub hours_end: String,
    /// unix 초. `PAUSE_FOREVER`면 직접 재개할 때까지.
    pub paused_until: Option<i64>,
    pub autostart: bool,
}

pub const PAUSE_FOREVER: i64 = i64::MAX;

impl Default for Settings {
    fn default() -> Self {
        Self {
            onboarded: false,
            language: "en".into(),
            source: Source::Builtin,
            level: "basic".into(),
            mode: Mode::Reveal,
            interval_min: 30,
            daily_goal: 10,
            popup_seconds: 10,
            popup_corner: Corner::BottomRight,
            hours_enabled: false,
            hours_start: "09:00".into(),
            hours_end: "18:00".into(),
            paused_until: None,
            autostart: false,
        }
    }
}

impl Settings {
    pub fn load(conn: &Connection) -> Result<Self> {
        match db::get_setting(conn, "settings")? {
            Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            None => Ok(Self::default()),
        }
    }

    pub fn save(&self, conn: &Connection) -> Result<()> {
        db::set_setting(conn, "settings", &serde_json::to_string(self)?)
    }

    pub fn validate(&mut self) -> Result<()> {
        if !matches!(self.language.as_str(), "en" | "ja") {
            return Err(Error::msg("지원하지 않는 언어예요"));
        }
        if !crate::validate::levels_for(&self.language).contains(&self.level.as_str()) {
            self.level = crate::validate::levels_for(&self.language)[0].to_string();
        }
        self.interval_min = self.interval_min.clamp(1, 24 * 60);
        self.daily_goal = self.daily_goal.clamp(1, 500);
        self.popup_seconds = self.popup_seconds.clamp(3, 120);
        for t in [&self.hours_start, &self.hours_end] {
            if parse_hm(t).is_none() {
                return Err(Error::msg("시간 형식은 HH:MM 이에요"));
            }
        }
        Ok(())
    }

    pub fn is_paused(&self, now: i64) -> bool {
        matches!(self.paused_until, Some(until) if until > now)
    }

    /// 알림 허용 시간대 안인지. 시작 > 끝이면 자정을 넘기는 구간으로 본다.
    pub fn within_hours(&self, now: NaiveTime) -> bool {
        if !self.hours_enabled {
            return true;
        }
        let (Some(start), Some(end)) = (parse_hm(&self.hours_start), parse_hm(&self.hours_end)) else {
            return true;
        };
        let t = now.hour() * 60 + now.minute();
        if start == end {
            true
        } else if start < end {
            start <= t && t < end
        } else {
            t >= start || t < end
        }
    }

    pub fn within_hours_now(&self) -> bool {
        self.within_hours(Local::now().time())
    }
}

fn parse_hm(s: &str) -> Option<u32> {
    let (h, m) = s.split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(h: u32, m: u32) -> NaiveTime {
        NaiveTime::from_hms_opt(h, m, 0).unwrap()
    }

    #[test]
    fn old_mixed_source_reads_as_builtin() {
        let s: Settings = serde_json::from_str(r#"{"onboarded":true,"source":"mixed"}"#).unwrap();
        assert_eq!(s.source, Source::Builtin);
        assert!(s.onboarded);
    }

    #[test]
    fn random_corner_never_repeats() {
        assert_eq!(Corner::TopLeft.resolve(Some(Corner::TopLeft)), Corner::TopLeft);
        for _ in 0..50 {
            let c = Corner::Random.resolve(Some(Corner::BottomRight));
            assert!(c != Corner::BottomRight && c != Corner::Random);
        }
    }

    #[test]
    fn hours_window() {
        let mut s = Settings { hours_enabled: true, ..Default::default() };
        assert!(s.within_hours(at(9, 0)));
        assert!(!s.within_hours(at(18, 0)));
        assert!(!s.within_hours(at(7, 59)));
        s.hours_start = "22:00".into();
        s.hours_end = "02:00".into();
        assert!(s.within_hours(at(23, 30)));
        assert!(s.within_hours(at(1, 0)));
        assert!(!s.within_hours(at(12, 0)));
    }
}
