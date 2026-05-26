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

## Tab engine: migrating to CEF (Chromium)

The original walking skeleton renders each tab inside an HTML `<iframe>`
hosted by Tauri's wry/webkit2gtk webview. That approach hit two walls:
major sites (Google, YouTube, Twitter/X, LinkedIn, Gmail, banks) refuse to
be iframed via `X-Frame-Options` / CSP `frame-ancestors`, and Widevine DRM
(Netflix, Spotify Web, Disney+) is not implementable on webkit2gtk at all.

The engine for tab webviews is being migrated to **CEF (Chromium Embedded
Framework)** via the [`cef` crate](https://crates.io/crates/cef). Tauri 2
remains the app shell (chrome, IPC, AI sidebar); CEF handles each tab as a
reparented child window. Plan and migration phases are tracked in
`../docs/plan-cef.md` (alongside the other phase plans). Step-level work
breakdown lives in `../workload/work04-engine-cef/`; runtime architecture
in `../workload/flow27-cef-runtime/`.

Phase A (CEF bootstrap) landed on the `engine/cef` branch 2026-05-26.

What this changes for runtime requirements (Linux):

- **Display server**: X11 or XWayland. Native Wayland is deferred — CEF
  embeds via X11 reparenting today.
- **Bundle size**: ~250 MB installed (CEF binary distribution at
  ~200 MB + Tauri shell).
- **Per-tab RAM**: 50–100 MB. Tab discard will be implemented; tabs
  inactive past a threshold will be unloaded and recreated on activation.
- **No system codec install needed** — CEF ships ffmpeg, H.264, AAC, VP9,
  Opus, AV1. (The previous version of this README documented a `gst-*`
  apt-install dance; that is no longer relevant for tab content.)

### DRM (Netflix, Spotify Web, Disney+, Prime Video, Apple Music, Tidal)

CEF supports Widevine, but the CDM binary is not redistributable. To enable
DRM playback, **install Google Chrome on your system** — HiveMind detects
and reuses Chrome's `WidevineCdm/` directory at startup. If Chrome is not
installed, HiveMind shows a settings banner pointing to it.

Even with the CDM available, **Linux is capped at Widevine L3**:

- Netflix plays at **480p only** (this is the same ceiling as Firefox /
  Chrome on Linux — a platform limit, not a HiveMind bug)
- Disney+ plays at SD
- Spotify Web plays at standard quality
- YouTube Premium plays at full quality (does not require L1)

Widevine L1 (HD / 4K) requires Google's OEM-only hardware certification and
is not attainable for non-OEM software regardless of engine.

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

- **Iframes per tab inside one OS webview** — `apps/desktop/src/components/Webview.tsx`
  renders each tab as an HTML `<iframe>` toggled by CSS `display`, not as a
  real webview. This is the biggest reason most sites (Google, YouTube,
  Twitter/X, LinkedIn, Gmail, banks) currently show blank pages: they send
  `X-Frame-Options: DENY` or CSP `frame-ancestors` and the webview honors
  them. The fix is the CEF migration described under "Tab engine" above —
  each tab becomes a reparented `CefBrowserHost`, not a wry webview.
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
