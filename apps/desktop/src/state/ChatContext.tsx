import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useReducer,
  useRef,
  type ReactNode,
} from "react";

import { events, ipc } from "../ipc";
import type {
  ChatCompletePayload,
  ChatErrorPayload,
  ChatMessage,
  ChatTokenPayload,
  ConversationInfo,
  MessageInfo,
  UsageInfo,
} from "../types";
import { useTabs } from "./TabsContext";

type ConvState = {
  messages: ChatMessage[];
  streaming: boolean;
  error: string | null;
  lastUsage?: UsageInfo;
};

type ChatState = {
  conversations: ConversationInfo[];
  active: string | null;
  byConv: Record<string, ConvState>;
};

type Action =
  | { type: "SET_CONVERSATIONS"; convs: ConversationInfo[] }
  | { type: "SET_ACTIVE"; id: string | null }
  | { type: "REPLACE_MESSAGES"; id: string; messages: ChatMessage[] }
  | { type: "ADD_USER"; id: string; messageId: string; text: string }
  | { type: "BEGIN_STREAM"; id: string; assistantId: string }
  | { type: "TOKEN"; id: string; messageId: string; delta: string }
  | {
      type: "COMPLETE";
      id: string;
      messageId: string;
      usage: UsageInfo;
    }
  | { type: "ERROR"; id: string; messageId: string; error: string }
  | { type: "CLEAR_ERROR"; id: string };

function emptyConv(): ConvState {
  return { messages: [], streaming: false, error: null };
}

function reducer(state: ChatState, action: Action): ChatState {
  switch (action.type) {
    case "SET_CONVERSATIONS":
      return { ...state, conversations: action.convs };
    case "SET_ACTIVE":
      return { ...state, active: action.id };
    case "REPLACE_MESSAGES": {
      const existing = state.byConv[action.id] ?? emptyConv();
      return {
        ...state,
        byConv: {
          ...state.byConv,
          [action.id]: { ...existing, messages: action.messages },
        },
      };
    }
    case "ADD_USER": {
      const existing = state.byConv[action.id] ?? emptyConv();
      const userMsg: ChatMessage = {
        id: action.messageId,
        role: "user",
        content: action.text,
        status: "complete",
      };
      return {
        ...state,
        byConv: {
          ...state.byConv,
          [action.id]: {
            ...existing,
            messages: [...existing.messages, userMsg],
            error: null,
          },
        },
      };
    }
    case "BEGIN_STREAM": {
      const existing = state.byConv[action.id] ?? emptyConv();
      const assistantMsg: ChatMessage = {
        id: action.assistantId,
        role: "assistant",
        content: "",
        status: "streaming",
      };
      return {
        ...state,
        byConv: {
          ...state.byConv,
          [action.id]: {
            ...existing,
            messages: [...existing.messages, assistantMsg],
            streaming: true,
          },
        },
      };
    }
    case "TOKEN": {
      const existing = state.byConv[action.id];
      if (!existing) return state;
      const messages = existing.messages.map((m) =>
        m.id === action.messageId
          ? { ...m, content: m.content + action.delta }
          : m,
      );
      return {
        ...state,
        byConv: { ...state.byConv, [action.id]: { ...existing, messages } },
      };
    }
    case "COMPLETE": {
      const existing = state.byConv[action.id];
      if (!existing) return state;
      const messages = existing.messages.map((m) =>
        m.id === action.messageId
          ? { ...m, status: "complete" as const, usage: action.usage }
          : m,
      );
      return {
        ...state,
        byConv: {
          ...state.byConv,
          [action.id]: {
            ...existing,
            messages,
            streaming: false,
            lastUsage: action.usage,
          },
        },
      };
    }
    case "ERROR": {
      const existing = state.byConv[action.id];
      if (!existing) return state;
      const messages = existing.messages.map((m) =>
        m.id === action.messageId
          ? { ...m, status: "error" as const, error: action.error }
          : m,
      );
      return {
        ...state,
        byConv: {
          ...state.byConv,
          [action.id]: {
            ...existing,
            messages,
            streaming: false,
            error: action.error,
          },
        },
      };
    }
    case "CLEAR_ERROR": {
      const existing = state.byConv[action.id];
      if (!existing) return state;
      return {
        ...state,
        byConv: { ...state.byConv, [action.id]: { ...existing, error: null } },
      };
    }
  }
}

const INITIAL: ChatState = { conversations: [], active: null, byConv: {} };

interface ChatContextValue {
  conversations: ConversationInfo[];
  active: ConversationInfo | null;
  messages: ChatMessage[];
  streaming: boolean;
  error: string | null;
  lastUsage?: UsageInfo;
  newConversation: () => Promise<void>;
  selectConversation: (id: string) => Promise<void>;
  deleteConversation: (id: string) => Promise<void>;
  sendMessage: (text: string, hint?: string) => Promise<void>;
  clearError: () => void;
  refresh: () => Promise<void>;
}

const ChatContext = createContext<ChatContextValue | null>(null);

function messageInfoToChat(m: MessageInfo): ChatMessage {
  const role: ChatMessage["role"] =
    m.role === "assistant" || m.role === "system" ? m.role : "user";
  return { id: m.id, role, content: m.content, status: "complete" };
}

