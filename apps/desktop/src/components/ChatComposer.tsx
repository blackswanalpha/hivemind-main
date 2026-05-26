import { useEffect, useRef, useState } from "react";

interface Props {
  disabled: boolean;
  initialText?: string;
  onSend: (text: string) => Promise<void> | void;
}

export function ChatComposer({ disabled, initialText, onSend }: Props) {
  const [text, setText] = useState(initialText ?? "");
  const ref = useRef<HTMLTextAreaElement | null>(null);

  useEffect(() => {
    if (initialText !== undefined) {
      setText(initialText);
      // Move caret to end on next paint.
      requestAnimationFrame(() => {
        const el = ref.current;
        if (el) {
          el.focus();
          el.setSelectionRange(el.value.length, el.value.length);
        }
      });
    }
  }, [initialText]);

  useEffect(() => {
    if (!disabled) {
      ref.current?.focus();
    }
  }, [disabled]);

  const trimmed = text.trim();
  const canSend = !disabled && trimmed.length > 0;

  const submit = async () => {
    if (!canSend) return;
    const payload = trimmed;
    setText("");
    await onSend(payload);
  };

  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter" && !e.shiftKey) {
      e.preventDefault();
      void submit();
    }
  };

  return (
    <form
      onSubmit={(e) => {
        e.preventDefault();
        void submit();
      }}
      className="flex flex-col gap-2"
    >
      <textarea
        ref={ref}
        rows={2}
        value={text}
        onChange={(e) => setText(e.target.value)}
        onKeyDown={handleKeyDown}
        disabled={disabled}
        placeholder={disabled ? "Streaming…" : "Ask HiveMind…"}
        className="w-full resize-none rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1.5 text-xs text-hivemind-fg outline-none placeholder:text-hivemind-mute focus:border-hivemind-accent disabled:cursor-not-allowed disabled:opacity-60"
      />
      <div className="flex items-center justify-between text-[10px] text-hivemind-mute">
        <span>⏎ send · Shift+⏎ newline</span>
        <button
          type="submit"
          disabled={!canSend}
          className="rounded border border-hivemind-border bg-hivemind-accent/20 px-3 py-1 text-[11px] text-hivemind-fg hover:bg-hivemind-accent/30 disabled:cursor-not-allowed disabled:opacity-50"
        >
          Send
        </button>
      </div>
    </form>
  );
}
