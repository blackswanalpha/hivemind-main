use thiserror::Error;

use crate::persistence::PersistenceError;
use hivemind_ai_provider::ProviderError;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("persistence: {0}")]
    Persistence(#[from] PersistenceError),

    #[error("provider: {0}")]
    Provider(#[from] ProviderError),

    #[error("conversation not found: {0}")]
    NoSuchConversation(String),

    #[error("internal: {0}")]
    Internal(String),
}

impl OrchestratorError {
    pub fn code(&self) -> &'static str {
        match self {
            OrchestratorError::Persistence(_) => "persistence",
            OrchestratorError::Provider(e) => e.code(),
            OrchestratorError::NoSuchConversation(_) => "no_such_conversation",
            OrchestratorError::Internal(_) => "internal",
        }
    }
}
