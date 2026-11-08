-- Recurring Transactions: Gastos fixos and Receitas fixas (UC05-UC07).
--
-- Only the definition is stored. The Occurrences (one per month) are computed
-- on every read and never written by a job (ADR-0002): the rows below are the
-- definition, its amount versions and the exceptions the user made.
--
-- Months are stored as the first day of the month.

CREATE TABLE recurring_transactions (
    id           UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id      UUID        NOT NULL REFERENCES users (id),
    type         TEXT        NOT NULL CHECK (type IN ('income', 'expense')),
    description  TEXT        NOT NULL CHECK (char_length(description) BETWEEN 1 AND 200),
    category_id  UUID        NOT NULL,
    -- Shorter months use their last day instead; decided when reading.
    day_of_month SMALLINT    NOT NULL CHECK (day_of_month BETWEEN 1 AND 31),
    -- First month with an Occurrence.
    start_month  DATE        NOT NULL CHECK (EXTRACT(day FROM start_month) = 1),
    -- Last month with an Occurrence, once the user ended it; NULL while it
    -- goes on. Occurrences of earlier months stay as history.
    end_month    DATE        CHECK (EXTRACT(day FROM end_month) = 1),
    created_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at   TIMESTAMPTZ NOT NULL DEFAULT now(),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

CREATE INDEX recurring_transactions_user ON recurring_transactions (user_id);

-- The amount in force from a month on, until the next version. Changing the
-- amount adds (or replaces) the version of the current month, so earlier
-- months keep the amount they had. The first row is the start month.
CREATE TABLE recurring_amounts (
    recurring_transaction_id UUID   NOT NULL REFERENCES recurring_transactions (id) ON DELETE CASCADE,
    effective_month          DATE   NOT NULL CHECK (EXTRACT(day FROM effective_month) = 1),
    amount_cents             BIGINT NOT NULL CHECK (amount_cents > 0),
    PRIMARY KEY (recurring_transaction_id, effective_month)
);

-- What the user changed in one single Occurrence: it was deleted ("skipped")
-- or its amount was edited. Nothing else about an Occurrence is stored.
CREATE TABLE recurring_exceptions (
    recurring_transaction_id UUID    NOT NULL REFERENCES recurring_transactions (id) ON DELETE CASCADE,
    month                    DATE    NOT NULL CHECK (EXTRACT(day FROM month) = 1),
    skipped                  BOOLEAN NOT NULL,
    amount_cents             BIGINT,
    PRIMARY KEY (recurring_transaction_id, month),
    CHECK ((skipped AND amount_cents IS NULL) OR (NOT skipped AND amount_cents > 0))
);
