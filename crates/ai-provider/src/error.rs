//! Errors returned from any [`Provider`](crate::Provider) call.
//!
//! Retry policy lives in the provider, not the orchestrator (see `docs/ai.md`
//! §2.6). The `code()` helper produces a stable short identifier that the
//! frontend uses for error rendering and switch statements.

use std::time::Duration;

use thiserror::Error;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("network: {0}")]
    Network(#[from] reqwest::Error),

    #[error("http {code}: {body}")]
    Status { code: u16, body: String },

    #[error("rate limited")]
    RateLimited { retry_after: Option<Duration> },

    #[error("auth failed")]
    Auth,

    #[error("schema drift: {0}")]
    SchemaDrift(String),

    #[error("unsupported: {0}")]
    Unsupported(String),

    #[error("cancelled")]
    Cancelled,
}

impl ProviderError {
    /// Stable short identifier suitable for matching in the frontend.
    pub fn code(&self) -> &'static str {
        match self {
            ProviderError::Network(_) => "network",
            ProviderError::Status { .. } => "status",
            ProviderError::RateLimited { .. } => "rate_limited",
            ProviderError::Auth => "auth",
            ProviderError::SchemaDrift(_) => "schema_drift",
            ProviderError::Unsupported(_) => "unsupported",
            ProviderError::Cancelled => "cancelled",
        }
    }

    /// Maps an HTTP status + headers + body into an error variant. The body is
    /// already drained because reqwest moves the response on `.bytes()`.
    pub fn from_status(code: u16, retry_after: Option<Duration>, body: String) -> Self {
        match code {
            401 | 403 => ProviderError::Auth,
            429 => ProviderError::RateLimited { retry_after },
            _ => ProviderError::Status { code, body },
        }
    }
}

/// Parse a `Retry-After` header value into a [`Duration`]. Supports the
/// integer-seconds form. HTTP-date form is unsupported and falls back to None
/// (the caller's backoff default applies).
pub fn parse_retry_after(value: Option<&str>) -> Option<Duration> {
    let raw = value?.trim();
    raw.parse::<u64>().ok().map(Duration::from_secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn status_401_to_auth() {
        let e = ProviderError::from_status(401, None, "nope".into());
        assert!(matches!(e, ProviderError::Auth));
        assert_eq!(e.code(), "auth");
    }

    #[test]
    fn status_429_to_rate_limited_with_retry_after() {
        let e = ProviderError::from_status(429, Some(Duration::from_secs(5)), "slow".into());
        match e {
            ProviderError::RateLimited { retry_after } => {
                assert_eq!(retry_after, Some(Duration::from_secs(5)));
            }
            _ => panic!("expected RateLimited"),
        }
    }

    #[test]
    fn status_500_to_status_variant() {
        let e = ProviderError::from_status(500, None, "boom".into());
        match e {
            ProviderError::Status { code, body } => {
                assert_eq!(code, 500);
                assert_eq!(body, "boom");
            }
            _ => panic!("expected Status"),
        }
    }

    #[test]
    fn parse_retry_after_seconds() {
        assert_eq!(parse_retry_after(Some("3")), Some(Duration::from_secs(3)));
        assert_eq!(parse_retry_after(Some("  10 ")), Some(Duration::from_secs(10)));
        assert_eq!(parse_retry_after(Some("Wed, 21 Oct 2026 07:28:00 GMT")), None);
        assert_eq!(parse_retry_after(None), None);
    }
}
