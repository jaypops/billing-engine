-- Add migration script here
CREATE TABLE plans (
    id                UUID PRIMARY KEY,
    name              TEXT NOT NULL,
    description       TEXT,
    amount_kobo       BIGINT NOT NULL CHECK (amount_kobo > 0),
    currency          TEXT NOT NULL DEFAULT 'NGN' CHECK (currency IN ('NGN')),
    status            TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'archived')),
    billing_interval  TEXT NOT NULL CHECK (billing_interval IN ('monthly', 'yearly')),
    created_at        TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX plans_active_idx ON plans (name) WHERE status = 'active';