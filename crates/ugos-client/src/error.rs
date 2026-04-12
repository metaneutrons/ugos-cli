//! Error types for the UGOS client library.

/// All errors returned by the UGOS client.
#[derive(Debug, thiserror::Error)]
pub enum UgosError {
    /// Incorrect account or password (UGOS code 1003).
    #[error("incorrect account or password")]
    AuthFailed,

    /// Parameter error (UGOS code 1005).
    #[error("parameter error: {0}")]
    ParameterError(String),

    /// Login expired or invalid token (UGOS code 1024).
    #[error("login expired")]
    LoginExpired,

    /// VM operation failed (UGOS code 3004).
    #[error("operation failed: {0}")]
    OperationFailed(String),

    /// App not found / not installed (UGOS code 9404).
    #[error("app not found: {0}")]
    AppNotFound(String),

    /// App service error (UGOS code 9405).
    #[error("app service error: {0}")]
    AppServiceError(String),

    /// Unexpected API error code.
    #[error("API error {code}: {msg}")]
    Api {
        /// The UGOS error code.
        code: i32,
        /// The error message from the API.
        msg: String,
    },

    /// RSA encryption or key parsing failure.
    #[error("encryption error: {0}")]
    Encryption(String),

    /// HTTP transport error.
    #[error(transparent)]
    Http(#[from] reqwest::Error),

    /// JSON serialization/deserialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// VM or resource not found by display name.
    #[error("{kind} not found: {name}")]
    NotFound {
        /// The resource kind (e.g. "VM", "network").
        kind: &'static str,
        /// The name that was looked up.
        name: String,
    },
}

/// Convenience alias used throughout the library.
pub type Result<T> = std::result::Result<T, UgosError>;
