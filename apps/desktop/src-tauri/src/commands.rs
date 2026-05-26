use std::str::FromStr;

use hivemind_browser_core::{
    repack_positions, Session, SessionStore, Tab, TabId, Workspace, WorkspaceId,
};
use hivemind_ipc_types::{SessionInfo, TabEventPayload, TabInfo, WorkspaceInfo};
use tauri::{AppHandle, Emitter, State};
use url::Url;

use crate::state::AppState;

fn err_to_str<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

fn tab_to_info(t: &Tab) -> TabInfo {
    TabInfo {
        id: t.id.to_string(),
        workspace_id: t.workspace_id.to_string(),
        url: t.url.to_string(),
        title: t.title.clone(),
        position: t.position,
        opened_at: t.opened_at.timestamp(),
        last_active_at: t.last_active_at.timestamp(),
        favicon: None,
    }
}

fn workspace_to_info(w: &Workspace) -> WorkspaceInfo {
    WorkspaceInfo {
        id: w.id.to_string(),
        name: w.name.clone(),
        created_at: w.created_at.timestamp(),
    }
}

fn session_to_info(s: &Session) -> SessionInfo {
    let mut tabs: Vec<TabInfo> = s
        .tabs_by_workspace
        .values()
        .flat_map(|v| v.iter().map(tab_to_info))
        .collect();
    tabs.sort_by(|a, b| {
        a.workspace_id
            .cmp(&b.workspace_id)
            .then(a.position.cmp(&b.position))
    });
    SessionInfo {
        workspaces: s.workspaces.iter().map(workspace_to_info).collect(),
        active_workspace: s.active_workspace.to_string(),
        tabs,
        active_tab: s.active_tab.map(|id| id.to_string()),
    }
}

/// Normalize what the user typed in the URL bar into a real URL:
/// - "https://x" stays
/// - "x.com" → "https://x.com"
/// - "search words" → defer to caller (we return an error here; the frontend
///   may decide to treat this as a search query in a later step).
fn coerce_url(input: &str) -> Result<Url, String> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err("url cannot be empty".into());
    }
    if let Ok(u) = Url::parse(trimmed) {
        return Ok(u);
    }
    let with_scheme = format!("https://{trimmed}");
    Url::parse(&with_scheme).map_err(|e| format!("invalid url `{input}`: {e}"))
}

fn title_from(url: &Url) -> String {
    url.host_str().unwrap_or("").to_string()
}

#[tauri::command]
pub fn ping(name: String) -> String {
    format!("hello {name}")
}

#[tauri::command]
pub async fn load_session(state: State<'_, AppState>) -> Result<SessionInfo, String> {
    let session = state.session.read().await;
    Ok(session_to_info(&session))
}

#[tauri::command]
pub async fn list_tabs(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<Vec<TabInfo>, String> {
    let ws = WorkspaceId::from_str(&workspace_id).map_err(err_to_str)?;
    let session = state.session.read().await;
    Ok(session
        .tabs_of(ws)
        .iter()
        .map(tab_to_info)
        .collect())
}

#[tauri::command]
pub async fn open_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    workspace_id: String,
    url: String,
) -> Result<TabInfo, String> {
    let ws = WorkspaceId::from_str(&workspace_id).map_err(err_to_str)?;
    let parsed = coerce_url(&url)?;

    let mut session = state.session.write().await;
    if !session.workspaces.iter().any(|w| w.id == ws) {
        return Err(format!("unknown workspace {workspace_id}"));
    }
    let existing = session.tabs_of(ws).to_vec();
    let mut tab = Tab::open(ws, parsed.clone(), &existing);
    tab.title = title_from(&tab.url);

    state.store.save_tab(&tab).await.map_err(err_to_str)?;
    let info = tab_to_info(&tab);
    session.tabs_of_mut(ws).push(tab.clone());
    session.active_tab = Some(tab.id);
    session.active_workspace = ws;
    drop(session);

    let _ = app.emit(
        "TabOpened",
        TabEventPayload {
            tab_id: info.id.clone(),
            workspace_id: Some(info.workspace_id.clone()),
            url: Some(info.url.clone()),
            title: Some(info.title.clone()),
        },
    );
    Ok(info)
}

