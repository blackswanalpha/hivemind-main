import { useEffect, useMemo, useState } from "react";

import { useAiSettings } from "../state/AiSettingsContext";
import type { AiSettingsPayload, TestProviderResult } from "../types";

interface Props {
  open: boolean;
  onClose: () => void;
}

const ANTHROPIC_MODELS = ["claude-sonnet-4-6", "claude-haiku-4-5-20251001"];
const OLLAMA_MODELS = ["llama3.2", "mistral-nemo", "qwen2.5"];

const POLICIES: Array<{ value: AiSettingsPayload["policy"]; label: string }> = [
  { value: "prefer_local", label: "Prefer local" },
  { value: "prefer_cloud", label: "Prefer cloud" },
  { value: "explicit_name", label: "Lock to selected provider" },
];

const FALLBACK_DRAFT: AiSettingsPayload = {
  provider: "ollama",
  model: undefined,
  policy: "prefer_local",
};

export function SettingsDrawer({ open, onClose }: Props) {
  const { providers, settings, isReady, reload, updateSettings, testProvider } =
    useAiSettings();
  const [draft, setDraft] = useState<AiSettingsPayload | null>(settings);
  const [testResult, setTestResult] = useState<TestProviderResult | null>(null);
  const [testing, setTesting] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Sync `draft` whenever the drawer opens OR settings finishes loading.
  // Without the second case, opening the drawer before settings has loaded
  // leaves `draft` null and the panel silently invisible.
  useEffect(() => {
    if (!open) return;
    if (settings) {
      setDraft(settings);
    } else if (isReady) {
      setDraft(FALLBACK_DRAFT);
    }
    setTestResult(null);
    setError(null);
  }, [open, isReady, settings]);

  // If the user opens the drawer before AiSettingsContext has loaded, kick a
  // reload so we don't sit at "Loading…" forever if the initial load lost a race.
  useEffect(() => {
    if (open && !isReady) {
      reload().catch((err) => setError(String(err)));
    }
  }, [open, isReady, reload]);

  const modelChoices = useMemo(() => {
    if (!draft) return [];
    if (draft.provider === "anthropic") return ANTHROPIC_MODELS;
    if (draft.provider === "ollama") return OLLAMA_MODELS;
    return [];
  }, [draft]);

  if (!open) return null;

  const handleProviderChange = (name: string) => {
    if (!draft) return;
    setDraft({ ...draft, provider: name, model: undefined });
    setTestResult(null);
  };

  const handlePolicyChange = (policy: string) => {
    if (!draft) return;
    setDraft({ ...draft, policy });
  };

  const handleTest = async () => {
    if (!draft) return;
    setTesting(true);
    setTestResult(null);
    setError(null);
    try {
      const result = await testProvider(draft.provider);
      setTestResult(result);
    } catch (err) {
      setError(String(err));
    } finally {
      setTesting(false);
    }
  };

  const handleSave = async () => {
    if (!draft) return;
    setSaving(true);
    setError(null);
    try {
      await updateSettings(draft);
      onClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="absolute inset-0 z-20 flex bg-black/40">
      <div
        className="flex-1"
        onClick={onClose}
        aria-label="Close settings"
        role="button"
      />
      <aside className="flex w-full max-w-[360px] flex-col gap-4 overflow-y-auto border-l border-hivemind-border bg-hivemind-panel p-4 shadow-xl">
        <div className="flex items-center justify-between">
          <h2 className="text-sm font-medium">AI Providers</h2>
          <button
            onClick={onClose}
            className="text-hivemind-mute hover:text-hivemind-fg"
            aria-label="Close"
          >
            ✕
          </button>
        </div>

        {!draft ? (
          <div className="flex flex-1 items-center justify-center text-xs text-hivemind-mute">
            Loading AI providers…
          </div>
        ) : (
          <>
        <section className="flex flex-col gap-2">
          <div className="text-[11px] uppercase tracking-wide text-hivemind-mute">
            Provider
          </div>
          {providers.length === 0 ? (
            <div className="text-xs text-hivemind-mute">
              No providers registered. Set ANTHROPIC_API_KEY or start Ollama.
            </div>
          ) : (
            providers.map((p) => (
              <label
                key={p.name}
                className="flex cursor-pointer items-center justify-between gap-2 rounded border border-hivemind-border bg-hivemind-bg/40 px-3 py-2 text-xs"
              >
                <span className="flex items-center gap-2">
                  <input
                    type="radio"
                    name="provider"
                    value={p.name}
                    checked={draft.provider === p.name}
                    onChange={() => handleProviderChange(p.name)}
                  />
                  <span className="font-medium capitalize">{p.name}</span>
                  <span className="text-hivemind-mute">
                    {p.local ? "local" : "cloud"}
                  </span>
                </span>
                <span className="text-[10px] text-hivemind-mute">
                  {p.supportsTools ? "tools" : ""}{" "}
                  {p.supportsPromptCaching ? "cache" : ""}
                </span>
              </label>
            ))
          )}
        </section>

        {modelChoices.length > 0 ? (
          <section className="flex flex-col gap-1">
            <div className="text-[11px] uppercase tracking-wide text-hivemind-mute">
              Model
            </div>
            <select
              value={draft.model ?? ""}
              onChange={(e) =>
                setDraft({ ...draft, model: e.target.value || undefined })
              }
              className="rounded border border-hivemind-border bg-hivemind-bg/40 px-2 py-1 text-xs text-hivemind-fg outline-none"
            >
              <option value="">(provider default)</option>
              {modelChoices.map((m) => (
                <option key={m} value={m}>
                  {m}
                </option>
              ))}
            </select>
          </section>
        ) : null}

        <section className="flex flex-col gap-1">
          <div className="text-[11px] uppercase tracking-wide text-hivemind-mute">
            Routing policy
          </div>
          {POLICIES.map((p) => (
            <label
              key={p.value}
              className="flex cursor-pointer items-center gap-2 text-xs"
            >
              <input
                type="radio"
                name="policy"
                value={p.value}
                checked={draft.policy === p.value}
                onChange={() => handlePolicyChange(p.value)}
              />
              {p.label}
            </label>
          ))}
        </section>

        <section className="flex flex-col gap-2">
          <button
            type="button"
            onClick={handleTest}
            disabled={testing}
            className="rounded border border-hivemind-border bg-hivemind-bg/40 px-3 py-1 text-xs hover:border-hivemind-accent disabled:opacity-50"
          >
            {testing ? "Testing…" : "Test connection"}
          </button>
          {testResult ? (
            <div
              className={`text-[11px] ${
                testResult.ok ? "text-emerald-400" : "text-red-300"
              }`}
            >
              {testResult.ok
                ? `✓ reachable (${testResult.latencyMs ?? "?"} ms)`
                : `✕ ${testResult.error ?? "failed"}`}
            </div>
          ) : null}
        </section>

        {error ? (
          <div className="text-[11px] text-red-300">{error}</div>
        ) : null}

        <div className="mt-auto flex justify-end gap-2">
          <button
            type="button"
            onClick={onClose}
            className="rounded border border-hivemind-border bg-hivemind-bg/40 px-3 py-1 text-xs"
          >
            Cancel
          </button>
          <button
            type="button"
            onClick={handleSave}
            disabled={saving}
            className="rounded border border-hivemind-accent bg-hivemind-accent/30 px-3 py-1 text-xs hover:bg-hivemind-accent/40 disabled:opacity-50"
          >
            {saving ? "Saving…" : "Save"}
          </button>
        </div>
          </>
        )}
      </aside>
    </div>
  );
}
