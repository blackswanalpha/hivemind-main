# HiveMind — Description

> An AI-native desktop browser written in Rust. HiveMind treats the browser as
> a workspace with memory and a model, not as a faster way to render HTML.

## In one paragraph

Most browsers ship features for opening pages. HiveMind ships features for
*using* what's on those pages — a sidebar chat that can see the current tab,
long-term memory that persists across sessions, and (later) agents that can
act on your behalf. It does not build its own rendering engine; the web is
hard enough. It uses the system webview (via Tauri 2 + wry) and spends its
budget on the parts a normal browser doesn't have: the AI provider layer, the
memory store, the tool registry, and the orchestrator that ties them
together.

## Who this is for

- **Heavy reader / researcher.** People who keep 60 tabs open, paste them
  into a chat, and lose the thread by Friday. HiveMind keeps the thread.
- **People who already pay for an LLM.** Bring-your-own-key for Anthropic /
  OpenAI; or run fully local against Ollama. No HiveMind cloud account is
  required to use it.
- **Privacy-leaning power users.** Local-first storage. Sync (Phase 3) is
  opt-in, end-to-end encrypted, and routes through a dumb relay rather than
  a service that reads your data.

It is **not** trying to replace Chrome for casual browsing, and it is not a
"Chrome with a chat button bolted on" — the memory and tool surfaces are the
point.

## What ships, by phase

The project is built in four phases. Each phase has a "daily-driver quality"
bar that gates the next one (see [explanation.md](./explanation.md) for the
rationale, and `../docs/plan*.md` for the per-phase plans).

| Phase | Theme                                | What you can do                                                                                         | Status        |
|-------|--------------------------------------|---------------------------------------------------------------------------------------------------------|---------------|
| 0     | Walking skeleton                     | Open tabs, navigate, switch workspaces, restart and find them again, talk to an AI in the sidebar.      | In progress   |
| 1     | Memory + tool use                    | Auto-extract & summarize visited pages, embed them, semantic recall in chat, 3 baseline tools.          | Planned       |
| 2     | Agents + automation                  | A Research agent that plans and runs tool calls. Workflow record/replay. WASM-sandboxed extensions.     | Planned       |
| 3     | Cross-device sync + light collab     | E2E-encrypted sync via a relay; CRDT merge of tabs / memories / workspaces across your devices.         | Planned       |

Multi-agent coordination and distributed inference are explicitly out of
scope for v1.

## Status today (2026-05-26)

Phase 0, steps 01–04 of `../workload/work00-walking-skeleton/`:

- Rust workspace compiles (`cargo build --workspace` clean).
- Pure-Rust domain model in `crates/browser-core` (Tab, Workspace, Session).
- SQLite persistence via `sqlx` in `crates/storage`, behind the
  `SessionStore` trait.
- Tauri 2 app with a React tab strip, URL bar, and stub sidebar.
- AI provider, orchestrator, and memory crates exist as stubs so the
  workspace builds today and step 05 (Anthropic streaming) drops in without
  restructuring.

Steps 05–08 — `Provider` trait + Anthropic SSE with prompt caching, Ollama,
the minimal orchestrator, and real streaming chat in the sidebar — are the
next session's work.

## What HiveMind is not

- **Not a new browser engine.** Building one is a decade of work; the PRD
  concedes this in `../docs/prd.md` §5. HiveMind uses the system webview.
- **Not a cloud product.** There is no HiveMind account, no telemetry by
  default, no server holding your data. Sync (Phase 3) is opt-in and the
  relay never sees plaintext.
- **Not shipping.** The Phase 0 skeleton runs locally; there are no
  binaries, no releases, no installers yet. v0.1 is the bar for that.

## Where to read more

- **How and why it's built this way** → [explanation.md](./explanation.md)
- **Getting it running locally** → [README.md](./README.md)
- **Architecture diagrams (C4, Mermaid)** → `../docs/arch.md`
- **Per-phase plans with preconditions and out-of-scope lists** →
  `../docs/plan.md`, `plan02.md`, `plan03.md`, `plan04.md`
- **Step-by-step execution contract for Phase 0** →
  `../workload/work00-walking-skeleton/`

License: MIT.
