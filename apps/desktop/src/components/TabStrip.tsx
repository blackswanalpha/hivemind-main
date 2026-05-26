import { useTabs } from "../state/TabsContext";
import type { TabInfo } from "../types";

function tabLabel(t: TabInfo): string {
  if (t.title) return t.title;
  try {
    const u = new URL(t.url);
    return u.hostname || t.url;
  } catch {
    return t.url || "Untitled";
  }
}

export function TabStrip() {
  const { tabs, activeTab, openTab, closeTab, setActiveTab } = useTabs();

  return (
    <div className="flex h-9 items-stretch border-b border-hivemind-border bg-hivemind-panel">
      <div className="flex flex-1 items-stretch overflow-x-auto">
        {tabs.map((t) => {
          const active = t.id === activeTab;
          return (
            <div
              key={t.id}
              onClick={() => {
                setActiveTab(t.id).catch((e) => console.error(e));
              }}
              className={[
                "group flex min-w-[140px] max-w-[240px] items-center gap-2 px-3 text-xs cursor-pointer select-none border-r border-hivemind-border",
                active
                  ? "bg-hivemind-bg text-hivemind-fg border-b-2 border-b-hivemind-accent"
                  : "text-hivemind-mute hover:text-hivemind-fg",
              ].join(" ")}
              title={t.url}
            >
              <span className="flex-1 truncate">{tabLabel(t)}</span>
              <button
                aria-label="Close tab"
                className="opacity-60 hover:opacity-100"
                onClick={(e) => {
                  e.stopPropagation();
                  closeTab(t.id).catch((err) => console.error(err));
                }}
              >
                ×
              </button>
            </div>
          );
        })}
        <button
          aria-label="New tab"
          className="px-3 text-hivemind-mute hover:text-hivemind-fg"
          onClick={() => openTab().catch((err) => console.error(err))}
        >
          +
        </button>
      </div>
    </div>
  );
}
