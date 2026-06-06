-- Background images for chroma-key compositing.
CREATE TABLE IF NOT EXISTS backgrounds (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ulid       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL DEFAULT '',
    path       TEXT NOT NULL,                 -- relative to the data directory
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
