import { useEffect, useState } from "react";

import { useChat } from "../state/ChatContext";
import { ChatComposer } from "./ChatComposer";
import { ChatHeader } from "./ChatHeader";
import { ConversationSwitcher } from "./ConversationSwitcher";
import { MessageList } from "./MessageList";
import { SettingsDrawer } from "./SettingsDrawer";
import { StarterPrompts } from "./StarterPrompts";

export function Sidebar() {
  const [open, setOpen] = useState(true);
  const [showSettings, setShowSettings] = useState(false);
  const [composerSeed, setComposerSeed] = useState<string | undefined>(undefined);
  const { messages, streaming, error, sendMessage, clearError } = useChat();

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      if (e.ctrlKey && e.shiftKey && (e.key === "A" || e.key === "a")) {
        e.preventDefault();
        setOpen((v) => !v);
      }
    };
    window.addEventListener("keydown", handler);
    return () => window.removeEventListener("keydown", handler);
  }, []);

  if (!open) {
    return (
      <button
        title="Open AI sidebar (Ctrl+Shift+A)"
        aria-label="Open AI sidebar"
        onClick={() => setOpen(true)}
        className="absolute right-0 top-1/2 h-12 w-5 -translate-y-1/2 rounded-l border border-r-0 border-hivemind-border bg-hivemind-panel text-hivemind-mute hover:text-hivemind-fg"
      >
        ‹
      </button>
    );
  }

  const handleSend = async (text: string) => {
    setComposerSeed(undefined);
    await sendMessage(text);
  };

  return (
    <aside className="relative flex h-full w-[360px] shrink-0 flex-col border-l border-hivemind-border bg-hivemind-panel">
      <ChatHeader
        onOpenSettings={() => setShowSettings(true)}
        onCollapse={() => setOpen(false)}
      />
      <ConversationSwitcher />
      {messages.length === 0 ? (
        <StarterPrompts onPick={(t) => setComposerSeed(t)} />
      ) : (
        <MessageList messages={messages} />
      )}
      {error ? (
        <div className="mx-3 mb-2 flex items-center justify-between rounded border border-red-500/60 bg-red-500/10 px-3 py-1.5 text-[11px] text-red-300">
          <span className="truncate" title={error}>
            {error}
          </span>
          <button
            onClick={clearError}
            aria-label="Dismiss error"
            className="ml-2 shrink-0 text-red-200 hover:text-red-100"
          >
            ✕
          </button>
        </div>
      ) : null}
      <footer className="border-t border-hivemind-border p-3">
        <ChatComposer
          disabled={streaming}
          initialText={composerSeed}
          onSend={handleSend}
        />
      </footer>
      <SettingsDrawer
        open={showSettings}
        onClose={() => setShowSettings(false)}
      />
    </aside>
  );
}
