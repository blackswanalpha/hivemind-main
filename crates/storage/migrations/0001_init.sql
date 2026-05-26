-- HiveMind initial schema (Phase 0).
--
-- Includes tables not strictly used until later steps/phases (conversations,
-- messages, memories, budget_ledger, config) so that work00 step 05+ does
-- not need a second migration. Each non-P0 table is harmless when unused.

CREATE TABLE workspaces (
    id          TEXT PRIMARY KEY,
    name        TEXT NOT NULL,
    created_at  INTEGER NOT NULL
);

CREATE TABLE tabs (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT NOT NULL REFERENCES workspaces(id) ON DELETE CASCADE,
    url             TEXT NOT NULL,
    title           TEXT NOT NULL DEFAULT '',
    favicon         BLOB,
    position        INTEGER NOT NULL,
    opened_at       INTEGER NOT NULL,
    last_active_at  INTEGER NOT NULL
);

CREATE TABLE history (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    url             TEXT NOT NULL,
    title           TEXT,
    visited_at      INTEGER NOT NULL,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE SET NULL
);

CREATE TABLE conversations (
    id              TEXT PRIMARY KEY,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    started_at      INTEGER NOT NULL
);

CREATE TABLE messages (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    conversation_id TEXT NOT NULL REFERENCES conversations(id) ON DELETE CASCADE,
    role            TEXT NOT NULL,
    content         TEXT NOT NULL,
    tool_calls      TEXT,
    created_at      INTEGER NOT NULL
);

CREATE TABLE memories (
    id              INTEGER PRIMARY KEY AUTOINCREMENT,
    kind            TEXT NOT NULL,
    source_url      TEXT,
    content         TEXT NOT NULL,
    embedding       BLOB,
    workspace_id    TEXT REFERENCES workspaces(id) ON DELETE SET NULL,
    created_at      INTEGER NOT NULL
);

CREATE TABLE budget_ledger (
    day             TEXT PRIMARY KEY,
    tokens_used     INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE config (
    key             TEXT PRIMARY KEY,
    value           TEXT
);

CREATE INDEX idx_tabs_workspace      ON tabs(workspace_id, position);
CREATE INDEX idx_history_visited     ON history(visited_at DESC);
CREATE INDEX idx_memories_workspace  ON memories(workspace_id, created_at DESC);
CREATE INDEX idx_messages_conv       ON messages(conversation_id, created_at);
