use serde::{Serialize, Serializer};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("데이터베이스 오류: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("파일 오류: {0}")]
    Io(#[from] std::io::Error),
    #[error("CSV 오류: {0}")]
    Csv(#[from] csv::Error),
    #[error("엑셀 파일을 읽지 못했어요: {0}")]
    Excel(String),
    #[error("{0}")]
    Tauri(#[from] tauri::Error),
    #[error("{0}")]
    Json(#[from] serde_json::Error),
    #[error("{0}")]
    Message(String),
}

impl Error {
    pub fn msg(s: impl Into<String>) -> Self {
        Error::Message(s.into())
    }
}

// invoke 오류는 프론트엔드에 사람이 읽을 수 있는 문자열로 전달한다.
impl Serialize for Error {
    fn serialize<S: Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        s.serialize_str(&self.to_string())
    }
}

pub type Result<T> = std::result::Result<T, Error>;
