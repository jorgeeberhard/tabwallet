-- Pending e-mail changes (UC14, ADR-0004).
--
-- Asking to change the e-mail in the profile does not change it: the request
-- waits here until the person opens the confirmation link sent to the NEW
-- address. Until then `users.email` is unchanged and the old e-mail keeps
-- logging in.
--
-- At most one per account (`user_id` is unique): a new request replaces the
-- previous one, with a new address and link, and the old link stops working.
-- Unlike `pending_signups`, `new_email` is not unique: the address is not
-- reserved by anyone until confirmed, and the account that confirms it first
-- takes it (the confirmation re-checks `users.email` in its transaction).
--
-- Like the other e-mailed links, only the SHA-256 hash of the random token is
-- stored. The change lasts 24 hours (`expires_at`, set by the API from its
-- clock); expired rows are ignored everywhere and deleted when a new request
-- arrives, with no scheduled job (ADR-0002).
--
-- Purely additive: the previous API version never touches this table.
CREATE TABLE pending_email_changes (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL UNIQUE REFERENCES users (id) ON DELETE CASCADE,
    new_email  CITEXT      NOT NULL,
    token_hash TEXT        NOT NULL UNIQUE,
    created_at TIMESTAMPTZ NOT NULL,
    expires_at TIMESTAMPTZ NOT NULL
);
