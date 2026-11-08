-- Accounts (UC01). The e-mail is unique regardless of letter case (citext).
-- `password_hash` holds an Argon2id PHC string, never the password.
CREATE TABLE users (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         CITEXT      NOT NULL UNIQUE,
    password_hash TEXT        NOT NULL,
    created_at    TIMESTAMPTZ NOT NULL DEFAULT now()
);
