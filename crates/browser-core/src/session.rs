use std::collections::HashMap;

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{StoreError, Tab, TabId, Workspace, WorkspaceId};

/// In-memory view of everything the frontend needs to restore on launch.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Session {
    pub workspaces: Vec<Workspace>,
    pub active_workspace: WorkspaceId,
    pub tabs_by_workspace: HashMap<WorkspaceId, Vec<Tab>>,
    pub active_tab: Option<TabId>,
}

impl Session {
    /// Build a `Session` with a single auto-created "Default" workspace and
    /// no tabs. Used when the store reports no persisted state.
    pub fn fresh() -> Self {
        let ws = Workspace::default_workspace();
        let id = ws.id;
        Self {
            workspaces: vec![ws],
            active_workspace: id,
            tabs_by_workspace: HashMap::from([(id, Vec::new())]),
            active_tab: None,
        }
    }

    pub fn tabs_of(&self, workspace: WorkspaceId) -> &[Tab] {
        self.tabs_by_workspace
            .get(&workspace)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn tabs_of_mut(&mut self, workspace: WorkspaceId) -> &mut Vec<Tab> {
        self.tabs_by_workspace.entry(workspace).or_default()
    }
}

/// Persistence boundary. The Tauri app holds an `Arc<dyn SessionStore>`,
/// concretely `hivemind-storage::SqliteSessionStore`.
#[async_trait]
pub trait SessionStore: Send + Sync {
    async fn load_session(&self) -> Result<Session, StoreError>;
    async fn save_tab(&self, tab: &Tab) -> Result<(), StoreError>;
    async fn remove_tab(&self, id: TabId) -> Result<(), StoreError>;
    async fn list_tabs(&self, workspace: WorkspaceId) -> Result<Vec<Tab>, StoreError>;
    async fn upsert_workspace(&self, ws: &Workspace) -> Result<(), StoreError>;
    async fn remove_workspace(&self, id: WorkspaceId) -> Result<(), StoreError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fresh_session_has_default_workspace() {
        let s = Session::fresh();
        assert_eq!(s.workspaces.len(), 1);
        assert_eq!(s.workspaces[0].name, Workspace::DEFAULT_NAME);
        assert_eq!(s.active_workspace, s.workspaces[0].id);
        assert!(s.tabs_of(s.active_workspace).is_empty());
    }
}
