-- Outgoing e-mail share queue (offline-friendly: failed sends are retried).
CREATE TABLE IF NOT EXISTS email_queue (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    token      TEXT NOT NULL,                    -- photo share token
    to_addr    TEXT NOT NULL,
    status     TEXT NOT NULL DEFAULT 'pending',  -- pending | sent | error
    error      TEXT,
    attempts   INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_email_queue_status ON email_queue (status);
