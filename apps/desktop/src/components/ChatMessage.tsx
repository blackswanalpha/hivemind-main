import type { ChatMessage as ChatMessageT } from "../types";

interface Props {
  message: ChatMessageT;
}

export function ChatMessage({ message }: Props) {
  const isUser = message.role === "user";
  const wrapper = isUser ? "items-end" : "items-start";
  const bubbleBase =
    "max-w-[88%] whitespace-pre-wrap break-words rounded-md px-3 py-2 text-xs leading-relaxed";
  const bubbleColor = isUser
    ? "bg-hivemind-bg/60 text-hivemind-fg"
    : "bg-hivemind-accent/10 text-hivemind-fg";
  const errorColor =
    "border border-red-500/60 bg-red-500/10 text-red-300";
  const caret =
    message.status === "streaming" ? (
      <span className="ml-1 inline-block animate-pulse">▌</span>
    ) : null;

  return (
    <div className={`flex flex-col ${wrapper} gap-1`}>
      <div className="text-[10px] uppercase tracking-wide text-hivemind-mute">
        {message.role}
      </div>
      <div
        className={`${bubbleBase} ${
          message.status === "error" ? errorColor : bubbleColor
        }`}
      >
        {message.content || (
          <span className="text-hivemind-mute">…</span>
        )}
        {caret}
      </div>
      {message.status === "error" && message.error ? (
        <div className="text-[10px] text-red-300">{message.error}</div>
      ) : null}
    </div>
  );
}
