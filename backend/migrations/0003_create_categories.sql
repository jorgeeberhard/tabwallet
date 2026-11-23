-- Categories (UC09). Every category belongs to one user and to exactly one
-- type. Categories are deactivated (`active = false`), never deleted, so past
-- transactions keep theirs.
CREATE TABLE categories (
    id         UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id    UUID        NOT NULL REFERENCES users (id),
    type       TEXT        NOT NULL CHECK (type IN ('income', 'expense')),
    -- citext: "lazer" and "Lazer" are the same name.
    name       CITEXT      NOT NULL CHECK (char_length(name) BETWEEN 1 AND 60),
    active     BOOLEAN     NOT NULL DEFAULT true,
    -- The default expense category "Juros e encargos", where Charges are
    -- recorded. Marked by this flag rather than by name, so renaming it does
    -- not lose track of it.
    is_charges BOOLEAN     NOT NULL DEFAULT false,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    -- Unique per type per user among active AND inactive categories.
    UNIQUE (user_id, type, name)
);

CREATE UNIQUE INDEX categories_one_charges_per_user
    ON categories (user_id) WHERE is_charges;

-- The default categories of a new user. Sign-up calls it inside its
-- transaction, and the statement below backfills the users that already
-- exist, so the list lives in one place. Safe to call twice.
CREATE FUNCTION seed_default_categories(target_user_id UUID) RETURNS void
LANGUAGE sql AS $$
    INSERT INTO categories (user_id, type, name, is_charges)
    SELECT target_user_id, defaults.type, defaults.name, defaults.is_charges
    FROM (VALUES
        ('expense', 'Alimentação',      false),
        ('expense', 'Moradia',          false),
        ('expense', 'Transporte',       false),
        ('expense', 'Saúde',            false),
        ('expense', 'Educação',         false),
        ('expense', 'Lazer',            false),
        ('expense', 'Assinaturas',      false),
        ('expense', 'Juros e encargos', true),
        ('expense', 'Outros',           false),
        ('income',  'Salário',          false),
        ('income',  'Outros',           false)
    ) AS defaults (type, name, is_charges)
    ON CONFLICT DO NOTHING
$$;

-- Backfill: users created before categories existed get the defaults too.
SELECT seed_default_categories(id) FROM users;
