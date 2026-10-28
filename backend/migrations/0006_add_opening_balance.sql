-- The Opening Balance (UC22): the money the user already had when starting to
-- use the system. It is optional and informed exactly once, so NULL means
-- "not informed yet" (zero and negative values are valid informed values).
-- There is one balance per user, so it lives on the user. Nothing else about
-- the balance is stored: the Current Balance is computed on read (ADR-0002).
ALTER TABLE users ADD COLUMN opening_balance_cents BIGINT;
