# hivemind-main

AI-native desktop browser. Walking skeleton (Phase 0, steps 01–04).

This checkpoint covers: Rust workspace scaffold, browser-core domain model, SQLite persistence, and a React tab strip / URL bar / stub sidebar wired through Tauri 2. **AI chat (steps 05–08) lands next session.**

See `../docs/` for full specs and `../workload/work00-walking-skeleton/` for the step-by-step contract this build follows.

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

## Linux dev prerequisites

Already installed on this machine: `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`, `libsoup-3.0-dev`, `libjavascriptcoregtk-4.1-dev`, `libssl-dev`, `build-essential`, `pkg-config`, `curl`, `wget`, `file`.

If recreating on another Ubuntu/Debian box:

```bash
sudo apt install libwebkit2gtk-4.1-dev build-essential curl wget file \
  libssl-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev \
  libsoup-3.0-dev libjavascriptcoregtk-4.1-dev pkg-config
```

Plus the Rust toolchain (`rustup`) and Node 22+ with `pnpm`.

## Run

```bash
cd apps/desktop
pnpm install                       # one-time
pnpm tauri dev                     # launches the Tauri window
```

Logging: `RUST_LOG=hivemind=debug,info pnpm tauri dev`.

App data (SQLite DB) lives at `~/.local/share/hivemind/hivemind.db`.

## Workspace commands

```bash
cargo build --workspace            # compile every crate
cargo test --workspace             # run unit + integration tests
cargo clippy --workspace --all-targets -- -D warnings
```

## Known shortcuts (revisit later)

- **Single webview per window**: tab switching currently re-navigates one shared webview rather than per-tab webviews. Persistence and behavior are correct; visual snap on switch is a polish item for step 08 / Phase 1.
- **`ts-rs` not wired**: frontend types in `src/types.ts` are hand-maintained mirrors of `crates/ipc-types`. Auto-generation can be added in step 05.

## What's next

`workload/work00-walking-skeleton/step-05-..step-08-*.md` covers:

- Step 05 — `Provider` trait + Anthropic streaming (SSE) with prompt caching.
- Step 06 — Ollama provider (NDJSON streaming + embeddings).
- Step 07 — Minimal AI orchestrator + Router + `send_message` Tauri command.
- Step 08 — Real sidebar chat UI with `hm:chat-token` / `hm:chat-complete` events.

License: MIT.
