use std::collections::HashMap;
use std::str::FromStr;

use async_trait::async_trait;
use chrono::{DateTime, TimeZone, Utc};
use hivemind_browser_core::{
    Session, SessionStore, StoreError, Tab, TabId, Workspace, WorkspaceId,
};
use sqlx::{Row, SqlitePool};
use url::Url;

#[derive(Clone)]
pub struct SqliteSessionStore {
    pool: SqlitePool,
}

impl SqliteSessionStore {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }
}

fn ts_to_dt(ts: i64) -> Result<DateTime<Utc>, StoreError> {
    Utc.timestamp_opt(ts, 0)
        .single()
        .ok_or_else(|| StoreError::invalid(format!("invalid timestamp {ts}")))
}

fn dt_to_ts(dt: DateTime<Utc>) -> i64 {
    dt.timestamp()
}

fn parse_url(s: &str) -> Result<Url, StoreError> {
    Url::parse(s).map_err(|e| StoreError::invalid(format!("invalid url `{s}`: {e}")))
}

#[async_trait]
impl SessionStore for SqliteSessionStore {
    async fn load_session(&self) -> Result<Session, StoreError> {
        let ws_rows = sqlx::query("SELECT id, name, created_at FROM workspaces ORDER BY created_at ASC")
            .fetch_all(&self.pool)
            .await
            .map_err(StoreError::backend)?;

        if ws_rows.is_empty() {
            let session = Session::fresh();
            // Persist the auto-created default workspace so subsequent boots
            // see consistent state.
            for ws in &session.workspaces {
                self.upsert_workspace(ws).await?;
            }
            return Ok(session);
        }

        let mut workspaces = Vec::with_capacity(ws_rows.len());
        let mut tabs_by_workspace: HashMap<WorkspaceId, Vec<Tab>> = HashMap::new();

        for row in ws_rows {
            let id_str: String = row.try_get("id").map_err(StoreError::backend)?;
            let name: String = row.try_get("name").map_err(StoreError::backend)?;
            let created_at: i64 = row.try_get("created_at").map_err(StoreError::backend)?;
            let id = WorkspaceId::from_str(&id_str)?;
            workspaces.push(Workspace {
                id,
                name,
                created_at: ts_to_dt(created_at)?,
            });
        }

        for ws in &workspaces {
            let tabs = self.list_tabs(ws.id).await?;
            tabs_by_workspace.insert(ws.id, tabs);
        }

        // Active workspace: most-recently-active tab decides; fallback to first ws.
        let mut active_workspace = workspaces[0].id;
        let mut active_tab: Option<TabId> = None;
        let mut newest_active: i64 = i64::MIN;
        for (ws_id, tabs) in &tabs_by_workspace {
            for t in tabs {
                let ts = dt_to_ts(t.last_active_at);
                if ts > newest_active {
                    newest_active = ts;
                    active_workspace = *ws_id;
                    active_tab = Some(t.id);
                }
            }
        }

        Ok(Session {
            workspaces,
            active_workspace,
            tabs_by_workspace,
            active_tab,
        })
    }

