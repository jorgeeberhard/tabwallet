-- Profile (UC14): an optional display name. Empty means "not set".
ALTER TABLE users ADD COLUMN name TEXT NOT NULL DEFAULT '';

-- Password recovery (UC03). The link e-mailed to the user carries a random
-- token; only its SHA-256 hash is stored, so a database leak cannot be used to
-- reset passwords. A token works once (`used_at`) and until `expires_at`.
CREATE TABLE password_reset_tokens (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id) ON DELETE CASCADE,
    token_hash TEXT        NOT NULL UNIQUE,
    expires_at TIMESTAMPTZ NOT NULL,
    used_at    TIMESTAMPTZ,
    created_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX password_reset_tokens_user_id_idx ON password_reset_tokens (user_id);
