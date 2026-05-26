import { useChat } from "../state/ChatContext";

function fmtTime(ts: number): string {
  if (!ts) return "—";
  const d = new Date(ts * 1000);
  return d.toLocaleString(undefined, {
    month: "short",
    day: "numeric",
    hour: "2-digit",
    minute: "2-digit",
  });
}

export function ConversationSwitcher() {
  const {
    conversations,
    active,
    newConversation,
    selectConversation,
    deleteConversation,
  } = useChat();

  const handleChange = (e: React.ChangeEvent<HTMLSelectElement>) => {
    const id = e.target.value;
    if (id) void selectConversation(id);
  };

  const handleDelete = () => {
    if (!active) return;
    const ok = window.confirm("Delete this conversation? This cannot be undone.");
    if (ok) void deleteConversation(active.id);
  };

  return (
    <div className="flex items-center gap-2 border-b border-hivemind-border px-3 py-2 text-[11px]">
      <select
        value={active?.id ?? ""}
        onChange={handleChange}
        className="flex-1 truncate rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1 text-xs text-hivemind-fg outline-none"
      >
        {conversations.length === 0 ? (
          <option value="">No conversations yet</option>
        ) : null}
        {conversations.map((c) => (
          <option key={c.id} value={c.id}>
            {c.preview ? c.preview.slice(0, 48) : `Conversation ${fmtTime(c.startedAt)}`}
          </option>
        ))}
      </select>
      <button
        type="button"
        onClick={() => void newConversation()}
        title="New conversation"
        className="rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1 text-hivemind-fg hover:border-hivemind-accent"
      >
        + new
      </button>
      <button
        type="button"
        onClick={handleDelete}
        disabled={!active}
        title="Delete this conversation"
        className="rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1 text-hivemind-mute hover:border-red-500/60 hover:text-red-300 disabled:opacity-50"
      >
        delete
      </button>
    </div>
  );
}