    async fn save_tab(&self, tab: &Tab) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO tabs (id, workspace_id, url, title, favicon, position, opened_at, last_active_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET
                workspace_id   = excluded.workspace_id,
                url            = excluded.url,
                title          = excluded.title,
                favicon        = excluded.favicon,
                position       = excluded.position,
                last_active_at = excluded.last_active_at
            "#,
        )
        .bind(tab.id.to_string())
        .bind(tab.workspace_id.to_string())
        .bind(tab.url.as_str())
        .bind(&tab.title)
        .bind(tab.favicon.as_deref())
        .bind(tab.position as i64)
        .bind(dt_to_ts(tab.opened_at))
        .bind(dt_to_ts(tab.last_active_at))
        .execute(&self.pool)
        .await
        .map_err(StoreError::backend)?;
        Ok(())
    }

    async fn remove_tab(&self, id: TabId) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM tabs WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(StoreError::backend)?;
        Ok(())
    }

    async fn list_tabs(&self, workspace: WorkspaceId) -> Result<Vec<Tab>, StoreError> {
        let rows = sqlx::query(
            r#"
            SELECT id, workspace_id, url, title, favicon, position, opened_at, last_active_at
            FROM tabs
            WHERE workspace_id = ?
            ORDER BY position ASC
            "#,
        )
        .bind(workspace.to_string())
        .fetch_all(&self.pool)
        .await
        .map_err(StoreError::backend)?;

        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let id_str: String = row.try_get("id").map_err(StoreError::backend)?;
            let ws_str: String = row.try_get("workspace_id").map_err(StoreError::backend)?;
            let url_str: String = row.try_get("url").map_err(StoreError::backend)?;
            let title: String = row.try_get("title").map_err(StoreError::backend)?;
            let favicon: Option<Vec<u8>> = row.try_get("favicon").ok();
            let position: i64 = row.try_get("position").map_err(StoreError::backend)?;
            let opened_at: i64 = row.try_get("opened_at").map_err(StoreError::backend)?;
            let last_active_at: i64 = row.try_get("last_active_at").map_err(StoreError::backend)?;

            out.push(Tab {
                id: TabId::from_str(&id_str)?,
                workspace_id: WorkspaceId::from_str(&ws_str)?,
                url: parse_url(&url_str)?,
                title,
                favicon,
                position: position.max(0) as u32,
                opened_at: ts_to_dt(opened_at)?,
                last_active_at: ts_to_dt(last_active_at)?,
            });
        }
        Ok(out)
    }

    async fn upsert_workspace(&self, ws: &Workspace) -> Result<(), StoreError> {
        sqlx::query(
            r#"
            INSERT INTO workspaces (id, name, created_at)
            VALUES (?, ?, ?)
            ON CONFLICT(id) DO UPDATE SET name = excluded.name
            "#,
        )
        .bind(ws.id.to_string())
        .bind(&ws.name)
        .bind(dt_to_ts(ws.created_at))
        .execute(&self.pool)
        .await
        .map_err(StoreError::backend)?;
        Ok(())
    }

    async fn remove_workspace(&self, id: WorkspaceId) -> Result<(), StoreError> {
        sqlx::query("DELETE FROM workspaces WHERE id = ?")
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(StoreError::backend)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pool::open_pool_in_memory;
    use url::Url;

    async fn store() -> SqliteSessionStore {
        let pool = open_pool_in_memory().await.expect("memory pool");
        SqliteSessionStore::new(pool)
    }

    #[tokio::test]
    async fn empty_load_creates_default_workspace_and_persists() {
        let s = store().await;
        let session = s.load_session().await.expect("load");
        assert_eq!(session.workspaces.len(), 1);
        assert_eq!(session.workspaces[0].name, Workspace::DEFAULT_NAME);

        // Second load should see the persisted workspace and not create another.
        let session2 = s.load_session().await.expect("load2");
        assert_eq!(session2.workspaces.len(), 1);
        assert_eq!(session2.workspaces[0].id, session.workspaces[0].id);
    }

    #[tokio::test]
    async fn tab_roundtrip_and_ordering() {
        let s = store().await;
        let session = s.load_session().await.expect("load");
        let ws = session.workspaces[0].id;

        let mut existing: Vec<Tab> = vec![];
        for url in ["https://a.example/", "https://b.example/", "https://c.example/"] {
            let t = Tab::open(ws, Url::parse(url).unwrap(), &existing);
            s.save_tab(&t).await.expect("save");
            existing.push(t);
        }

        let tabs = s.list_tabs(ws).await.expect("list");
        assert_eq!(tabs.len(), 3);
        assert_eq!(tabs[0].url.as_str(), "https://a.example/");
        assert_eq!(tabs[2].url.as_str(), "https://c.example/");
        for (i, t) in tabs.iter().enumerate() {
            assert_eq!(t.position, i as u32);
        }

        s.remove_tab(tabs[1].id).await.expect("remove");
        let tabs = s.list_tabs(ws).await.expect("list2");
        assert_eq!(tabs.len(), 2);
    }
}
