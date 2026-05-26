use std::sync::Arc;

use hivemind_browser_core::{Session, SessionStore};
use hivemind_ipc_types::AppStartedPayload;
use hivemind_storage::{default_db_path, open_pool, SqliteSessionStore, StorageError};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::RwLock;

#[derive(Debug, thiserror::Error)]
pub enum InitError {
    #[error(transparent)]
    Storage(#[from] StorageError),
    #[error(transparent)]
    Store(#[from] hivemind_browser_core::StoreError),
}

pub struct AppState {
    pub store: Arc<SqliteSessionStore>,
    pub session: Arc<RwLock<Session>>,
}

impl AppState {
    pub async fn build() -> Result<Self, InitError> {
        let path = default_db_path()?;
        tracing::info!(?path, "opening hivemind database");
        let pool = open_pool(&path).await?;
        let store = Arc::new(SqliteSessionStore::new(pool));
        let session = store.load_session().await?;
        let tabs: usize = session
            .tabs_by_workspace
            .values()
            .map(Vec::len)
            .sum();
        tracing::info!(
            workspaces = session.workspaces.len(),
            tabs,
            "loaded session"
        );
        Ok(Self {
            store,
            session: Arc::new(RwLock::new(session)),
        })
    }
}

/// Called from the Tauri setup hook. Builds the app state, manages it, and
/// emits `AppStarted` once everything is wired.
pub async fn initialize(handle: &AppHandle) -> Result<(), InitError> {
    let state = AppState::build().await?;
    handle.manage(state);
    let payload = AppStartedPayload {
        version: env!("CARGO_PKG_VERSION").to_string(),
    };
    if let Err(err) = handle.emit("AppStarted", payload) {
        tracing::warn!(error = ?err, "failed to emit AppStarted");
    }
    Ok(())
}
