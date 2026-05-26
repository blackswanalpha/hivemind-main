import { useEffect, useState } from "react";

const STARTER_PROMPTS = [
  "Summarize the current tab",
  "What did I read about X this week?",
  "Open the docs for this library",
  "Compare these tabs",
];

export function Sidebar() {
  const [open, setOpen] = useState(true);

  useEffect(() => {
    const handler = (e: KeyboardEvent) => {
      // Ctrl+Shift+A toggles the sidebar (per docs/sitemap/04-ai-sidebar.md).
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

  return (
    <aside className="flex h-full w-[360px] shrink-0 flex-col border-l border-hivemind-border bg-hivemind-panel">
      <header className="flex items-center justify-between border-b border-hivemind-border px-3 py-2">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-hivemind-accent" />
          <span className="text-sm font-medium">AI assistant</span>
        </div>
        <button
          aria-label="Collapse sidebar"
          onClick={() => setOpen(false)}
          className="text-hivemind-mute hover:text-hivemind-fg"
        >
          ›
        </button>
      </header>

      <div className="flex flex-1 flex-col items-center justify-center px-6 text-center">
        <h3 className="mb-2 text-sm font-medium text-hivemind-fg">
          Hi — ask me about what you&apos;ve read,
          <br />
          or what to do next.
        </h3>
        <p className="mb-6 text-xs text-hivemind-mute">
          AI chat lands in step 08. These starters are placeholders.
        </p>
        <div className="flex w-full flex-col gap-2">
          {STARTER_PROMPTS.map((p) => (
            <button
              key={p}
              disabled
              className="w-full cursor-not-allowed rounded border border-hivemind-border bg-hivemind-bg/40 px-3 py-2 text-left text-xs text-hivemind-mute"
            >
              {p}
            </button>
          ))}
        </div>
      </div>

      <footer className="border-t border-hivemind-border p-3">
        <textarea
          disabled
          rows={2}
          placeholder="Ask anything… (disabled until step 08)"
          className="w-full resize-none rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1.5 text-xs text-hivemind-mute outline-none"
        />
      </footer>
    </aside>
  );
}
