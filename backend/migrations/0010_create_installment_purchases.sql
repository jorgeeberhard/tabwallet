-- Installment purchases on a credit card and their Installments (UC18,
-- ADR-0003).
--
-- Like a single card purchase, an installment purchase never touches the
-- Current Balance, so it lives apart from `transactions`. Each Installment is
-- its own row with its own amount and its own invoice, so that a later
-- operation (Installment Anticipation) can move one Installment to another
-- invoice without touching the others. Invoices themselves are still not
-- stored (ADR-0002): `invoice_month` only says which one the Installment is
-- in, by the first day of its closing month; its dates and state are derived.

CREATE TABLE installment_purchases (
    id                UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id           UUID        NOT NULL REFERENCES users (id),
    credit_card_id    UUID        NOT NULL REFERENCES credit_cards (id),
    -- What the user informed, already with interest if any.
    total_cents       BIGINT      NOT NULL CHECK (total_cents > 0),
    -- How many Installments it was launched in. It stays the same when the
    -- remaining Installments are cancelled.
    installment_count INTEGER     NOT NULL CHECK (installment_count BETWEEN 2 AND 48),
    -- The day of the purchase.
    date              DATE        NOT NULL,
    description       TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 200),
    category_id       UUID        NOT NULL,
    -- Always an expense; lets the foreign key below refuse an income Category.
    type              TEXT        NOT NULL DEFAULT 'expense' CHECK (type = 'expense'),
    -- Set when the remaining Installments were cancelled after some had
    -- been paid; the paid ones stay.
    cancelled_at      TIMESTAMPTZ,
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at        TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

CREATE INDEX installment_purchases_user_date
    ON installment_purchases (user_id, date DESC, created_at DESC, id DESC);
CREATE INDEX installment_purchases_card ON installment_purchases (credit_card_id, date);

CREATE TABLE installments (
    id                      UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id                 UUID    NOT NULL REFERENCES users (id),
    installment_purchase_id UUID    NOT NULL
        REFERENCES installment_purchases (id) ON DELETE CASCADE,
    -- 1 for the first Installment, up to the purchase's installment_count.
    number                  INTEGER NOT NULL CHECK (number >= 1),
    amount_cents            BIGINT  NOT NULL CHECK (amount_cents > 0),
    -- The invoice the Installment is in: the first day of its closing month.
    invoice_month           DATE    NOT NULL CHECK (EXTRACT(DAY FROM invoice_month) = 1),
    UNIQUE (installment_purchase_id, number)
);

CREATE INDEX installments_user_invoice ON installments (user_id, invoice_month);
