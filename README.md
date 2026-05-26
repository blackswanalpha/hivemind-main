# hivemind-main

AI-native desktop browser in Rust. Walking skeleton, Phase 0.

> **Status (2026-05-26):** `work00` steps 01–04 done — Rust workspace,
> browser-core domain model, SQLite persistence, React tab strip / URL bar /
> stub sidebar wired through Tauri 2. **AI chat (steps 05–08) lands next
> session.**

## Read first

- **[description.md](./description.md)** — what HiveMind is, who it's for,
  what ships in each phase.
- **[explanation.md](./explanation.md)** — why the code is shaped the way it
  is: locked decisions, the IPC boundary, the phase ladder, accepted
  trade-offs.
- **`../docs/`** — full specs (PRD, system architecture, C4 diagrams, per-phase
  plans).
- **`../workload/work00-walking-skeleton/`** — the step-by-step contract this
  build follows.

## Layout

```
hivemind-main/
├── Cargo.toml                    # Rust workspace (6 crates + 1 app)
├── apps/desktop/                 # Tauri 2 + Vite + React + Tailwind
│   ├── src/                      # React frontend
│   └── src-tauri/                # Rust backend (Tauri commands)
└── crates/
    ├── browser-core/             # pure-Rust domain model (Tab, Workspace, Session, SessionStore)
    ├── storage/                  # sqlx SQLite + SqliteSessionStore
    ├── ipc-types/                # serde types shared with frontend
    ├── ai-provider/              # stub — wired in step 05
    ├── ai-orchestrator/          # stub — wired in step 07
    └── memory/                   # stub — Phase 1
```

Why 6 crates and not the 19 in `../docs/feature.md` §16 — see
[explanation.md §1](./explanation.md#1-decisions-that-are-locked).

## Prerequisites (Linux)

Already installed on this dev machine: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`,
`libayatana-appindicator3-dev`, `librsvg2-dev`, `libsoup-3.0-dev`,
`libjavascriptcoregtk-4.1-dev`, `libssl-dev`, `build-essential`, `pkg-config`,
`curl`, `wget`, `file`.

If recreating on a fresh Ubuntu/Debian box:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev pkg-config
```

Plus the Rust toolchain (`rustup`, see `rust-toolchain.toml`) and Node 22+
with `pnpm`.

Target OS for v0 is **Linux only** — Mac/Windows happen after the skeleton
is daily-driver quality.

## Run

From the workspace root (uses the proxy scripts in the root `package.json`):

```bash
pnpm install:ui                    # one-time; runs pnpm install in apps/desktop
pnpm tauri:dev                     # launches the Tauri window
```

Or directly from the UI package if you prefer:

```bash
cd apps/desktop && pnpm tauri dev
```

Verbose logging:

```bash
RUST_LOG=hivemind=debug,info pnpm tauri:dev
```

App data (SQLite DB) lives at `~/.local/share/hivemind/hivemind.db`. Delete
that file to reset the session; the next launch will seed a fresh default
workspace.

## Workspace commands

```bash
cargo build --workspace                                 # compile every crate
cargo test  --workspace                                 # unit + integration tests
cargo clippy --workspace --all-targets -- -D warnings   # lint gate
cargo fmt --all                                         # format
```

## Known shortcuts (revisit later)

These are deliberate, not bugs. The reasoning lives in
[explanation.md §5](./explanation.md#5-trade-offs-we-accepted-in-the-skeleton).

- **Single webview per window** — tab switching re-navigates one shared
  webview rather than per-tab webviews. Persistence and behavior are
  correct; the visual snap on switch is a polish item for step 08 / Phase 1.
- **`ts-rs` not wired** — `src/types.ts` is a hand-maintained mirror of
  `crates/ipc-types`. Auto-generation lands when the type surface grows
  (likely step 05).

## What's next

`workload/work00-walking-skeleton/step-05-…step-08-*.md` covers:

- **Step 05** — `Provider` trait + Anthropic streaming (SSE) with prompt
  caching.
- **Step 06** — Ollama provider (NDJSON streaming + embeddings).
- **Step 07** — Minimal AI orchestrator + Router + `send_message` Tauri
  command.
- **Step 08** — Real sidebar chat UI driven by `hm:chat-token` /
  `hm:chat-complete` events.

After Phase 0 wraps, see `../docs/plan02.md` for the memory + tools phase.

## License

MIT — see [LICENSE](./LICENSE).
