# HiveMind — Explanation

This document explains *why* the code in `hivemind-main/` is shaped the way
it is. For a product-level pitch see [description.md](./description.md); for
how to run it, see [README.md](./README.md).

The short version: HiveMind is a large idea built on a small foundation, on
purpose. The spec in `../docs/` describes a ~19-crate, multi-team system.
What's on disk is a 6-crate workspace + 1 Tauri app, because the only honest
way to ship something this ambitious solo is to keep the core tiny and let
each phase add load-bearing pieces only when the previous phase actually
runs.

## 1. Decisions that are locked

These were settled during the planning pass (2026-05-26, see `../docs/arch.md`)
and are **not** revisited per-step. If you want to argue with them, argue
with the plan docs, not with the code.

| Decision                                  | Choice                                              | Why                                                              |
|-------------------------------------------|-----------------------------------------------------|------------------------------------------------------------------|
| Rendering engine                          | System webview (Tauri 2 + wry)                      | Custom engines are 10+ engineer-years. PRD §5 already concedes.  |
| Desktop shell                             | Tauri 2                                             | Rust-first, small binary, good IPC story, mature on Linux.       |
| Initial target OS                         | Linux only                                          | Cuts packaging surface for v0; Mac/Win added once daily-driver.  |
| AI provider boundary                      | One `Provider` trait, multiple impls                | User requirement; lets cloud and local coexist behind one router.|
| First two providers                       | Anthropic (with prompt caching) + Ollama (local)    | One paid frontier model, one free local fallback.                |
| Storage                                   | `sqlx` + SQLite, single file                        | Local-first; no daemon; great enough until ~50k memories.        |
| Vector recall (Phase 1)                   | Brute-force cosine in SQLite                        | At <50k vectors it's fast and has no extra dependency.           |
| Sync transport (Phase 3)                  | Dumb relay + Automerge CRDTs + age/AEAD             | Relay never sees plaintext; merges are deterministic.            |
| Crate count at v0                         | 6 (was 19 in feature.md §16)                        | Most of the 19 are premature; introduce them when needed.        |

## 2. The shape of the workspace

```
hivemind-main/
├── Cargo.toml                    # workspace; pins shared deps once
├── apps/desktop/
│   ├── src/                      # React + Vite + Tailwind frontend
│   └── src-tauri/                # Tauri commands + AppState (Rust side)
└── crates/
    ├── browser-core/             # pure Rust: Tab, Workspace, Session, SessionStore (trait)
    ├── storage/                  # sqlx + SqliteSessionStore (impl of the trait)
    ├── ipc-types/                # serde DTOs shared with the frontend
    ├── ai-provider/              # stub today; Provider trait + Anthropic/Ollama in step 05–06
    ├── ai-orchestrator/          # stub today; Router + send_message in step 07
    └── memory/                   # stub today; ingest + recall in Phase 1
```

Two things matter about this shape:

**(a) `browser-core` has no Tauri and no SQL.** Persistence is the
`SessionStore` trait. `hivemind-storage::SqliteSessionStore` is one
implementation; tests use an in-memory fake. This is the hexagonal seam —
the domain model doesn't know it lives inside a Tauri app, which means we
can change shells (CLI for tests, headless for CI, eventually a server)
without rewriting the model.

**(b) The AI crates are present-but-empty.** That is deliberate: the
6-crate workspace already compiles, so step 05 lights up
`ai-provider/src/lib.rs` without renaming directories or rewriting
`Cargo.toml`. Empty stubs are cheaper than future refactors.

## 3. The IPC boundary

The frontend never sees Rust newtypes. The split is:

- **`browser-core`** speaks `TabId`, `WorkspaceId`, `Url`, `DateTime<Utc>`.
  Type-safe, opinionated, *internal*.
- **`ipc-types`** speaks `String`, `i64`, plain `Vec<T>`. The wire format.
  All `#[serde(rename_all = "camelCase")]` so the React side reads
  idiomatically.
- **`apps/desktop/src-tauri/commands.rs`** is the translation layer. It
  parses strings → newtypes on the way in, formats newtypes → strings on
  the way out. Every Tauri command goes through there.
