use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use url::Url;

use crate::{TabId, WorkspaceId};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct Tab {
    pub id: TabId,
    pub workspace_id: WorkspaceId,
    pub url: Url,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub favicon: Option<Vec<u8>>,
    pub position: u32,
    pub opened_at: DateTime<Utc>,
    pub last_active_at: DateTime<Utc>,
}

impl Tab {
    /// Build a new tab for `workspace_id` whose `position` is one past the
    /// highest position currently in `existing` (or 0 if empty).
    pub fn open(workspace_id: WorkspaceId, url: Url, existing: &[Tab]) -> Self {
        let position = existing.iter().map(|t| t.position).max().map_or(0, |p| p + 1);
        let now = Utc::now();
        Self {
            id: TabId::new(),
            workspace_id,
            url,
            title: String::new(),
            favicon: None,
            position,
            opened_at: now,
            last_active_at: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_active_at = Utc::now();
    }
}

/// Re-pack tab positions to a dense `0..N` range in their current order.
/// Mutates in place; returns `true` if any position changed (caller can use
/// this to decide whether to persist).
pub fn repack_positions(tabs: &mut [Tab]) -> bool {
    let mut changed = false;
    for (idx, tab) in tabs.iter_mut().enumerate() {
        let expected = idx as u32;
        if tab.position != expected {
            tab.position = expected;
            changed = true;
        }
    }
    changed
}

/// Move the tab identified by `tab_id` to `new_position` (clamped). Returns
/// the new list ordering with positions repacked.
pub fn move_tab(mut tabs: Vec<Tab>, tab_id: TabId, new_position: u32) -> Vec<Tab> {
    tabs.sort_by_key(|t| t.position);
    let Some(current_idx) = tabs.iter().position(|t| t.id == tab_id) else {
        return tabs;
    };
    let target = (new_position as usize).min(tabs.len().saturating_sub(1));
    if current_idx != target {
        let item = tabs.remove(current_idx);
        tabs.insert(target, item);
    }
    let _ = repack_positions(&mut tabs);
    tabs
}

#[cfg(test)]
mod tests {
    use super::*;

    fn url(s: &str) -> Url {
        Url::parse(s).expect("test url")
    }

    fn mk(ws: WorkspaceId, u: &str, existing: &[Tab]) -> Tab {
        Tab::open(ws, url(u), existing)
    }

    #[test]
    fn open_assigns_increasing_position() {
        let ws = WorkspaceId::new();
        let mut tabs = vec![];
        let t1 = mk(ws, "https://a.example", &tabs);
        tabs.push(t1.clone());
        let t2 = mk(ws, "https://b.example", &tabs);
        tabs.push(t2.clone());
        assert_eq!(t1.position, 0);
        assert_eq!(t2.position, 1);
    }

    #[test]
    fn move_tab_reorders_and_repacks() {
        let ws = WorkspaceId::new();
        let mut tabs = vec![];
        for url in ["https://a", "https://b", "https://c"] {
            let t = mk(ws, url, &tabs);
            tabs.push(t);
        }
        let third_id = tabs[2].id;
        let result = move_tab(tabs, third_id, 0);
        assert_eq!(result[0].id, third_id);
        assert_eq!(result[0].position, 0);
        assert_eq!(result[1].position, 1);
        assert_eq!(result[2].position, 2);
    }

    #[test]
    fn move_to_oob_clamps() {
        let ws = WorkspaceId::new();
        let mut tabs = vec![];
        for url in ["https://a", "https://b"] {
            let t = mk(ws, url, &tabs);
            tabs.push(t);
        }
        let first_id = tabs[0].id;
        let result = move_tab(tabs, first_id, 99);
        assert_eq!(result.last().unwrap().id, first_id);
    }
}
