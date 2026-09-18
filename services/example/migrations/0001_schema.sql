-- Example service: a trivially simple widget resource, demonstrating this
-- workspace's migration conventions (see CLAUDE.md "Migrations Are
-- Immutable" — this file, once applied anywhere, must never be edited again;
-- changes land as a new migration). Not real domain data — replace `widgets`
-- with your own service's first table when you fork this template.

CREATE SCHEMA IF NOT EXISTS example;

-- ============================================================================
-- widgets — id, name, created_at. Ids are minted by Postgres at insert time
-- (see feature::widget::repository::PgWidgetRepository::create), so there is
-- no client-supplied-id column default to worry about.
-- ============================================================================
CREATE TABLE IF NOT EXISTS example.widgets (
    id         UUID PRIMARY KEY,
    name       TEXT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);
CREATE INDEX IF NOT EXISTS idx_widgets_pagination ON example.widgets (created_at DESC, id DESC);
