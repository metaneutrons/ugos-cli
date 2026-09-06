//! Error types for the UGOS client library.

/// All errors returned by the UGOS client.
///
/// Non-exhaustive on purpose. This audit added [`UgosError::Io`], and without
/// the attribute that alone would have broken every downstream `match` that
/// listed the variants, making each future error kind a breaking release. A
/// consumer matches the variants it handles and keeps a `_` arm for the rest.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    ///
    /// Carries the request URL for context, with sensitive query parameters
    /// replaced. See [`redact_url`] for why that matters.
    #[error("request to {url} failed")]
    Http {
        /// The request URL, with sensitive query parameters redacted.
        url: String,
        /// The underlying transport error, stripped of its own URL.
        source: reqwest::Error,
    },

    /// JSON serialization/deserialization error.
    #[error(transparent)]
    Json(#[from] serde_json::Error),

    /// Filesystem failure while reading or writing a local file.
    ///
    /// Kept apart from [`UgosError::Encryption`] on purpose. A download into
    /// an unwritable directory used to report an encryption error, which sent
    /// the reader looking at TLS and keys instead of at the directory.
    #[error("{context}")]
    Io {
        /// What was being attempted, including the path.
        context: String,
        /// The underlying filesystem error.
        source: std::io::Error,
    },

    /// VM or resource not found by display name.
    #[error("{kind} not found: {name}")]
    NotFound {
        /// The resource kind (e.g. "VM", "network").
        kind: &'static str,
        /// The name that was looked up.
        name: String,
    },
}

impl UgosError {
    /// Wrap a filesystem error with what was being attempted.
    pub(crate) fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source,
        }
    }
}

/// Convenience alias used throughout the library.
pub type Result<T> = std::result::Result<T, UgosError>;

/// Query parameters whose values must never reach an error message.
///
/// `encrypt_query` is deliberately absent: it carries the token too, but
/// encrypted, and seeing that a request was encrypted is useful when
/// debugging.
const SENSITIVE_PARAMS: [&str; 3] = ["token", "password", "passwd"];

/// Render a URL with sensitive query parameters replaced.
///
/// UGOS passes the session token as `?token=`, and a token stays valid for
/// 25 minutes. Error text is routinely pasted into bug reports, so leaving
/// it in would publish a live credential.
#[must_use]
pub fn redact_url(url: &reqwest::Url) -> String {
    if url.query().is_none() {
        return url.to_string();
    }

    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(key, value)| {
            let value = if SENSITIVE_PARAMS.contains(&key.as_ref()) {
                "REDACTED".to_owned()
            } else {
                value.into_owned()
            };
            (key.into_owned(), value)
        })
        .collect();

    let mut clean = url.clone();
    let _serializer = clean.query_pairs_mut().clear().extend_pairs(pairs);
    clean.to_string()
}

impl From<reqwest::Error> for UgosError {
    /// Redact the URL before it can reach a log or a bug report.
    ///
    /// The error's own copy of the URL is removed as well: it is reachable
    /// through `source()`, which error reporters print, so redacting only
    /// the outer message would leave the token visible one line down.
    fn from(err: reqwest::Error) -> Self {
        let url = err
            .url()
            .map_or_else(|| "<unknown URL>".to_owned(), redact_url);
        Self::Http {
            url,
            source: err.without_url(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(input: &str) -> reqwest::Url {
        reqwest::Url::parse(input).unwrap_or_else(|_| {
            reqwest::Url::parse("https://invalid.example/").unwrap_or_else(|_| unreachable!())
        })
    }

    #[test]
    fn removes_the_session_token() {
        let shown = redact_url(&url("https://nas:9443/ugreen/v1/vm?token=SECRET123"));
        assert!(!shown.contains("SECRET123"));
        // Deliberately free of URL-escapable characters, so the redaction
        // stays readable in the rendered URL.
        assert!(shown.contains("token=REDACTED"));
    }

    #[test]
    fn keeps_the_path_for_debugging() {
        let shown = redact_url(&url(
            "https://nas:9443/ugreen/v1/sysinfo/machine/common?token=X",
        ));
        assert!(shown.contains("/ugreen/v1/sysinfo/machine/common"));
        assert!(shown.contains("nas:9443"));
    }

    #[test]
    fn keeps_harmless_parameters() {
        let shown = redact_url(&url("https://nas/v1/kvm/PowerOn?name=debian&token=X"));
        assert!(shown.contains("name=debian"));
        assert!(!shown.contains("token=X"));
    }

    #[test]
    fn redacts_a_token_in_any_position() {
        let shown = redact_url(&url(
            "https://nas/v1/x?token=SECRET&name=a&password=HUNTER2",
        ));
        assert!(!shown.contains("SECRET"));
        assert!(!shown.contains("HUNTER2"));
    }

    #[test]
    fn leaves_a_url_without_a_query_alone() {
        let plain = "https://nas:9443/ugreen/v1/verify/login";
        assert_eq!(redact_url(&url(plain)), plain);
    }

    #[test]
    fn keeps_the_encrypted_query_visible() {
        // Encrypted queries carry the token too, but not in the clear.
        let shown = redact_url(&url("https://nas/v2/filemgr/list?encrypt_query=AbCd"));
        assert!(shown.contains("encrypt_query=AbCd"));
    }
}
