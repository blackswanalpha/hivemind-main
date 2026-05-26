//! Persistence trait for chat conversations.
//!
//! Lives in `ai-orchestrator` (not `storage`) so the dependency edge runs
//! `storage → ai-orchestrator`, never the reverse. The `storage` crate
//! provides `SqliteAiPersistence` as the concrete impl.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use thiserror::Error;

#[derive(Debug, Error)]
#[error("persistence error: {0}")]
pub struct PersistenceError(pub anyhow::Error);

impl PersistenceError {
    pub fn new(e: impl Into<anyhow::Error>) -> Self {
        Self(e.into())
    }
}

#[derive(Clone, Debug)]
pub struct ConversationRecord {
    pub id: String,
    pub workspace_id: String,
    pub started_at: DateTime<Utc>,
}

#[derive(Clone, Debug)]
pub struct MessageRecord {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub created_at: DateTime<Utc>,
}

#[async_trait]
pub trait AiPersistence: Send + Sync + 'static {
    async fn create_conversation(&self, workspace_id: &str) -> Result<String, PersistenceError>;

    async fn list_conversations(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ConversationRecord>, PersistenceError>;

    async fn delete_conversation(&self, conversation_id: &str) -> Result<(), PersistenceError>;

    async fn load_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageRecord>, PersistenceError>;

    async fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<i64, PersistenceError>;

    async fn first_user_message(
        &self,
        conversation_id: &str,
    ) -> Result<Option<String>, PersistenceError>;

    /// Config getter/setter — used to persist routing settings.
    async fn get_config(&self, key: &str) -> Result<Option<String>, PersistenceError>;
    async fn set_config(&self, key: &str, value: &str) -> Result<(), PersistenceError>;
}
