-- Pending signups (UC01, ADR-0004).
--
-- A sign-up request waits here, outside `users`, until the person opens the
-- confirmation link sent to the e-mail; only then does the Account (and its
-- default categories) exist. `users` therefore holds confirmed accounts only,
-- and a pending signup never occupies an e-mail: it is unique per address
-- (any letter case, citext), and a new request for the same address replaces
-- it, with a new password, name and link.
--
-- Like the password-reset tokens, the link carries a random token and only its
-- SHA-256 hash is stored. The signup lasts 24 hours (`expires_at`, set by the
-- API from its clock); expired rows are ignored everywhere and deleted when a
-- new request arrives, with no scheduled job (ADR-0002). `password_hash` is an
-- Argon2id PHC string, never the password.
--
-- Purely additive: the previous API version never touches this table.
CREATE TABLE pending_signups (
    id            UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    email         CITEXT      NOT NULL UNIQUE,
    name          TEXT        NOT NULL,
    password_hash TEXT        NOT NULL,
    token_hash    TEXT        NOT NULL UNIQUE,
    created_at    TIMESTAMPTZ NOT NULL,
    expires_at    TIMESTAMPTZ NOT NULL
);
