-- SnapStation initial schema.

-- User-facing settings (event name, language, theme, chroma calibration, ...).
-- Everything configurable lives here and is edited via the web interface.
CREATE TABLE IF NOT EXISTS settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- An event / capture session that photos belong to.
CREATE TABLE IF NOT EXISTS sessions (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    ulid       TEXT NOT NULL UNIQUE,
    name       TEXT NOT NULL DEFAULT '',
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Captured photos. Image bytes live on disk; only metadata is stored here.
CREATE TABLE IF NOT EXISTS photos (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    ulid          TEXT NOT NULL UNIQUE,
    token         TEXT NOT NULL UNIQUE,           -- unguessable share/download token
    session_id    INTEGER REFERENCES sessions(id) ON DELETE SET NULL,
    original_path TEXT NOT NULL,
    thumb_path    TEXT,
    width         INTEGER,
    height        INTEGER,
    kind          TEXT NOT NULL DEFAULT 'single', -- single | collage | gif
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_photos_created ON photos (created_at DESC, id DESC);
CREATE INDEX IF NOT EXISTS idx_photos_session ON photos (session_id);

-- Print jobs and their CUPS-derived status.
CREATE TABLE IF NOT EXISTS print_jobs (
    id          INTEGER PRIMARY KEY AUTOINCREMENT,
    photo_id    INTEGER NOT NULL REFERENCES photos(id) ON DELETE CASCADE,
    printer     TEXT NOT NULL,
    cups_job_id INTEGER,
    status      TEXT NOT NULL DEFAULT 'queued',   -- queued | printing | done | error
    message     TEXT,
    created_at  TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_print_jobs_photo ON print_jobs (photo_id);

-- Single administrator credential (row id is pinned to 1).
CREATE TABLE IF NOT EXISTS admin (
    id            INTEGER PRIMARY KEY CHECK (id = 1),
    password_hash TEXT NOT NULL,
    created_at    TEXT NOT NULL DEFAULT (datetime('now'))
);
