//! API key loading.
//!
//! P0 reads from the `ANTHROPIC_API_KEY` env var only. The keyring path stays
//! out of the dependency graph until packaging time — having `keyring` as a
//! workspace dep in `Cargo.toml` does not buy us anything until we wire a UI
//! that lets users set keys without a terminal.

use crate::error::ProviderError;

pub const ANTHROPIC_KEY_VAR: &str = "ANTHROPIC_API_KEY";

pub fn anthropic_api_key() -> Result<String, ProviderError> {
    match std::env::var(ANTHROPIC_KEY_VAR) {
        Ok(v) if !v.trim().is_empty() => Ok(v),
        _ => Err(ProviderError::Auth),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // The whole point of these tests is to mutate a process-global env var.
    // Serialize them so the parallel test runner doesn't shred the results.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_env<F: FnOnce()>(value: Option<&str>, f: F) {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let prev = std::env::var(ANTHROPIC_KEY_VAR).ok();
        // SAFETY: this whole block is serialized by ENV_LOCK; the function
        // is called only from test threads inside #[cfg(test)].
        match value {
            Some(v) => unsafe { std::env::set_var(ANTHROPIC_KEY_VAR, v) },
            None => unsafe { std::env::remove_var(ANTHROPIC_KEY_VAR) },
        }
        f();
        match prev {
            Some(v) => unsafe { std::env::set_var(ANTHROPIC_KEY_VAR, v) },
            None => unsafe { std::env::remove_var(ANTHROPIC_KEY_VAR) },
        }
    }

    #[test]
    fn missing_env_returns_auth() {
        with_env(None, || {
            let err = anthropic_api_key().unwrap_err();
            assert!(matches!(err, ProviderError::Auth));
        });
    }

    #[test]
    fn empty_env_returns_auth() {
        with_env(Some("   "), || {
            let err = anthropic_api_key().unwrap_err();
            assert!(matches!(err, ProviderError::Auth));
        });
    }

    #[test]
    fn set_env_returns_value() {
        with_env(Some("sk-test-key"), || {
            assert_eq!(anthropic_api_key().unwrap(), "sk-test-key");
        });
    }
}
