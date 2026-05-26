use std::fmt;

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("not found: {0}")]
    NotFound(String),

    #[error("invalid input: {0}")]
    Invalid(String),

    #[error("storage backend error: {0}")]
    Backend(Box<dyn std::error::Error + Send + Sync + 'static>),
}

impl StoreError {
    pub fn backend<E>(err: E) -> Self
    where
        E: std::error::Error + Send + Sync + 'static,
    {
        StoreError::Backend(Box::new(err))
    }

    pub fn invalid(msg: impl fmt::Display) -> Self {
        StoreError::Invalid(msg.to_string())
    }
}
