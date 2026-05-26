import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AppStartedPayload,
  SessionInfo,
  TabEventPayload,
  TabInfo,
} from "./types";

// Tauri 2 converts JS camelCase arg keys to Rust snake_case automatically.

export const ipc = {
  ping(name: string): Promise<string> {
    return invoke("ping", { name });
  },

  loadSession(): Promise<SessionInfo> {
    return invoke("load_session");
  },

  listTabs(workspaceId: string): Promise<TabInfo[]> {
    return invoke("list_tabs", { workspaceId });
  },

  openTab(workspaceId: string, url: string): Promise<TabInfo> {
    return invoke("open_tab", { workspaceId, url });
  },

  closeTab(tabId: string): Promise<void> {
    return invoke("close_tab", { tabId });
  },

  setActiveTab(tabId: string): Promise<void> {
    return invoke("set_active_tab", { tabId });
  },

  navigate(tabId: string, url: string): Promise<TabInfo> {
    return invoke("navigate", { tabId, url });
  },

  switchWorkspace(workspaceId: string): Promise<void> {
    return invoke("switch_workspace", { workspaceId });
  },
};

export const events = {
  onAppStarted(cb: (p: AppStartedPayload) => void): Promise<UnlistenFn> {
    return listen<AppStartedPayload>("AppStarted", (e) => cb(e.payload));
  },
  onTabOpened(cb: (p: TabEventPayload) => void): Promise<UnlistenFn> {
    return listen<TabEventPayload>("TabOpened", (e) => cb(e.payload));
  },
  onTabClosed(cb: (p: TabEventPayload) => void): Promise<UnlistenFn> {
    return listen<TabEventPayload>("TabClosed", (e) => cb(e.payload));
  },
  onTabNavigated(cb: (p: TabEventPayload) => void): Promise<UnlistenFn> {
    return listen<TabEventPayload>("TabNavigated", (e) => cb(e.payload));
  },
};
