// Mirrors crates/ipc-types/src/lib.rs. Keep in sync by hand for now; consider
// `ts-rs` codegen in step 05.

export interface TabInfo {
  id: string;
  workspaceId: string;
  url: string;
  title: string;
  position: number;
  openedAt: number;
  lastActiveAt: number;
  favicon?: string;
}

export interface WorkspaceInfo {
  id: string;
  name: string;
  createdAt: number;
}

export interface SessionInfo {
  workspaces: WorkspaceInfo[];
  activeWorkspace: string;
  tabs: TabInfo[];
  activeTab: string | null;
}

export interface AppStartedPayload {
  version: string;
}

export interface TabEventPayload {
  tabId: string;
  workspaceId?: string;
  url?: string;
  title?: string;
}
