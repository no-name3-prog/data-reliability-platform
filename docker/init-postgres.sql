-- Schema bootstrap for future SQL-backed stores.
-- The default in-memory store works without these tables; they exist so
-- Postgres is exercised from day one in the compose stack.

CREATE TABLE IF NOT EXISTS schema_migrations (
    version TEXT PRIMARY KEY,
    applied_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

INSERT INTO schema_migrations (version) VALUES ('0001_init')
ON CONFLICT DO NOTHING;