export function ChatProvider({ children }: { children: ReactNode }) {
  const [state, dispatch] = useReducer(reducer, INITIAL);
  const { activeWorkspace } = useTabs();
  const workspaceRef = useRef(activeWorkspace);
  workspaceRef.current = activeWorkspace;

  const refresh = useCallback(async () => {
    if (!activeWorkspace) return;
    const convs = await ipc.listConversations(activeWorkspace);
    dispatch({ type: "SET_CONVERSATIONS", convs });
    // Keep current active if still present; otherwise pick newest or null.
    const stillThere = convs.find((c) => c.id === state.active);
    if (!stillThere) {
      const next = convs[0]?.id ?? null;
      dispatch({ type: "SET_ACTIVE", id: next });
    }
  }, [activeWorkspace, state.active]);

  useEffect(() => {
    refresh().catch((err) => console.error("listConversations failed", err));
  }, [activeWorkspace, refresh]);

  // Subscribe to streaming events once per provider lifetime.
  useEffect(() => {
    const subs: Promise<() => void>[] = [
      events.onChatToken((p: ChatTokenPayload) => {
        dispatch({
          type: "TOKEN",
          id: p.conversationId,
          messageId: p.messageId,
          delta: p.delta,
        });
      }),
      events.onChatComplete((p: ChatCompletePayload) => {
        dispatch({
          type: "COMPLETE",
          id: p.conversationId,
          messageId: p.messageId,
          usage: p.usage,
        });
      }),
      events.onChatError((p: ChatErrorPayload) => {
        dispatch({
          type: "ERROR",
          id: p.conversationId,
          messageId: p.messageId,
          error: p.message ?? p.code ?? "error",
        });
      }),
    ];
    return () => {
      subs.forEach((p) =>
        p.then((un) => un()).catch((err) => console.error(err)),
      );
    };
  }, []);

  const newConversation = useCallback(async () => {
    if (!activeWorkspace) return;
    const info = await ipc.createConversation(activeWorkspace);
    dispatch({
      type: "SET_CONVERSATIONS",
      convs: [info, ...state.conversations],
    });
    dispatch({ type: "SET_ACTIVE", id: info.id });
    dispatch({ type: "REPLACE_MESSAGES", id: info.id, messages: [] });
  }, [activeWorkspace, state.conversations]);

  const selectConversation = useCallback(async (id: string) => {
    dispatch({ type: "SET_ACTIVE", id });
    const msgs = await ipc.loadMessages(id);
    dispatch({
      type: "REPLACE_MESSAGES",
      id,
      messages: msgs.map(messageInfoToChat),
    });
  }, []);

  const deleteConversation = useCallback(
    async (id: string) => {
      await ipc.deleteConversation(id);
      const remaining = state.conversations.filter((c) => c.id !== id);
      dispatch({ type: "SET_CONVERSATIONS", convs: remaining });
      if (state.active === id) {
        const next = remaining[0]?.id ?? null;
        dispatch({ type: "SET_ACTIVE", id: next });
      }
    },
    [state.active, state.conversations],
  );

  const sendMessage = useCallback(
    async (text: string, hint?: string) => {
      let convId = state.active;
      if (!convId) {
        if (!activeWorkspace) return;
        const info = await ipc.createConversation(activeWorkspace);
        dispatch({
          type: "SET_CONVERSATIONS",
          convs: [info, ...state.conversations],
        });
        dispatch({ type: "SET_ACTIVE", id: info.id });
        dispatch({ type: "REPLACE_MESSAGES", id: info.id, messages: [] });
        convId = info.id;
      }
      const fingerprint = `tmp-${Math.random().toString(36).slice(2, 10)}`;
      dispatch({
        type: "ADD_USER",
        id: convId,
        messageId: fingerprint,
        text,
      });
      try {
        const assistantId = await ipc.sendMessage(convId, text, hint);
        dispatch({ type: "BEGIN_STREAM", id: convId, assistantId });
      } catch (err) {
        dispatch({
          type: "ERROR",
          id: convId,
          messageId: fingerprint,
          error: String(err),
        });
      }
    },
    [activeWorkspace, state.active, state.conversations],
  );

  const clearError = useCallback(() => {
    if (state.active) dispatch({ type: "CLEAR_ERROR", id: state.active });
  }, [state.active]);

  const active = useMemo(
    () => state.conversations.find((c) => c.id === state.active) ?? null,
    [state.active, state.conversations],
  );

  const convState = state.active ? state.byConv[state.active] : undefined;

  const value: ChatContextValue = {
    conversations: state.conversations,
    active,
    messages: convState?.messages ?? [],
    streaming: convState?.streaming ?? false,
    error: convState?.error ?? null,
    lastUsage: convState?.lastUsage,
    newConversation,
    selectConversation,
    deleteConversation,
    sendMessage,
    clearError,
    refresh,
  };

  return <ChatContext.Provider value={value}>{children}</ChatContext.Provider>;
}

export function useChat(): ChatContextValue {
  const v = useContext(ChatContext);
  if (!v) throw new Error("useChat must be used within <ChatProvider>");
  return v;
}
