import { useAiSettings } from "../state/AiSettingsContext";

interface Props {
  onOpenSettings: () => void;
  onCollapse: () => void;
}

function GearIcon({ className }: { className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      width="14"
      height="14"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
      strokeLinejoin="round"
      className={className}
      aria-hidden="true"
    >
      <circle cx="12" cy="12" r="3" />
      <path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 1 1-2.83 2.83l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-4 0v-.09a1.65 1.65 0 0 0-1-1.51 1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 1 1-2.83-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1 0-4h.09a1.65 1.65 0 0 0 1.51-1 1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 1 1 2.83-2.83l.06.06a1.65 1.65 0 0 0 1.82.33h.01a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 4 0v.09a1.65 1.65 0 0 0 1 1.51h.01a1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 1 1 2.83 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82v.01a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 0 4h-.09a1.65 1.65 0 0 0-1.51 1z" />
    </svg>
  );
}

export function ChatHeader({ onOpenSettings, onCollapse }: Props) {
  const { providers, settings } = useAiSettings();
  const active = providers.find((p) => p.name === settings?.provider);
  const modelLabel = settings?.model ?? (active?.name ?? "—");
  const toolsLabel = active?.supportsTools ? "tools: on" : "tools: off";

  return (
    <header className="flex flex-col gap-2 border-b border-hivemind-border px-3 py-2">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <span className="h-2 w-2 rounded-full bg-hivemind-accent" />
          <span className="text-sm font-medium">AI assistant</span>
        </div>
        <div className="flex items-center gap-1">
          <button
            type="button"
            aria-label="Open AI settings"
            title="AI settings (provider, model, policy)"
            onClick={onOpenSettings}
            className="inline-flex items-center gap-1 rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-0.5 text-xs text-hivemind-fg hover:border-hivemind-accent hover:text-hivemind-accent"
          >
            <GearIcon />
            <span>Settings</span>
          </button>
          <button
            aria-label="Collapse sidebar"
            onClick={onCollapse}
            className="px-1 text-hivemind-mute hover:text-hivemind-fg"
          >
            ›
          </button>
        </div>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-[11px] text-hivemind-mute">
        <button
          type="button"
          onClick={onOpenSettings}
          className="inline-flex items-center gap-1 rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-0.5 hover:border-hivemind-accent"
          title="Open AI settings"
        >
          <GearIcon />
          <span>{`Model: ${modelLabel} ▾`}</span>
        </button>
        <span
          className={`rounded border border-hivemind-border px-2 py-0.5 ${
            active?.supportsTools ? "text-hivemind-fg" : ""
          }`}
          title="Tool availability (read-only in P0)"
        >
          {toolsLabel}
        </span>
        <span
          className="rounded border border-hivemind-border px-2 py-0.5"
          title="Memory scope (Phase 1)"
        >
          memory: workspace
        </span>
      </div>
    </header>
  );
}
