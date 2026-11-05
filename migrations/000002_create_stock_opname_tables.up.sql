CREATE TABLE IF NOT EXISTS stock_opnames (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    opname_number VARCHAR(50) UNIQUE NOT NULL,
    opname_date TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    status VARCHAR(30) NOT NULL DEFAULT 'DRAFT',
    total_loss_value NUMERIC(14, 2) NOT NULL DEFAULT 0,
    notes TEXT,
    created_by VARCHAR(100) NOT NULL,
    approved_by VARCHAR(100),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS stock_opname_items (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    opname_id UUID NOT NULL REFERENCES stock_opnames(id) ON DELETE CASCADE,
    ingredient_id UUID NOT NULL REFERENCES ingredients(id),
    system_stock NUMERIC(12, 3) NOT NULL,
    physical_stock NUMERIC(12, 3) NOT NULL,
    difference NUMERIC(12, 3) NOT NULL,
    unit_cost NUMERIC(12, 2) NOT NULL DEFAULT 0,
    loss_value NUMERIC(14, 2) NOT NULL DEFAULT 0,
    notes TEXT,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_stock_opnames_status ON stock_opnames(status);
CREATE INDEX IF NOT EXISTS idx_stock_opnames_created_at ON stock_opnames(created_at);
CREATE INDEX IF NOT EXISTS idx_stock_opname_items_opname_id ON stock_opname_items(opname_id);

