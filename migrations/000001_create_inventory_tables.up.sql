CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

CREATE TABLE IF NOT EXISTS ingredients (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    sku VARCHAR(50) UNIQUE NOT NULL,
    name VARCHAR(150) NOT NULL,
    category VARCHAR(50) NOT NULL,
    unit VARCHAR(20) NOT NULL,
    current_stock NUMERIC(12, 3) NOT NULL DEFAULT 0,
    min_stock_alert NUMERIC(12, 3) NOT NULL DEFAULT 10,
    cost_per_unit NUMERIC(12, 2) NOT NULL DEFAULT 0,
    is_active BOOLEAN NOT NULL DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS recipes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    menu_id BIGINT NOT NULL,
    ingredient_id UUID NOT NULL REFERENCES ingredients(id) ON DELETE RESTRICT,
    amount NUMERIC(12, 3) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE(menu_id, ingredient_id)
);

CREATE TABLE IF NOT EXISTS stock_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    ingredient_id UUID NOT NULL REFERENCES ingredients(id),
    change_type VARCHAR(30) NOT NULL,
    quantity NUMERIC(12, 3) NOT NULL,
    balance_after NUMERIC(12, 3) NOT NULL,
    reference_id VARCHAR(100),
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_recipes_menu_id ON recipes(menu_id);
CREATE INDEX IF NOT EXISTS idx_stock_logs_ingredient_id ON stock_logs(ingredient_id);
CREATE INDEX IF NOT EXISTS idx_stock_logs_created_at ON stock_logs(created_at);

