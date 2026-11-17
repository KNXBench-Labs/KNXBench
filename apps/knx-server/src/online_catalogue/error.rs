//! The closed set of reasons an online-catalogue operation can end without its result.

use axum::http::StatusCode;

/// Why a vault operation failed. No variant carries a secret.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VaultError {
    /// The credential file exists but its key file does not: never replaced silently.
    KeyMissing,
    /// The key file is readable by group or others and is therefore not used.
    KeyTooOpen,
    /// The key file is not exactly one 32-byte key.
    KeyInvalid,
    /// The credential file does not authenticate under the key (damaged or altered).
    Corrupt,
    /// The stored account belongs to another server than the configured one.
    OriginMismatch,
    /// A filesystem error.
    Io,
}

/// An online-catalogue operation's refusal, each with a stable machine-readable token.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OnlineError {
    /// This server has nowhere to keep catalogue state (no data directory).
    Unavailable,
    UnknownSource,
    /// The configured base address is not an acceptable HTTPS origin.
    InvalidBaseUrl(&'static str),
    /// No usable local index: a manual synchronisation is needed first.
    NoIndex,
    UnknownEntry,
    /// The selected search hit changed since it was displayed.
    SelectionChanged,
    /// The entry's file type is outside the downloadable set.
    NotDownloadable,
    /// No stored account: the user must sign in first.
    NeedsLogin,
    /// A stored token was refused and no password exists to renew it.
    TokenRejected,
    /// The server refused the user name or password; nothing was retried.
    InvalidCredentials,
    Forbidden,
    NotFound,
    /// The per-account daily download limit; `reset_at` is the server's own value.
    DailyLimit {
        reset_at: Option<String>,
        retry_after_secs: Option<u64>,
    },
    RateLimited {
        retry_after_secs: Option<u64>,
    },
    ServiceUnavailable,
    /// Another synchronisation or download is already running.
    Busy,
    Cancelled,
    /// Connect, TLS, timeout, protocol or redirect failure.
    Transport(&'static str),
    /// A response or file failed validation (size, digest, schema, encoding).
    Integrity(&'static str),
    Vault(VaultError),
    /// A local storage failure.
    Storage(&'static str),
}

impl OnlineError {
    /// The camelCase token a client acts on.
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Unavailable => "unavailable",
            Self::UnknownSource => "unknownSource",
            Self::InvalidBaseUrl(_) => "invalidBaseUrl",
            Self::NoIndex => "noIndex",
            Self::UnknownEntry => "unknownEntry",
            Self::SelectionChanged => "selectionChanged",
            Self::NotDownloadable => "notDownloadable",
            Self::NeedsLogin => "needsLogin",
            Self::TokenRejected => "tokenRejected",
            Self::InvalidCredentials => "invalidCredentials",
            Self::Forbidden => "forbidden",
            Self::NotFound => "notFound",
            Self::DailyLimit { .. } => "dailyLimit",
            Self::RateLimited { .. } => "rateLimited",
            Self::ServiceUnavailable => "serviceUnavailable",
            Self::Busy => "busy",
            Self::Cancelled => "cancelled",
            Self::Transport(_) => "transport",
            Self::Integrity(_) => "integrity",
            Self::Vault(VaultError::KeyMissing) => "vaultKeyMissing",
            Self::Vault(VaultError::OriginMismatch) => "vaultOriginMismatch",
            Self::Vault(_) => "vault",
            Self::Storage(_) => "storage",
        }
    }

    pub fn http_status(&self) -> StatusCode {
        match self {
            Self::Unavailable | Self::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            Self::UnknownSource | Self::UnknownEntry | Self::NotFound => StatusCode::NOT_FOUND,
            Self::InvalidBaseUrl(_) | Self::NotDownloadable => StatusCode::BAD_REQUEST,
            Self::NoIndex
            | Self::NeedsLogin
            | Self::TokenRejected
            | Self::InvalidCredentials
            | Self::Busy
            | Self::SelectionChanged
            | Self::Cancelled
            | Self::Vault(VaultError::KeyMissing | VaultError::OriginMismatch) => {
                StatusCode::CONFLICT
            }
            Self::Forbidden => StatusCode::FORBIDDEN,
            Self::DailyLimit { .. } | Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
            Self::Transport(_) | Self::Integrity(_) => StatusCode::BAD_GATEWAY,
            Self::Vault(_) | Self::Storage(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    /// A short English sentence naming the refusal; clients word their own text from `kind`.
    pub fn message(&self) -> String {
        match self {
            Self::InvalidBaseUrl(why) => format!("the catalogue address is not acceptable: {why}"),
            Self::Transport(why) => format!("the catalogue server could not be reached: {why}"),
            Self::Integrity(why) => format!("the catalogue server's answer was refused: {why}"),
            Self::Storage(why) => format!("catalogue storage failed: {why}"),
            other => other.kind().to_string(),
        }
    }
}
