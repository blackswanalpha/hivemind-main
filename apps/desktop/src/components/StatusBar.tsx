import { useTabs } from "../state/TabsContext";

export function StatusBar() {
  const { workspaces, activeWorkspace, tabs } = useTabs();
  const wsName =
    workspaces.find((w) => w.id === activeWorkspace)?.name ?? "—";

  return (
    <footer className="flex h-6 items-center justify-between border-t border-hivemind-border bg-hivemind-panel px-3 text-[11px] text-hivemind-mute">
      <div className="flex items-center gap-3">
        <span>workspace: {wsName}</span>
        <span>tabs: {tabs.length}</span>
      </div>
      <div className="flex items-center gap-3">
        <span title="Memory pipeline (Phase 1)">⚲ idle</span>
        <span title="Sync status (Phase 3)">⤴ off</span>
        <span title="Daily token budget (step 06+)">$ 0</span>
      </div>
    </footer>
  );
}