- **`apps/desktop/src/types.ts`** hand-mirrors `ipc-types`. This is the one
  duplication in the system; `ts-rs` codegen is the cleanup item once the
  surface grows past a handful of types (revisit in step 05).

If you find yourself wanting to expose `TabId` to the frontend directly,
don't. The shim is load-bearing — it's how the model stays free to evolve
without breaking the React side.

## 4. The phase ladder

Each phase has a hard precondition: the previous phase must be **daily-driver
quality**, not just "compiles." That bar exists because the project is
solo-built and every premature phase is a tax paid forever.

- **Phase 0 — walking skeleton (`work00-…`):** open tabs, persist, restart,
  one streaming AI conversation. If this isn't pleasant to use for an hour,
  Phase 1 doesn't start.
- **Phase 1 — memory + tools (`plan02.md`):** Readability.js extraction,
  summarize-and-embed pipeline, brute-force cosine recall, three tools
  (`search_memory`, `open_url`, `summarize_tab`), workspaces UI. The gate
  is "I'd notice if memory broke."
- **Phase 2 — agents + automation (`plan03.md`):** one Research agent,
  workflow record/replay, wasmtime-sandboxed extensions. Multi-agent
  coordination is **explicitly deferred**.
- **Phase 3 — sync (`plan04.md`):** relay + Automerge + E2E encryption.
  Real-time collab is sketched, not promised. Distributed inference is
  deferred indefinitely.

Each `plan*.md` ends with an "Out of scope" list and "Open questions" the
user should decide *before* the phase starts. That's where most of the
honest scope-cutting lives.

## 5. Trade-offs we accepted in the skeleton

These exist on purpose. Each one trades engineering effort now for a small
follow-up later, when it's cheaper to know what the right answer is.

- **Single webview per window.** Tab switching re-navigates one shared
  webview instead of stacking per-tab webviews. Visual snap on switch;
  persistence is correct. Polish item for step 08 / Phase 1.
- **Hand-maintained TS types.** `src/types.ts` mirrors `crates/ipc-types`
  manually. `ts-rs` codegen lands when the type surface justifies the
  build-step cost (likely step 05).
- **No real engine for embeddings yet.** Phase 1's brute-force cosine is
  fine until ~50k vectors. Beyond that we'll plug in `hnswlib` or move to
  `sqlite-vec`. Pre-optimizing for 1M is wrong.
- **AI provider crates are stubs.** They cost zero to compile and save a
  workspace restructure when steps 05–06 fill them in.

## 6. What's *not* here, and why

These are not oversights — they're explicitly out of v0 scope. Don't add
them without a phase plan covering them.

- **Custom rendering engine.** See decision table.
- **Profiles / sandboxing per-site.** Phase 2 wasmtime work covers
  extension sandboxing; user-process isolation is a much bigger
  conversation, deferred.
- **Cloud sync of any kind.** Phase 3, opt-in, E2E, relay-not-server.
- **Mobile.** Tauri Mobile is plausible but unscoped.
- **Telemetry.** Not in v0. Any future telemetry must be opt-in and
  documented before it ships.

## 7. How to read the rest of the docs

- **`../docs/prd.md`** — product requirements, target users, monetization
  thinking. The "why does this exist as a product" file.
- **`../docs/system.md`** — original 25-section system architecture (the
  ambitious version).
- **`../docs/feature.md`** — full Rust feature matrix and the original
  19-crate layout (we trimmed to 6 for v0).
- **`../docs/arch.md`** — current architecture in C4 / Mermaid. **This is
  the source of truth.** Every box is tagged with the phase it lights up
  (P0/P1/P2/P3).
- **`../docs/plan.md`, `plan02.md`, `plan03.md`, `plan04.md`** — the
  per-phase plans, each with preconditions, scope, out-of-scope, and open
  questions.
- **`../workload/work00-walking-skeleton/`** — the step-by-step contract
  the current code follows. `step01..step04` are done; `step05..step08`
  are next.

When the docs and the code disagree, **the code is right and the docs
need updating** — but check the commit history first, because the docs are
often ahead of the code on purpose.