#[tauri::command]
pub async fn close_tab(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<(), String> {
    let tid = TabId::from_str(&tab_id).map_err(err_to_str)?;
    let mut session = state.session.write().await;
    let mut workspace_holding: Option<WorkspaceId> = None;
    for (ws_id, tabs) in session.tabs_by_workspace.iter_mut() {
        if let Some(pos) = tabs.iter().position(|t| t.id == tid) {
            tabs.remove(pos);
            let _ = repack_positions(tabs);
            workspace_holding = Some(*ws_id);
            break;
        }
    }
    let Some(ws_id) = workspace_holding else {
        return Err(format!("tab {tab_id} not found"));
    };

    state.store.remove_tab(tid).await.map_err(err_to_str)?;
    // Re-persist the repacked tabs in this workspace.
    let tabs_now: Vec<Tab> = session.tabs_of(ws_id).to_vec();
    for t in &tabs_now {
        state.store.save_tab(t).await.map_err(err_to_str)?;
    }

    if session.active_tab == Some(tid) {
        session.active_tab = tabs_now.last().map(|t| t.id);
    }
    drop(session);

    let _ = app.emit(
        "TabClosed",
        TabEventPayload {
            tab_id: tid.to_string(),
            workspace_id: Some(ws_id.to_string()),
            url: None,
            title: None,
        },
    );
    Ok(())
}

#[tauri::command]
pub async fn set_active_tab(
    state: State<'_, AppState>,
    tab_id: String,
) -> Result<(), String> {
    let tid = TabId::from_str(&tab_id).map_err(err_to_str)?;
    let mut session = state.session.write().await;
    let mut found_ws: Option<WorkspaceId> = None;
    let mut touched: Option<Tab> = None;
    for (ws_id, tabs) in session.tabs_by_workspace.iter_mut() {
        if let Some(t) = tabs.iter_mut().find(|t| t.id == tid) {
            t.touch();
            found_ws = Some(*ws_id);
            touched = Some(t.clone());
            break;
        }
    }
    let Some(ws_id) = found_ws else {
        return Err(format!("tab {tab_id} not found"));
    };
    session.active_workspace = ws_id;
    session.active_tab = Some(tid);
    drop(session);
    if let Some(t) = touched {
        state.store.save_tab(&t).await.map_err(err_to_str)?;
    }
    Ok(())
}

#[tauri::command]
pub async fn navigate(
    app: AppHandle,
    state: State<'_, AppState>,
    tab_id: String,
    url: String,
) -> Result<TabInfo, String> {
    let tid = TabId::from_str(&tab_id).map_err(err_to_str)?;
    let parsed = coerce_url(&url)?;
    let mut session = state.session.write().await;
    let mut updated: Option<Tab> = None;
    for tabs in session.tabs_by_workspace.values_mut() {
        if let Some(t) = tabs.iter_mut().find(|t| t.id == tid) {
            t.url = parsed.clone();
            t.title = title_from(&parsed);
            t.touch();
            updated = Some(t.clone());
            break;
        }
    }
    let Some(tab) = updated else {
        return Err(format!("tab {tab_id} not found"));
    };
    state.store.save_tab(&tab).await.map_err(err_to_str)?;
    let info = tab_to_info(&tab);
    drop(session);

    let _ = app.emit(
        "TabNavigated",
        TabEventPayload {
            tab_id: info.id.clone(),
            workspace_id: Some(info.workspace_id.clone()),
            url: Some(info.url.clone()),
            title: Some(info.title.clone()),
        },
    );
    Ok(info)
}

#[tauri::command]
pub async fn switch_workspace(
    state: State<'_, AppState>,
    workspace_id: String,
) -> Result<(), String> {
    let ws = WorkspaceId::from_str(&workspace_id).map_err(err_to_str)?;
    let mut session = state.session.write().await;
    if !session.workspaces.iter().any(|w| w.id == ws) {
        return Err(format!("unknown workspace {workspace_id}"));
    }
    session.active_workspace = ws;
    session.active_tab = session
        .tabs_of(ws)
        .iter()
        .max_by_key(|t| t.last_active_at)
        .map(|t| t.id);
    Ok(())
}

