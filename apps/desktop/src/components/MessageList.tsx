import { useEffect, useRef } from "react";

import type { ChatMessage as ChatMessageT } from "../types";
import { ChatMessage } from "./ChatMessage";

interface Props {
  messages: ChatMessageT[];
}

const PIN_THRESHOLD_PX = 32;

export function MessageList({ messages }: Props) {
  const scrollerRef = useRef<HTMLDivElement | null>(null);
  const wasPinnedRef = useRef(true);

  useEffect(() => {
    const el = scrollerRef.current;
    if (!el) return;
    if (wasPinnedRef.current) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages]);

  const handleScroll = () => {
    const el = scrollerRef.current;
    if (!el) return;
    wasPinnedRef.current =
      el.scrollHeight - el.scrollTop - el.clientHeight < PIN_THRESHOLD_PX;
  };

  return (
    <div
      ref={scrollerRef}
      onScroll={handleScroll}
      className="flex flex-1 flex-col gap-3 overflow-y-auto px-3 py-3"
    >
      {messages.map((m) => (
        <ChatMessage key={m.id} message={m} />
      ))}
    </div>
  );
}
