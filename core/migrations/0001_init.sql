-- Initial schema. Mirrors the DTOs in the `syle-types` crate.
-- Single catalog: no tenant scoping.

CREATE TABLE galleries (
    id        UUID PRIMARY KEY,
    slug      TEXT NOT NULL UNIQUE,
    title     TEXT NOT NULL,
    position  INTEGER NOT NULL DEFAULT 0,
    published BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE TABLE photos (
    id         UUID PRIMARY KEY,
    gallery_id UUID NOT NULL REFERENCES galleries (id) ON DELETE CASCADE,
    alt        TEXT NOT NULL DEFAULT '',
    thumbhash  TEXT NOT NULL,
    width      INTEGER NOT NULL,
    height     INTEGER NOT NULL,
    position   INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX photos_gallery_id_idx ON photos (gallery_id);

-- Photo.variants (Vec<ImageVariant>) normalized out.
CREATE TABLE photo_variants (
    photo_id UUID NOT NULL REFERENCES photos (id) ON DELETE CASCADE,
    format   TEXT NOT NULL CHECK (format IN ('avif', 'webp')),
    width    INTEGER NOT NULL,
    path     TEXT NOT NULL,
    PRIMARY KEY (photo_id, format, width)
);

CREATE TABLE blog_posts (
    id           UUID PRIMARY KEY,
    slug         TEXT NOT NULL UNIQUE,
    title        TEXT NOT NULL,
    body_md      TEXT NOT NULL,
    status       TEXT NOT NULL DEFAULT 'draft'
                 CHECK (status IN ('draft', 'published')),
    published_at BIGINT
);

CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         TEXT NOT NULL UNIQUE,
    password_hash TEXT NOT NULL
);

CREATE TABLE sessions (
    token      TEXT PRIMARY KEY,
    user_id    UUID NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    expires_at BIGINT NOT NULL
);
CREATE INDEX sessions_user_id_idx ON sessions (user_id);
