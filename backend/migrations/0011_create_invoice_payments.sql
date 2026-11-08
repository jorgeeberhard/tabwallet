-- Early Payments of credit-card invoices (UC17, ADR-0003).
--
-- Only what the user does is stored. The payment that settles an invoice on
-- its Due Day is derived on every read (ADR-0002) and has no row. An invoice
-- is not stored either: it is identified by its card and its closing month
-- (the first day of the month in which it closes). A payment is not an
-- expense, so it lives apart from `transactions` and `card_purchases`.

CREATE TABLE invoice_payments (
    id             UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id        UUID        NOT NULL REFERENCES users (id),
    credit_card_id UUID        NOT NULL REFERENCES credit_cards (id),
    closing_month  DATE        NOT NULL CHECK (EXTRACT(DAY FROM closing_month) = 1),
    amount_cents   BIGINT      NOT NULL CHECK (amount_cents > 0),
    -- The day the money left the balance: the day the user paid.
    date           DATE        NOT NULL,
    created_at     TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at     TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX invoice_payments_user_date
    ON invoice_payments (user_id, date DESC, created_at DESC, id DESC);
CREATE INDEX invoice_payments_invoice ON invoice_payments (credit_card_id, closing_month);
