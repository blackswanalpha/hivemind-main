import { useAiSettings } from "../state/AiSettingsContext";
import { useChat } from "../state/ChatContext";
import { useTabs } from "../state/TabsContext";

export function StatusBar() {
  const { workspaces, activeWorkspace, tabs } = useTabs();
  const { providers, settings } = useAiSettings();
  const { lastUsage, error } = useChat();

  const wsName =
    workspaces.find((w) => w.id === activeWorkspace)?.name ?? "—";
  const providerLabel = settings?.provider ?? "—";
  const providerReg = providers.find((p) => p.name === providerLabel)?.registered;
  const providerGlyph = providerReg === false ? "⚠" : "●";

  const tokens = lastUsage
    ? `${lastUsage.inputTokens}/${lastUsage.outputTokens}`
    : "0/0";

  return (
    <footer className="flex h-6 items-center justify-between border-t border-hivemind-border bg-hivemind-panel px-3 text-[11px] text-hivemind-mute">
      <div className="flex items-center gap-3">
        <span>workspace: {wsName}</span>
        <span>tabs: {tabs.length}</span>
      </div>
      <div className="flex items-center gap-3">
        <span title="AI provider">{providerGlyph} {providerLabel}</span>
        <span title="Last turn tokens (in/out)">tok: {tokens}</span>
        <span title="Memory pipeline (Phase 1)">⚲ idle</span>
        <span title="Sync status (Phase 3)">⤴ off</span>
        {error ? (
          <span title={error} className="text-red-300">!</span>
        ) : null}
      </div>
    </footer>
  );
}
