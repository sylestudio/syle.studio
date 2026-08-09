-- Standalone portfolio links. These appear beside galleries in the public
-- project grid but navigate directly to their configured URL.
CREATE TABLE projects (
    id        UUID PRIMARY KEY,
    title     TEXT NOT NULL,
    url       TEXT NOT NULL,
    category  TEXT NOT NULL DEFAULT '',
    position  INTEGER NOT NULL DEFAULT 0,
    published BOOLEAN NOT NULL DEFAULT FALSE,
    cover     JSONB
);

CREATE INDEX projects_published_position_idx
    ON projects (published, position, id);
