-- Income and expense Transactions outside the credit card (UC04, UC06-UC08).
--
-- Money is stored in integer cents and is always positive: `type` gives the
-- direction. `date` is a calendar day with no time. Whether a transaction is a
-- Projection (a date after today) depends on the clock, so it is computed when
-- read and never stored. Credit-card purchases and invoice payments are not
-- Transactions: later tickets keep them in their own tables.

-- Lets a Transaction reference "this category AND its type", so the database
-- itself refuses a category of the other type.
ALTER TABLE categories ADD CONSTRAINT categories_id_type_key UNIQUE (id, type);

CREATE TABLE transactions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID        NOT NULL REFERENCES users (id),
    type         TEXT        NOT NULL CHECK (type IN ('income', 'expense')),
    amount_cents BIGINT      NOT NULL CHECK (amount_cents > 0),
    date         DATE        NOT NULL,
    description  TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 200),
    category_id  UUID        NOT NULL,
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

-- The statement: a user's transactions, newest first.
CREATE INDEX transactions_user_date
    ON transactions (user_id, date DESC, created_at DESC, id DESC);
