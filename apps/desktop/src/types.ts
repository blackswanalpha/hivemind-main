// Mirrors crates/ipc-types/src/lib.rs. Keep in sync by hand for now; consider
// `ts-rs` codegen once the surface grows.

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

// ---------- AI / chat wire types (work00 step 07) ----------

export interface ConversationInfo {
  id: string;
  workspaceId: string;
  startedAt: number;
  preview?: string;
}

export interface MessageInfo {
  id: string;
  role: string;
  content: string;
  createdAt: number;
}

export interface AiProviderInfo {
  name: string;
  local: boolean;
  supportsTools: boolean;
  supportsEmbeddings: boolean;
  supportsPromptCaching: boolean;
  registered: boolean;
}

export interface AiSettingsPayload {
  provider: string;
  model?: string;
  /** `"prefer_local" | "prefer_cloud" | "explicit_name"` */
  policy: string;
}

export interface TestProviderResult {
  ok: boolean;
  latencyMs?: number;
  error?: string;
}

export interface UsageInfo {
  inputTokens: number;
  outputTokens: number;
  cacheCreationInputTokens?: number;
  cacheReadInputTokens?: number;
}

export interface ChatTokenPayload {
  conversationId: string;
  messageId: string;
  delta: string;
}

export interface ChatCompletePayload {
  conversationId: string;
  messageId: string;
  stopReason: string;
  usage: UsageInfo;
}

export interface ChatErrorPayload {
  conversationId: string;
  messageId: string;
  code: string;
  message: string;
}

// ---------- Frontend-only chat state ----------

export type ChatMessageStatus = "complete" | "streaming" | "error";

export interface ChatMessage {
  id: string;
  role: "user" | "assistant" | "system";
  content: string;
  status: ChatMessageStatus;
  error?: string;
  usage?: UsageInfo;
}
