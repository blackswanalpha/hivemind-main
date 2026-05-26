import { useEffect } from "react";

import { AddressBar } from "./components/AddressBar";
import { EmptyWorkspace } from "./components/EmptyWorkspace";
import { Sidebar } from "./components/Sidebar";
import { StatusBar } from "./components/StatusBar";
import { TabStrip } from "./components/TabStrip";
import { Webview } from "./components/Webview";
import { events } from "./ipc";
import { AiSettingsProvider } from "./state/AiSettingsContext";
import { ChatProvider } from "./state/ChatContext";
import { TabsProvider, useTabs } from "./state/TabsContext";

function MainSurface() {
  const { tabs, isReady } = useTabs();

  if (!isReady) {
    return (
      <div className="flex h-full items-center justify-center text-sm text-hivemind-mute">
        Loading session…
      </div>
    );
  }

  return (
    <div className="flex h-full flex-col">
      <TabStrip />
      <AddressBar />
      <div className="relative flex flex-1 overflow-hidden">
        {tabs.length === 0 ? <EmptyWorkspace /> : <Webview />}
        <Sidebar />
      </div>
      <StatusBar />
    </div>
  );
}

function AppStartedLogger() {
  useEffect(() => {
    const p = events.onAppStarted((payload) => {
      console.info("AppStarted", payload);
    });
    return () => {
      p.then((un) => un()).catch((err) => console.error(err));
    };
  }, []);
  return null;
}

export default function App() {
  return (
    <TabsProvider>
      <AiSettingsProvider>
        <ChatProvider>
          <AppStartedLogger />
          <MainSurface />
        </ChatProvider>
      </AiSettingsProvider>
    </TabsProvider>
  );
}
