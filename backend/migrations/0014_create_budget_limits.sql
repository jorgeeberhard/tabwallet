-- Monthly Budgets (UC12, UC13).
--
-- A Budget is a limit for an expense Category that applies every month until
-- the user changes it. Past months must keep the limit they had, so a limit is
-- stored as a version that starts in a month (like the amounts of a recurring
-- Transaction): the limit of a month is the latest version that started on or
-- before it. Changing a limit adds (or replaces) the version of the current
-- month; removing it adds a version without a limit, which ends the Budget
-- from that month on.
--
-- The Alert is never stored (ADR-0002): the spending of a Category in a
-- Reference Month is derived from the Transactions on every read.
--
-- The Budget of a deactivated Category stays here untouched; whether it
-- produces an Alert is decided on read, from the Category's state.

CREATE TABLE budget_limits (
    user_id         UUID        NOT NULL REFERENCES users (id),
    category_id     UUID        NOT NULL,
    -- Always an expense Category; lets the foreign key below refuse an income
    -- one.
    type            TEXT        NOT NULL DEFAULT 'expense' CHECK (type = 'expense'),
    -- The first day of the month the version starts in.
    effective_month DATE        NOT NULL CHECK (EXTRACT(DAY FROM effective_month) = 1),
    -- NULL: the Budget was removed from this month on.
    limit_cents     BIGINT      CHECK (limit_cents IS NULL OR limit_cents > 0),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (category_id, effective_month),
    FOREIGN KEY (category_id, type) REFERENCES categories (id, type)
);

CREATE INDEX budget_limits_user ON budget_limits (user_id);
