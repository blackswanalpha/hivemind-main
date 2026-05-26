import { useEffect, useRef, useState } from "react";

import { useTabs } from "../state/TabsContext";

function findActive(tabs: ReturnType<typeof useTabs>["tabs"], id: string | null) {
  if (!id) return null;
  return tabs.find((t) => t.id === id) ?? null;
}

export function AddressBar() {
  const { tabs, activeTab, navigate } = useTabs();
  const active = findActive(tabs, activeTab);
  const [value, setValue] = useState(active?.url ?? "");
  const inputRef = useRef<HTMLInputElement | null>(null);

  useEffect(() => {
    setValue(active?.url ?? "");
  }, [active?.id, active?.url]);

  if (!active) {
    return (
      <div className="flex h-10 items-center border-b border-hivemind-border bg-hivemind-bg px-3">
        <input
          aria-label="URL"
          disabled
          placeholder="Open a tab to navigate"
          className="w-full rounded bg-hivemind-panel/40 px-3 py-1.5 text-sm text-hivemind-mute outline-none"
        />
      </div>
    );
  }

  const submit = async () => {
    const trimmed = value.trim();
    if (!trimmed) return;
    try {
      await navigate(active.id, trimmed);
    } catch (err) {
      console.error("navigate failed", err);
    }
  };

  return (
    <div className="flex h-10 items-center gap-2 border-b border-hivemind-border bg-hivemind-bg px-3">
      <input
        ref={inputRef}
        aria-label="URL"
        value={value}
        onChange={(e) => setValue(e.target.value)}
        onKeyDown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            submit();
          } else if (e.key === "Escape") {
            setValue(active.url);
            inputRef.current?.blur();
          }
        }}
        className="w-full rounded border border-hivemind-border bg-hivemind-panel px-3 py-1.5 text-sm text-hivemind-fg outline-none focus:border-hivemind-accent"
        placeholder="https://…"
        spellCheck={false}
      />
    </div>
  );
}
