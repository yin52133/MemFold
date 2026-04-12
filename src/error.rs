use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("TOML decode error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML encode error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("invalid scope id: {0}")]
    InvalidScopeId(String),

    #[error("invalid {kind}: {value}")]
    InvalidEnumValue { kind: &'static str, value: String },

    #[error("bundle missing for scope: {0}")]
    BundleMissing(String),

    #[error("unstable mutation prevents load for scope: {0}")]
    UnstableMutation(String),

    #[error("dreaming not eligible: {0}")]
    DreamingNotEligible(String),

    #[error("query cannot be empty")]
    EmptyQuery,

    #[error("unsafe summary rejected")]
    UnsafeSummary,

    #[error("session missing: {0}")]
    SessionMissing(String),

    #[error("trace not found: {0}")]
    TraceNotFound(String),

    #[error("session log corrupted: {0}")]
    SessionLogCorrupted(String),

    #[error("history summary failed: {0}")]
    HistorySummaryFailed(String),
}

impl Error {
    pub fn error_code(&self) -> i32 {
        match self {
            Self::Io(_) => 10,
            Self::Sqlite(_) => 11,
            Self::InvalidScopeId(_) | Self::InvalidEnumValue { .. } => 12,
            Self::BundleMissing(_) => 21,
            Self::UnstableMutation(_) => 22,
            Self::DreamingNotEligible(_) => 51,
            Self::EmptyQuery => 40,
            Self::UnsafeSummary => 30,
            Self::SessionMissing(_) => 31,
            Self::TraceNotFound(_) => 41,
            Self::SessionLogCorrupted(_) => 42,
            Self::HistorySummaryFailed(_) => 52,
            Self::TomlDe(_) | Self::TomlSer(_) | Self::Json(_) => 13,
        }
    }

    pub fn error_name(&self) -> &'static str {
        match self {
            Self::Io(_) => "IO_ERROR",
            Self::Sqlite(_) => "SQLITE_ERROR",
            Self::InvalidScopeId(_) => "INVALID_SCOPE_ID",
            Self::InvalidEnumValue { .. } => "INVALID_ARGUMENT",
            Self::BundleMissing(_) => "BUNDLE_MISSING",
            Self::UnstableMutation(_) => "UNSTABLE_MUTATION",
            Self::DreamingNotEligible(_) => "DREAMING_NOT_ELIGIBLE",
            Self::EmptyQuery => "EMPTY_QUERY",
            Self::UnsafeSummary => "UNSAFE_SUMMARY",
            Self::SessionMissing(_) => "SESSION_MISSING",
            Self::TraceNotFound(_) => "TRACE_NOT_FOUND",
            Self::SessionLogCorrupted(_) => "SESSION_LOG_CORRUPTED",
            Self::HistorySummaryFailed(_) => "HISTORY_SUMMARY_FAILED",
            Self::TomlDe(_) => "TOML_DECODE_ERROR",
            Self::TomlSer(_) => "TOML_ENCODE_ERROR",
            Self::Json(_) => "JSON_ERROR",
        }
    }

    pub fn retryable(&self) -> bool {
        matches!(self, Self::UnstableMutation(_))
    }
}
