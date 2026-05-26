import { useState } from "react";

import { useTabs } from "../state/TabsContext";

const RECENT_SUGGESTIONS = [
  "https://example.com",
  "https://www.rust-lang.org",
  "https://tauri.app",
];

export function EmptyWorkspace() {
  const { workspaces, activeWorkspace, openTab } = useTabs();
  const [value, setValue] = useState("");
  const wsName =
    workspaces.find((w) => w.id === activeWorkspace)?.name ?? "Workspace";

  const go = async (url: string) => {
    const trimmed = url.trim();
    if (!trimmed) return;
    try {
      await openTab(trimmed);
    } catch (e) {
      console.error("openTab failed", e);
    }
  };

  return (
    <div className="flex flex-1 items-center justify-center bg-hivemind-bg">
      <div className="w-full max-w-[560px] px-6">
        <h1 className="mb-4 text-center text-lg font-medium text-hivemind-fg">
          {wsName}
        </h1>
        <input
          autoFocus
          value={value}
          onChange={(e) => setValue(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              go(value);
            }
          }}
          placeholder="Type a URL or press + to open a new tab"
          className="mb-6 w-full rounded border border-hivemind-border bg-hivemind-panel px-3 py-2 text-sm text-hivemind-fg outline-none focus:border-hivemind-accent"
        />
        <p className="mb-2 text-xs uppercase tracking-wide text-hivemind-mute">
          Suggested
        </p>
        <ul className="flex flex-col gap-1">
          {RECENT_SUGGESTIONS.map((u) => (
            <li key={u}>
              <button
                onClick={() => go(u)}
                className="w-full rounded px-3 py-1.5 text-left text-sm text-hivemind-fg hover:bg-hivemind-panel"
              >
                {u}
              </button>
            </li>
          ))}
        </ul>
      </div>
    </div>
  );
}
