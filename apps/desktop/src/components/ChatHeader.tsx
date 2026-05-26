import { useAiSettings } from "../state/AiSettingsContext";

interface Props {
  onOpenSettings: () => void;
  onCollapse: () => void;
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
        <button
          aria-label="Collapse sidebar"
          onClick={onCollapse}
          className="text-hivemind-mute hover:text-hivemind-fg"
        >
          ›
        </button>
      </div>
      <div className="flex flex-wrap items-center gap-2 text-[11px] text-hivemind-mute">
        <button
          type="button"
          onClick={onOpenSettings}
          className="rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-0.5 hover:border-hivemind-accent"
          title="Open AI settings"
        >
          {`Model: ${modelLabel} ▾`}
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
          ⚲ workspace
        </span>
      </div>
    </header>
  );
}
