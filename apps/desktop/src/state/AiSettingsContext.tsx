import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useState,
  type ReactNode,
} from "react";

import { ipc } from "../ipc";
import type {
  AiProviderInfo,
  AiSettingsPayload,
  TestProviderResult,
} from "../types";

interface AiSettingsContextValue {
  providers: AiProviderInfo[];
  settings: AiSettingsPayload | null;
  isReady: boolean;
  reload: () => Promise<void>;
  updateSettings: (next: AiSettingsPayload) => Promise<void>;
  testProvider: (name: string) => Promise<TestProviderResult>;
}

const AiSettingsContext = createContext<AiSettingsContextValue | null>(null);

export function AiSettingsProvider({ children }: { children: ReactNode }) {
  const [providers, setProviders] = useState<AiProviderInfo[]>([]);
  const [settings, setSettings] = useState<AiSettingsPayload | null>(null);
  const [isReady, setIsReady] = useState(false);

  const reload = useCallback(async () => {
    const [ps, s] = await Promise.all([ipc.listProviders(), ipc.getAiSettings()]);
    setProviders(ps);
    setSettings(s);
    setIsReady(true);
  }, []);

  useEffect(() => {
    reload().catch((err) => console.error("ai settings load failed", err));
  }, [reload]);

  const updateSettings = useCallback(
    async (next: AiSettingsPayload) => {
      await ipc.setAiSettings(next);
      setSettings(next);
    },
    [],
  );

  const testProvider = useCallback(
    async (name: string) => ipc.testProvider(name),
    [],
  );

  const value: AiSettingsContextValue = {
    providers,
    settings,
    isReady,
    reload,
    updateSettings,
    testProvider,
  };

  return (
    <AiSettingsContext.Provider value={value}>
      {children}
    </AiSettingsContext.Provider>
  );
}

export function useAiSettings(): AiSettingsContextValue {
  const v = useContext(AiSettingsContext);
  if (!v) throw new Error("useAiSettings must be used within <AiSettingsProvider>");
  return v;
}
