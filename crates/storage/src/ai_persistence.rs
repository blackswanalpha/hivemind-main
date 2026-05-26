//! SQLite-backed implementation of [`AiPersistence`].
//!
//! Targets the existing `conversations`, `messages`, and `config` tables from
//! `0001_init.sql`. No new migrations required.

use async_trait::async_trait;
use chrono::{TimeZone, Utc};
use hivemind_ai_orchestrator::{
    AiPersistence, ConversationRecord, MessageRecord, PersistenceError,
};
use sqlx::{Row, SqlitePool};
use ulid::Ulid;

#[derive(Clone)]
pub struct SqliteAiPersistence {
    pool: SqlitePool,
}

impl SqliteAiPersistence {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

fn wrap<E: std::error::Error + Send + Sync + 'static>(e: E) -> PersistenceError {
    PersistenceError::new(anyhow::anyhow!(e))
}

#[async_trait]
impl AiPersistence for SqliteAiPersistence {
    async fn create_conversation(&self, workspace_id: &str) -> Result<String, PersistenceError> {
        let id = Ulid::new().to_string();
        let now = Utc::now().timestamp();
        sqlx::query("INSERT INTO conversations (id, workspace_id, started_at) VALUES (?, ?, ?)")
            .bind(&id)
            .bind(workspace_id)
            .bind(now)
            .execute(&self.pool)
            .await
            .map_err(wrap)?;
        Ok(id)
    }

    async fn list_conversations(
        &self,
        workspace_id: &str,
    ) -> Result<Vec<ConversationRecord>, PersistenceError> {
        let rows = sqlx::query(
            r#"SELECT id, workspace_id, started_at
               FROM conversations
               WHERE workspace_id = ?
               ORDER BY started_at DESC"#,
        )
        .bind(workspace_id)
        .fetch_all(&self.pool)
        .await
        .map_err(wrap)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let id: String = r.try_get("id").map_err(wrap)?;
            let ws: String = r.try_get("workspace_id").map_err(wrap)?;
            let ts: i64 = r.try_get("started_at").map_err(wrap)?;
            let started_at = Utc.timestamp_opt(ts, 0).single().ok_or_else(|| {
                PersistenceError::new(anyhow::anyhow!("invalid started_at: {ts}"))
            })?;
            out.push(ConversationRecord {
                id,
                workspace_id: ws,
                started_at,
            });
        }
        Ok(out)
    }

    async fn delete_conversation(&self, conversation_id: &str) -> Result<(), PersistenceError> {
        sqlx::query("DELETE FROM conversations WHERE id = ?")
            .bind(conversation_id)
            .execute(&self.pool)
            .await
            .map_err(wrap)?;
        Ok(())
    }

    async fn load_messages(
        &self,
        conversation_id: &str,
    ) -> Result<Vec<MessageRecord>, PersistenceError> {
        let rows = sqlx::query(
            r#"SELECT id, role, content, created_at
               FROM messages
               WHERE conversation_id = ?
               ORDER BY id ASC"#,
        )
        .bind(conversation_id)
        .fetch_all(&self.pool)
        .await
        .map_err(wrap)?;
        let mut out = Vec::with_capacity(rows.len());
        for r in rows {
            let id: i64 = r.try_get("id").map_err(wrap)?;
            let role: String = r.try_get("role").map_err(wrap)?;
            let content: String = r.try_get("content").map_err(wrap)?;
            let ts: i64 = r.try_get("created_at").map_err(wrap)?;
            let created_at = Utc.timestamp_opt(ts, 0).single().ok_or_else(|| {
                PersistenceError::new(anyhow::anyhow!("invalid created_at: {ts}"))
            })?;
            out.push(MessageRecord {
                id,
                role,
                content,
                created_at,
            });
        }
        Ok(out)
    }

    async fn append_message(
        &self,
        conversation_id: &str,
        role: &str,
        content: &str,
    ) -> Result<i64, PersistenceError> {
        let now = Utc::now().timestamp();
        let result = sqlx::query(
            r#"INSERT INTO messages (conversation_id, role, content, tool_calls, created_at)
               VALUES (?, ?, ?, NULL, ?)"#,
        )
        .bind(conversation_id)
        .bind(role)
        .bind(content)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(wrap)?;
        Ok(result.last_insert_rowid())
    }

    async fn first_user_message(
        &self,
        conversation_id: &str,
    ) -> Result<Option<String>, PersistenceError> {
        let row = sqlx::query(
            r#"SELECT content FROM messages
               WHERE conversation_id = ? AND role = 'user'
               ORDER BY id ASC LIMIT 1"#,
        )
        .bind(conversation_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(wrap)?;
        Ok(row.and_then(|r| r.try_get::<String, _>("content").ok()))
    }

    async fn get_config(&self, key: &str) -> Result<Option<String>, PersistenceError> {
        let row = sqlx::query("SELECT value FROM config WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(wrap)?;
        Ok(row.and_then(|r| r.try_get::<Option<String>, _>("value").ok().flatten()))
    }

    async fn set_config(&self, key: &str, value: &str) -> Result<(), PersistenceError> {
        sqlx::query(
            r#"INSERT INTO config (key, value) VALUES (?, ?)
               ON CONFLICT(key) DO UPDATE SET value = excluded.value"#,
        )
        .bind(key)
        .bind(value)
        .execute(&self.pool)
        .await
        .map_err(wrap)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::open_pool_in_memory;

    #[tokio::test]
    async fn conversation_and_messages_round_trip() {
        let pool = open_pool_in_memory().await.unwrap();
        // Seed a workspace because conversations.workspace_id is a FK target;
        // but `ON DELETE SET NULL` makes the FK nullable, so an orphan id is
        // legal in P0. We seed for completeness.
        sqlx::query("INSERT INTO workspaces (id, name, created_at) VALUES ('ws-1', 'Default', 0)")
            .execute(&pool)
            .await
            .unwrap();
        let p = SqliteAiPersistence::new(pool);

        let conv = p.create_conversation("ws-1").await.unwrap();
        let conv_list = p.list_conversations("ws-1").await.unwrap();
        assert_eq!(conv_list.len(), 1);
        assert_eq!(conv_list[0].id, conv);

        let id1 = p.append_message(&conv, "user", "hello").await.unwrap();
        let id2 = p
            .append_message(&conv, "assistant", "world")
            .await
            .unwrap();
        assert!(id2 > id1);

        let msgs = p.load_messages(&conv).await.unwrap();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0].role, "user");
        assert_eq!(msgs[1].role, "assistant");

        let preview = p.first_user_message(&conv).await.unwrap();
        assert_eq!(preview.as_deref(), Some("hello"));

        p.delete_conversation(&conv).await.unwrap();
        assert!(p.list_conversations("ws-1").await.unwrap().is_empty());
        // Messages cascade.
        assert!(p.load_messages(&conv).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn config_round_trip() {
        let pool = open_pool_in_memory().await.unwrap();
        let p = SqliteAiPersistence::new(pool);
        assert!(p.get_config("k").await.unwrap().is_none());
        p.set_config("k", "v1").await.unwrap();
        assert_eq!(p.get_config("k").await.unwrap().as_deref(), Some("v1"));
        p.set_config("k", "v2").await.unwrap();
        assert_eq!(p.get_config("k").await.unwrap().as_deref(), Some("v2"));
    }
}
