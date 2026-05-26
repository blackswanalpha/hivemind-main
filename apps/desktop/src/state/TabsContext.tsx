import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";

import { events, ipc } from "../ipc";
import type { SessionInfo, TabInfo, WorkspaceInfo } from "../types";

interface TabsContextValue {
  workspaces: WorkspaceInfo[];
  activeWorkspace: string;
  tabs: TabInfo[]; // tabs for the active workspace, sorted by position
  activeTab: string | null;
  isReady: boolean;
  openTab: (url?: string) => Promise<void>;
  closeTab: (id: string) => Promise<void>;
  setActiveTab: (id: string) => Promise<void>;
  navigate: (id: string, url: string) => Promise<void>;
  switchWorkspace: (id: string) => Promise<void>;
}

const TabsContext = createContext<TabsContextValue | null>(null);

const ABOUT_BLANK = "about:blank";

export function TabsProvider({ children }: { children: ReactNode }) {
  const [session, setSession] = useState<SessionInfo | null>(null);

  const refresh = useCallback(async () => {
    const s = await ipc.loadSession();
    setSession(s);
  }, []);

  useEffect(() => {
    refresh().catch((err) => {
      console.error("loadSession failed", err);
    });
  }, [refresh]);

  useEffect(() => {
    const unsubs: Promise<() => void>[] = [
      events.onTabOpened(() => {
        refresh().catch((err) => console.error(err));
      }),
      events.onTabClosed(() => {
        refresh().catch((err) => console.error(err));
      }),
      events.onTabNavigated(() => {
        refresh().catch((err) => console.error(err));
      }),
    ];
    return () => {
      unsubs.forEach((p) =>
        p.then((un) => un()).catch((err) => console.error(err)),
      );
    };
  }, [refresh]);

  const tabs = useMemo(() => {
    if (!session) return [];
    return session.tabs
      .filter((t) => t.workspaceId === session.activeWorkspace)
      .sort((a, b) => a.position - b.position);
  }, [session]);

  const openTab = useCallback(
    async (url?: string) => {
      if (!session) return;
      const target = url ?? ABOUT_BLANK;
      await ipc.openTab(session.activeWorkspace, target);
      await refresh();
    },
    [session, refresh],
  );

  const closeTab = useCallback(
    async (id: string) => {
      await ipc.closeTab(id);
      await refresh();
    },
    [refresh],
  );

  const setActiveTab = useCallback(
    async (id: string) => {
      await ipc.setActiveTab(id);
      await refresh();
    },
    [refresh],
  );

  const navigate = useCallback(
    async (id: string, url: string) => {
      await ipc.navigate(id, url);
      await refresh();
    },
    [refresh],
  );

  const switchWorkspace = useCallback(
    async (id: string) => {
      await ipc.switchWorkspace(id);
      await refresh();
    },
    [refresh],
  );

  const value: TabsContextValue = {
    workspaces: session?.workspaces ?? [],
    activeWorkspace: session?.activeWorkspace ?? "",
    tabs,
    activeTab: session?.activeTab ?? null,
    isReady: session !== null,
    openTab,
    closeTab,
    setActiveTab,
    navigate,
    switchWorkspace,
  };

  return <TabsContext.Provider value={value}>{children}</TabsContext.Provider>;
}

export function useTabs(): TabsContextValue {
  const v = useContext(TabsContext);
  if (!v) throw new Error("useTabs must be used within <TabsProvider>");
  return v;
}
