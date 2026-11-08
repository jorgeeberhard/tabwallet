-- Credit cards and the purchases made with them (UC15, UC16, ADR-0003).
--
-- Invoices are not stored: they are derived from the card's closing and due
-- days, its purchases and the clock (ADR-0002). A card purchase is an expense
-- linked to a card; it never touches the Current Balance, so it lives in its
-- own table instead of `transactions`.

CREATE TABLE credit_cards (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id),
    name       TEXT        NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60),
    -- Deactivating is a flag, never a delete: the history stays.
    active     BOOLEAN     NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- A name is unique per user, whatever the case.
CREATE UNIQUE INDEX credit_cards_user_name ON credit_cards (user_id, lower(name));

-- The history of a card's closing and due days. Changing the days only
-- affects invoices still open, so each change is a new row that starts at the
-- first invoice it applies to (`effective_from`, the first day of that
-- invoice's closing month); closed and paid invoices keep their rows.
CREATE TABLE credit_card_schedules (
    credit_card_id UUID     NOT NULL REFERENCES credit_cards (id),
    effective_from DATE     NOT NULL CHECK (EXTRACT(DAY FROM effective_from) = 1),
    closing_day    SMALLINT NOT NULL CHECK (closing_day BETWEEN 1 AND 31),
    due_day        SMALLINT NOT NULL CHECK (due_day BETWEEN 1 AND 31),
    PRIMARY KEY (credit_card_id, effective_from)
);

CREATE TABLE card_purchases (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users (id),
    credit_card_id UUID        NOT NULL REFERENCES credit_cards (id),
    amount_cents   BIGINT      NOT NULL CHECK (amount_cents > 0),
    date           DATE        NOT NULL,
    description    TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 200),
    category_id    UUID        NOT NULL,
    -- Always an expense; lets the foreign key below refuse an income Category.
    type           TEXT        NOT NULL DEFAULT 'expense' CHECK (type = 'expense'),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

CREATE INDEX card_purchases_user_date
    ON card_purchases (user_id, date DESC, created_at DESC, id DESC);
CREATE INDEX card_purchases_card ON card_purchases (credit_card_id, date);
