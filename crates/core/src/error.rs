use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("serde json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("toml deserialization error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("toml serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("sqlite error: {0}")]
    Sqlite(#[from] rusqlite::Error),

    #[error("http error: {0}")]
    Http(String),

    #[error("http status {status}: {body}")]
    HttpStatus { status: u16, body: String },

    #[error("oauth error: {0}")]
    Oauth(String),

    #[error("rate limited (HTTP 429). retry after {retry_after_secs}s")]
    RateLimited { retry_after_secs: u64 },

    #[error("Claude Code not logged in on this PC. Run `claude login` or set CLAUDE_OAUTH_TOKEN (searched: {path})")]
    CredentialsNotFound { path: String },

    #[error("credentials malformed: {0}")]
    CredentialsMalformed(String),

    #[error("config error: {0}")]
    Config(String),

    #[error("unexpected response shape: {0}")]
    UnexpectedResponse(String),

    #[error("time parsing error: {0}")]
    Time(String),
}

impl From<ureq::Error> for CoreError {
    fn from(e: ureq::Error) -> Self {
        match e {
            ureq::Error::Status(code, resp) => {
                if code == 429 {
                    let retry_after = resp
                        .header("retry-after")
                        .and_then(|v| v.parse::<u64>().ok())
                        .unwrap_or(300);
                    CoreError::RateLimited {
                        retry_after_secs: retry_after,
                    }
                } else {
                    let body = resp
                        .into_string()
                        .unwrap_or_else(|_| "<unreadable body>".to_string());
                    CoreError::HttpStatus { status: code, body }
                }
            }
            ureq::Error::Transport(t) => CoreError::Http(t.to_string()),
        }
    }
}

pub type Result<T> = std::result::Result<T, CoreError>;
