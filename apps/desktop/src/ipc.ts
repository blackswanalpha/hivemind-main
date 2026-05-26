import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

import type {
  AiProviderInfo,
  AiSettingsPayload,
  AppStartedPayload,
  ChatCompletePayload,
  ChatErrorPayload,
  ChatTokenPayload,
  ConversationInfo,
  MessageInfo,
  SessionInfo,
  TabEventPayload,
  TabInfo,
  TestProviderResult,
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

  // --- AI / chat ---

  createConversation(workspaceId: string): Promise<ConversationInfo> {
    return invoke("create_conversation", { workspaceId });
  },

  listConversations(workspaceId: string): Promise<ConversationInfo[]> {
    return invoke("list_conversations", { workspaceId });
  },

  loadMessages(conversationId: string): Promise<MessageInfo[]> {
    return invoke("load_messages", { conversationId });
  },

  deleteConversation(conversationId: string): Promise<void> {
    return invoke("delete_conversation", { conversationId });
  },

  sendMessage(
    conversationId: string,
    text: string,
    hint?: string,
  ): Promise<string> {
    return invoke("send_message", {
      conversationId,
      text,
      hint: hint ?? null,
    });
  },

  listProviders(): Promise<AiProviderInfo[]> {
    return invoke("list_providers");
  },

  getAiSettings(): Promise<AiSettingsPayload> {
    return invoke("get_ai_settings");
  },

  setAiSettings(settings: AiSettingsPayload): Promise<void> {
    return invoke("set_ai_settings", { settings });
  },

  testProvider(name: string): Promise<TestProviderResult> {
    return invoke("test_provider", { name });
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
  onChatToken(cb: (p: ChatTokenPayload) => void): Promise<UnlistenFn> {
    return listen<ChatTokenPayload>("hm:chat-token", (e) => cb(e.payload));
  },
  onChatComplete(cb: (p: ChatCompletePayload) => void): Promise<UnlistenFn> {
    return listen<ChatCompletePayload>("hm:chat-complete", (e) =>
      cb(e.payload),
    );
  },
  onChatError(cb: (p: ChatErrorPayload) => void): Promise<UnlistenFn> {
    return listen<ChatErrorPayload>("hm:chat-error", (e) => cb(e.payload));
  },
};
