-- Charges (UC21) and corrections of the due-date payment (UC20).
--
-- Both are things the user does, so both are stored; everything else about
-- them is derived on every read (ADR-0002). The Carried Balance that a
-- correction leaves on the next invoice has no row: it is the difference
-- between what the invoice came to and what the user said they paid.
-- An invoice is not stored either: it is identified by its card and its
-- closing month (the first day of the month in which it closes).

-- Interest, penalty and IOF the bank charged on an invoice, recorded by the
-- user as an expense of the "Juros e encargos" category. The system never
-- calculates them. A Charge belongs to the invoice the user linked it to (not
-- to the invoice its date would fall in) and is part of that invoice's total.
-- It is an expense but never touches the Current Balance by itself: it leaves
-- through the invoice payment, so it lives apart from `transactions`.
CREATE TABLE invoice_charges (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users (id),
    credit_card_id UUID        NOT NULL REFERENCES credit_cards (id),
    closing_month  DATE        NOT NULL CHECK (EXTRACT(DAY FROM closing_month) = 1),
    amount_cents   BIGINT      NOT NULL CHECK (amount_cents > 0),
    -- The day the bank charged it. It orders the statement; the month it
    -- counts in is the invoice's, not this date's.
    date           DATE        NOT NULL,
    description    TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 200),
    category_id    UUID        NOT NULL,
    -- Always an expense; lets the foreign key below refuse an income Category.
    type           TEXT        NOT NULL DEFAULT 'expense' CHECK (type = 'expense'),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

CREATE INDEX invoice_charges_user_date
    ON invoice_charges (user_id, date DESC, created_at DESC, id DESC);
CREATE INDEX invoice_charges_invoice ON invoice_charges (credit_card_id, closing_month);
-- "Juros e encargos" cannot be deactivated while it has Charges.
CREATE INDEX invoice_charges_category ON invoice_charges (user_id, category_id);

-- The amount the user said they really paid of an invoice at its due date,
-- replacing the automatic payment derived on read. One per invoice: correcting
-- again replaces it. The difference from what the invoice came to moves to the
-- next invoice as its Carried Balance.
CREATE TABLE invoice_payment_corrections (
    credit_card_id UUID        NOT NULL REFERENCES credit_cards (id),
    closing_month  DATE        NOT NULL CHECK (EXTRACT(DAY FROM closing_month) = 1),
    user_id        UUID        NOT NULL REFERENCES users (id),
    -- Zero is valid: the user paid nothing, and all of it moves on.
    amount_cents   BIGINT      NOT NULL CHECK (amount_cents >= 0),
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (credit_card_id, closing_month)
);

CREATE INDEX invoice_payment_corrections_user ON invoice_payment_corrections (user_id);
