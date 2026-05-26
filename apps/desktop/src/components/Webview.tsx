import { useTabs } from "../state/TabsContext";

// Walking-skeleton trade-off: instead of one OS webview per tab (Tauri 2's
// multi-webview-windows API), we render an <iframe> per tab and hide all but
// the active one with `display`. The behavior is correct enough for step 04
// (URL loads, navigation works, persistence verifies). Real per-tab OS
// webviews are queued for step 08 / Phase 1 (see README "Known shortcuts").

export function Webview() {
  const { tabs, activeTab } = useTabs();

  if (tabs.length === 0) {
    return null;
  }

  return (
    <div className="relative flex-1 bg-white">
      {tabs.map((t) => {
        const active = t.id === activeTab;
        // about:blank in an iframe shows nothing useful, so we render a
        // simple placeholder for fresh tabs.
        const isBlank = t.url === "about:blank";
        return (
          <div
            key={t.id}
            className="absolute inset-0"
            style={{ display: active ? "block" : "none" }}
          >
            {isBlank ? (
              <div className="flex h-full flex-col items-center justify-center bg-hivemind-bg text-hivemind-mute">
                <h2 className="mb-2 text-xl text-hivemind-fg">New Tab</h2>
                <p className="text-sm">Type a URL in the address bar above.</p>
              </div>
            ) : (
              <iframe
                title={t.title || t.url}
                src={t.url}
                className="h-full w-full border-0 bg-white"
                referrerPolicy="no-referrer-when-downgrade"
                sandbox="allow-same-origin allow-scripts allow-forms allow-popups"
              />
            )}
          </div>
        );
      })}
    </div>
  );
}
