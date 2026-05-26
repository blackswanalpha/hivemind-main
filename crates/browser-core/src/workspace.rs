use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::WorkspaceId;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Workspace {
    pub id: WorkspaceId,
    pub name: String,
    pub created_at: DateTime<Utc>,
}

impl Workspace {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: WorkspaceId::new(),
            name: name.into(),
            created_at: Utc::now(),
        }
    }

    pub const DEFAULT_NAME: &'static str = "Default";

    pub fn default_workspace() -> Self {
        Self::new(Self::DEFAULT_NAME)
    }
}
